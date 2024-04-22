use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::ControlFlow;
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::{BitBlock, IntoOwned};
use crate::bit_queue::BitQueue;
use crate::const_int::{const_for_rev, ConstInt, ConstInteger, ConstIntVisitor};
use crate::primitive_array::Array;

// TODO: could be u32's
/// [usize; T::LevelCount::N - 1]
type LevelIndices<T: SparseHierarchy> = 
    <<T::LevelCount as ConstInteger>::Prev as ConstInteger>
    ::Array<usize>;

/// Each hierarchy level has its own iterator.
/// 
/// [T::LevelMaskType::BitsIter; T::LevelCount]
type LevelIterators<T: SparseHierarchy> = 
    <T::LevelCount as ConstInteger>::Array<
        <T::LevelMaskType as BitBlock>::BitsIter       
    >;

pub struct CachingBlockIter<'a, T>
where
    T: SparseHierarchy,
{
    container: &'a T,
    
    /// [T::LevelMaskType::BitsIter; T::LevelCount]
    level_iters: LevelIterators<T>,
    
    /*level0_iter: <T::Level0MaskType as BitBlock>::BitsIter,
    level1_iter: <T::Level1MaskType as BitBlock>::BitsIter,
    level2_iter: <T::Level2MaskType as BitBlock>::BitsIter,*/
    
    /// [usize; T::LevelCount::N - 1]
    level_indices: LevelIndices<T>,
    /*level0_index: usize,
    level1_index: usize,*/

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
            state.select_level_bock(ConstInt::<0>, container, 0)
        };
        let level0_iter = root_mask.into_owned().into_bits_iter();
        
        level_iters.as_mut()[0] = level0_iter; 
        
        Self{
            container,
            level_iters,
            
            /*level0_iter,
            level1_iter: BitQueue::empty(),
            level2_iter: BitQueue::empty(),*/
            
            // TODO: refactor this
            // usize::MAX - is marker, that we're in "intial state".
            // Which means that only level0_iter initialized, and in original state.
            level_indices: Array::from_fn(|_| usize::MAX),
            /*level0_index: usize::MAX,
            level1_index: usize::MAX,*/    

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
                let ctrl = const_for_rev(ConstInt::<0>, T::LevelCount::DEFAULT.prev(), V(self)); 
                struct V<'b,'a,T: SparseHierarchy>(&'b mut CachingBlockIter<'a, T>); 
                impl<'b,'a,T: SparseHierarchy> ConstIntVisitor for V<'b,'a,T> {
                    fn visit<I: ConstInteger>(&mut self, i: I) -> ControlFlow<()> {
                        let level_iter = unsafe{
                            self.0
                            .level_iters.as_mut()
                            .get_unchecked_mut(i.value())
                        };
                        if let Some(index) = level_iter.next(){
                            /*// 1. update level_index
                            unsafe{
                                *self.0
                                    .level_indices.as_mut()
                                    .get_unchecked_mut(i.value()) 
                                    = index; 
                            }*/
                            
                            // 2. update level_iter from mask
                            let level_depth = i.next();                            
                            let (level_mask, _) = unsafe{
                                self.0.state.select_level_bock(
                                    level_depth,
                                    &self.0.container, 
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
            
/*            // update level2
            if let Some(index) = self.level2_iter.next() {
                break index;
            } else {
                // update level1
                if let Some(index) = self.level1_iter.next() {
                    if let LevelBypass::Level2 = level_bypass::<T>() {
                        break index;
                    }
                    
                    self.level1_index = index;
                    let level2_mask = unsafe {
                        let (level_mask, _) = self.state.select_level2(&self.container, index);
                        level_mask
                    };
                    self.level2_iter = level2_mask.into_owned().into_bits_iter();                    
                } else {
                    //update level0
                    if let Some(index) = self.level0_iter.next() {
                        if let LevelBypass::Level1Level2 = level_bypass::<T>(){
                            break index;
                        }
                        
                        self.level0_index = index;
                        let level1_mask = unsafe {
                            let (level1_mask, _) = self.state.select_level1(&self.container, index);
                            level1_mask
                        };
                        self.level1_iter = level1_mask.into_owned().into_bits_iter();
                    } else {
                        return None;
                    }
                }
            }*/
        };
        
        // TODO: Specialization for TRUSTED_HIERARCHY without loop

        let data_block = unsafe {
            self.state.data_block(&self.container, level_index)
        };
        
        // TODO
        //let block_index = data_block_index::<T>(self.level0_index, self.level1_index, level_index);
        let block_index = 0;
        
        Some((block_index, data_block))
    }    
}