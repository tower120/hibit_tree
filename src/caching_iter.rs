use std::mem::{ManuallyDrop, MaybeUninit};
use crate::level_masks::{LevelMasksIter, LevelMasksIterState};
use crate::{BitBlock, LevelMasks, RefOrVal};
use crate::bit_queue::BitQueue;

pub struct CachingBlockIter<T>
where
    T: RefOrVal,
    T::Type: LevelMasksIter,
{
    container: T,
    level0_iter: <<T::Type as LevelMasks>::Level0Mask as BitBlock>::BitsIter,
    level1_iter: <<T::Type as LevelMasks>::Level1Mask as BitBlock>::BitsIter,
    level0_index: usize,

    state: ManuallyDrop<<T::Type as LevelMasksIter>::IterState>,
    level1_block_data: MaybeUninit<<T::Type as LevelMasksIter>::Level1BlockInfo>,
}

impl<T> CachingBlockIter<T>
where
    T: RefOrVal,
    T::Type: LevelMasksIter,
{
    #[inline]
    pub fn new(container: T) -> Self {
        let container_ref = container.as_ref();
        let level0_iter = container_ref.level0_mask().into_bits_iter(); 
        let state = <T::Type as LevelMasksIter>::IterState::make(container_ref); 
        Self{
            container,
            
            level0_iter,
            level1_iter: BitQueue::empty(),
            
            // TODO: refactor this
            // usize::MAX - is marker, that we're in "intial state".
            // Which means that only level0_iter initialized, and in original state.
            level0_index: usize::MAX,    

            state: ManuallyDrop::new(state),
            level1_block_data: MaybeUninit::new(Default::default())
        }
    }
}

impl<T> Iterator for CachingBlockIter<T>
where
    T: RefOrVal,
    T::Type: LevelMasksIter,
{
    type Item = (usize/*index*/, <T::Type as LevelMasks>::DataBlock);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let level1_index = loop {
            if let Some(index) = self.level1_iter.next() {
                break index;
            } else {
                //update level0
                if let Some(index) = self.level0_iter.next() {
                    self.level0_index = index;
                    
                    let level1_mask = unsafe {
                        self.level1_block_data.assume_init_drop();
                        let (level1_mask, _) = 
                            self.container.as_ref().init_level1_block_info(
                                &mut self.state,
                                &mut self.level1_block_data,
                                index
                            );
                        level1_mask
                    };

                    self.level1_iter = level1_mask.into_bits_iter();
                } else {
                    return None;
                }
            }
        };

        let data_block = unsafe {
            <T::Type as LevelMasksIter>::data_block_from_info(
                self.level1_block_data.assume_init_ref(), level1_index
            )
        };

        let block_index =
            self.level0_index << <T::Type as LevelMasks>::Level1Mask::SIZE_POT_EXPONENT
            + level1_index;

        Some((block_index, data_block))
    }    
}

impl<T> Drop for CachingBlockIter<T>
where
    T: RefOrVal,
    T::Type: LevelMasksIter,
{
    #[inline]
    fn drop(&mut self) {
        unsafe{
            self.level1_block_data.assume_init_drop();
            
            <T::Type as LevelMasksIter>::IterState
                ::drop(self.container.as_ref(), &mut self.state);
        }
    }
}