use crate::bit_block::BitBlock;
use crate::bit_queue::BitQueue;
use crate::{LevelMasks, RefOrVal};

pub struct SimpleBlockIter<T>
where
    T: RefOrVal,
    T::Type: LevelMasks,
{
    container: T,
    level0_iter: <<T::Type as LevelMasks>::Level0Mask as BitBlock>::BitsIter,
    level1_iter: <<T::Type as LevelMasks>::Level1Mask as BitBlock>::BitsIter,
    level0_index: usize,
}

impl<T> SimpleBlockIter<T>
where
    T: RefOrVal,
    T::Type: LevelMasks,
{
    #[inline]
    pub fn new(container: T) -> Self {
        let level0_iter = container.as_ref().level0_mask().into_bits_iter();
        Self{
            container,
            level0_iter,
            level1_iter: BitQueue::empty(),
            level0_index: 0
        }
    }
}

impl<T> Iterator for SimpleBlockIter<T>
where
    T: RefOrVal,
    T::Type: LevelMasks,
{
    type Item = (usize/*index*/, <T::Type as LevelMasks>::DataBlock);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let level1_index = loop{
            if let Some(index) = self.level1_iter.next(){
                break index;
            } else {
                //update level0
                if let Some(index) = self.level0_iter.next(){
                    self.level0_index = index;

                    // update level1 iter
                    let level1_mask = unsafe {
                        self.container.as_ref().level1_mask(index)
                    };
                    self.level1_iter = level1_mask.into_bits_iter();
                } else {
                    return None;
                }
            }
        };

        let data_block = unsafe {
            self.container.as_ref().data_block(self.level0_index, level1_index)
        };

        let block_index =
            self.level0_index << <T::Type as LevelMasks>::Level1Mask::SIZE_POT_EXPONENT
            + level1_index;

        Some((block_index, data_block))
    }
}