use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::ControlFlow;
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::{BitBlock, data_block_index, IntoOwned};
use crate::bit_queue::BitQueue;
use crate::const_int::{const_for_rev, ConstInt, ConstInteger, ConstIntVisitor};
use crate::primitive_array::{Array, ConstArray, ConstArrayType};

// TODO: could be u32's
/// [usize; T::LevelCount::N - 1]
type LevelIndices<T: SparseHierarchy> =
    ConstArrayType<
        usize,
        <T::LevelCount as ConstInteger>::Dec   
    >;

/// Each hierarchy level has its own iterator.
/// 
/// [T::LevelMaskType::BitsIter; T::LevelCount]
type LevelIterators<T: SparseHierarchy> =
    ConstArrayType<
        <T::LevelMaskType as BitBlock>::BitsIter,
        T::LevelCount
    >;

pub struct CachingBlockIter<'a, T>
where
    T: SparseHierarchy,
{
    container: &'a T,
    
    /// [T::LevelMaskType::BitsIter; T::LevelCount]
    level_iters: LevelIterators<T>,
    
    /// [usize; T::LevelCount::N - 1]
    level_indices: LevelIndices<T>,

    state: T::State,
}

impl<'a, T> CachingBlockIter<'a, T>
where
    T: SparseHierarchy,
{
    #[inline]
    pub fn new(container: &'a T) -> Self {
        let mut level_iters: LevelIterators<T> = Array::from_fn(|_| BitQueue::empty());
        
        let mut state = T::State::new(container);
        
        // TODO: This probably could be better
        let (root_mask, _) = unsafe{
            state.select_level_bock(container, ConstInt::<0>, 0)
        };
        let level0_iter = root_mask.into_owned().into_bits_iter();
        
        level_iters.as_mut()[0] = level0_iter; 
        
        Self{
            container,
            level_iters,
            
            // TODO: refactor this
            // usize::MAX - is marker, that we're in "intial state".
            // Which means that only level0_iter initialized, and in original state.
            level_indices: Array::from_fn(|_| usize::MAX),

            state,
        }
    }
}

impl<'a, T> Iterator for CachingBlockIter<'a, T>
where
    T: SparseHierarchy,
{
    type Item = (usize/*index*/, T::DataBlock<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let level_index = loop {
            // We're driven by top-level iterator.
            let top_level_iter = self.level_iters.as_mut().last_mut().unwrap();
            if let Some(index) = top_level_iter.next() {
                break index;
            } else {
                let ctrl = const_for_rev(ConstInt::<0>, T::LevelCount::DEFAULT.dec(), V(self)); 
                struct V<'b,'a,T: SparseHierarchy>(&'b mut CachingBlockIter<'a, T>); 
                impl<'b,'a,T: SparseHierarchy> ConstIntVisitor for V<'b,'a,T> {
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
                            let (level_mask, _) = unsafe{
                                self.0.state.select_level_bock(
                                    &self.0.container,
                                    level_depth,
                                    index
                                )
                            };
                            self.0
                                .level_iters.as_mut()
                                [level_depth.value()]
                            = level_mask.into_owned().into_bits_iter(); 
                            
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
            self.state.data_block(&self.container, level_index)
        };
        let block_index = data_block_index::<T>(&self.level_indices, level_index);
        Some((block_index, data_block))
    }    
}