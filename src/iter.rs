use std::ops::ControlFlow;
use crate::bitmap_tree::{BitmapTree, BitmapTreeCursor};
use crate::{BitBlock, data_block_index, RegularBitmapTree, BitmapTreeCursorTypes, BitmapTreeTypes};
use crate::bit_queue::BitQueue;
use crate::const_utils::const_int::{const_for_rev, ConstInteger, ConstIntVisitor, ConstUsize};
use crate::const_utils::const_array::ConstArrayType;
use crate::utils::LendingIterator;
use crate::utils::Array;

// TODO: could be u8's
/// [usize; T::LevelCount::N - 1]
type LevelIndices<T: BitmapTree> =
    ConstArrayType<
        usize,
        <T::LevelCount as ConstInteger>::Dec   
    >;

/// Each hierarchy level has its own iterator.
/// 
/// [T::LevelMaskType::BitsIter; T::LevelCount]
type LevelIterators<T: BitmapTree> =
    ConstArrayType<
        <<T as BitmapTree>::LevelMask as BitBlock>::BitsIter,
        T::LevelCount
    >;

/// [BitmapTree] iterator.
///  
/// This is [LendingIterator], that also [Iterator] for [RegularBitmapTree]. 
pub struct Iter<'a, T>
where
    T: BitmapTree,
{
    container: &'a T,
    
    /// [T::LevelMaskType::BitsIter; T::LevelCount]
    level_iters: LevelIterators<T>,
    
    /// [usize; T::LevelCount - 1]
    level_indices: LevelIndices<T>,

    cursor: <T as BitmapTreeTypes<'a>>::Cursor,
}

impl<'a, T> Iter<'a, T>
where
    T: BitmapTree,
{
    #[inline]
    pub fn new(container: &'a T) -> Self {
        let mut level_iters: LevelIterators<T> = Array::from_fn(|_| BitQueue::empty());
        
        let mut cursor = T::Cursor::new(container);
        
        let root_mask = unsafe{
            cursor.select_level_node_unchecked(container, ConstUsize::<0>, 0)
        };
        let level0_iter = root_mask.into_bits_iter();
        
        level_iters.as_mut()[0] = level0_iter; 
        
        Self{
            container,
            level_iters,
            
            // TODO: refactor this
            // usize::MAX - is marker, that we're in "intial state".
            // Which means that only level0_iter initialized, and in original state.
            level_indices: Array::from_fn(|_| usize::MAX),

            cursor,
        }
    }
}

impl<'a, T> LendingIterator for Iter<'a, T>
where
    T: BitmapTree,
{
    type Item<'this>= (
        usize/*index*/, 
        <<T as BitmapTreeTypes<'a>>::Cursor as BitmapTreeCursorTypes<'this>>::Data
    ) where Self:'this;

    #[inline]
    fn next(&mut self) -> Option<Self::Item<'_>> {
        let level_index = loop {
            // We're driven by top-level iterator.
            let last_level_iter = self.level_iters.as_mut().last_mut().unwrap();
            if let Some(index) = last_level_iter.next() {
                break index;
            } else {
                let ctrl = const_for_rev(ConstUsize::<0>, T::LevelCount::DEFAULT.dec(), V(self)); 
                struct V<'b,'a,T: BitmapTree>(&'b mut Iter<'a, T>); 
                impl<'b,'a,T: BitmapTree> ConstIntVisitor for V<'b,'a,T> {
                    type Out = ();
                    #[inline(always)]
                    fn visit<I: ConstInteger>(&mut self, i: I) -> ControlFlow<()> {
                        let level_iter = unsafe{
                            self.0
                            .level_iters.as_mut()
                            .get_unchecked_mut(i.value())
                        };
                        if let Some(index) = level_iter.next(){
                            // 1. update level_index
                            unsafe{
                                *self.0
                                    .level_indices.as_mut()
                                    .get_unchecked_mut(i.value()) 
                                    = index; 
                            }
                            
                            // 2. update level_iter from mask
                            let level_depth = i.inc();                            
                            let level_mask = unsafe{
                                self.0.cursor.select_level_node_unchecked(
                                    &self.0.container,
                                    level_depth,
                                    index
                                )
                            };
                            *unsafe{
                                self.0
                                .level_iters.as_mut()
                                .get_unchecked_mut(level_depth.value())
                            } = level_mask.into_bits_iter(); 
                            
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    }
                }   
                if ctrl.is_continue(){
                    // We traversed through whole hierarchy and 
                    // root iter have nothing more. 
                    return None;
                }
            }
        };

        let data_block = unsafe {
            self.cursor.data_unchecked(&self.container, level_index)
        };
        let block_index = data_block_index::<T::LevelCount, T::LevelMask>(&self.level_indices, level_index);
        Some((block_index, data_block))
    }    
}


impl<'a, T> Iterator for Iter<'a, T>
where
    T: RegularBitmapTree,
{
    type Item = (usize, <T as BitmapTreeTypes<'a>>::Data);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        LendingIterator::next(self)
    }
}