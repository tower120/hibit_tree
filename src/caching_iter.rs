use std::mem::{ManuallyDrop, MaybeUninit};
use crate::level_masks::{LevelMasksIter, LevelMasks/*, LevelMasksIterState*/, level_bypass, LevelBypass};
use crate::{BitBlock, data_block_index, IntoOwned};
use crate::bit_queue::BitQueue;

pub struct CachingBlockIter<'a, T>
where
    T: LevelMasksIter,
{
    container: &'a T,
    level0_iter: <T::Level0MaskType as BitBlock>::BitsIter,
    level1_iter: <T::Level1MaskType as BitBlock>::BitsIter,
    level2_iter: <T::Level2MaskType as BitBlock>::BitsIter,
    
    // TODO: could be u32's
    level0_index: usize,
    level1_index: usize,

    state: T::IterState,
}

impl<'a, T> CachingBlockIter<'a, T>
where
    T: LevelMasksIter,
{
    #[inline]
    pub fn new(container: &'a T) -> Self {
        let level0_iter = container.level0_mask().into_owned().into_bits_iter(); 
        Self{
            container,
            
            level0_iter,
            level1_iter: BitQueue::empty(),
            level2_iter: BitQueue::empty(),
            
            // TODO: refactor this
            // usize::MAX - is marker, that we're in "intial state".
            // Which means that only level0_iter initialized, and in original state.
            level0_index: usize::MAX,
            level1_index: usize::MAX,    

            state: container.make_state(),
        }
    }
}

impl<'a, T> Iterator for CachingBlockIter<'a, T>
where
    T: LevelMasksIter,
{
    type Item = (usize/*index*/, T::DataBlock<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let level_index = loop {
            // update level2
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
                        let (level_mask, _) = 
                            self.container.init_level2_block_meta(
                                &mut self.state, index
                            );
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
                            let (level1_mask, _) = 
                                self.container.init_level1_block_meta(
                                    &mut self.state,
                                    index
                                );
                            level1_mask
                        };
                        self.level1_iter = level1_mask.into_owned().into_bits_iter();
                    } else {
                        return None;
                    }
                }
            }
        };
        
        // TODO: Specialization for TRUSTED_HIERARCHY without loop

        let data_block = unsafe {
            T::data_block_from_meta(&self.container, &self.state, level_index)
        };
        let block_index = data_block_index::<T>(self.level0_index, self.level1_index, level_index);
        Some((block_index, data_block))
    }    
}