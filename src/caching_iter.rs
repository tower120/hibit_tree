use std::any::TypeId;
use std::mem::{ManuallyDrop, MaybeUninit};
use crate::level_masks::{LevelMasksIter, LevelMasksIterState};
use crate::{BitBlock, LevelMasks, IntoOwned, data_block_index};
use crate::bit_block::is_bypass_bitblock;
use crate::bit_queue::BitQueue;

pub struct CachingBlockIter<'a, T>
where
    T: LevelMasksIter,
{
    container: &'a T,
    level0_iter: <T::Level0MaskType as BitBlock>::BitsIter,
    level1_iter: <T::Level1MaskType as BitBlock>::BitsIter,
    level0_index: usize,

    state: ManuallyDrop<T::IterState>,
    level1_block_meta: MaybeUninit<T::Level1BlockMeta>,
}

impl<'a, T> CachingBlockIter<'a, T>
where
    T: LevelMasksIter,
{
    #[inline]
    pub fn new(container: &'a T) -> Self {
        let level0_iter = container.level0_mask().into_owned().into_bits_iter(); 
        let state = T::IterState::make(container); 
        Self{
            container,
            
            level0_iter,
            level1_iter: BitQueue::empty(),
            
            // TODO: refactor this
            // usize::MAX - is marker, that we're in "intial state".
            // Which means that only level0_iter initialized, and in original state.
            level0_index: usize::MAX,    

            state: ManuallyDrop::new(state),
            level1_block_meta: MaybeUninit::new(Default::default())
        }
    }
    
    #[inline]
    /*const*/ fn bypass_lvl1() -> bool{
        is_bypass_bitblock::<T::Level1MaskType>()
    }
}

impl<'a, T> Iterator for CachingBlockIter<'a, T>
where
    T: LevelMasksIter,
{
    type Item = (usize/*index*/, T::DataBlock<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let level1_index = loop {
            if let Some(index) = self.level1_iter.next() {
                break index;
            } else {
                //update level0
                if let Some(index) = self.level0_iter.next() {
                    if Self::bypass_lvl1(){
                        break index;
                    }
                    
                    self.level0_index = index;
                    
                    let level1_mask = unsafe {
                        self.level1_block_meta.assume_init_drop();
                        let (level1_mask, _) = 
                            self.container.init_level1_block_meta(
                                &mut self.state,
                                &mut self.level1_block_meta,
                                index
                            );
                        level1_mask
                    };

                    self.level1_iter = level1_mask.into_owned().into_bits_iter();
                } else {
                    return None;
                }
            }
        };
        
        // TODO: Specialization for TRUSTED_HIERARCHY without loop

        let data_block = unsafe {
            T::data_block_from_meta(
                &self.container,
                self.level1_block_meta.assume_init_ref(), level1_index
            )
        };

        let block_index = data_block_index::<T>(self.level0_index, level1_index);

        Some((block_index, data_block))
    }    
}

impl<'a, T> Drop for CachingBlockIter<'a, T>
where
    T: LevelMasksIter
{
    #[inline]
    fn drop(&mut self) {
        unsafe{
            self.level1_block_meta.assume_init_drop();
            
            T::IterState
                ::drop(self.container, &mut self.state);
        }
    }
}