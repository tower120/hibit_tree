use crate::bit_block::BitBlock;
use crate::bit_queue::BitQueue;
use crate::{LevelMasks, IntoOwned};

pub struct SimpleBlockIter<'a, T>
where
    T: LevelMasks,
{
    container: &'a T,
    level0_iter: <T::Level0MaskType as BitBlock>::BitsIter,
    level1_iter: <T::Level1MaskType as BitBlock>::BitsIter,
    level0_index: usize,
}

impl<'a, T> SimpleBlockIter<'a, T>
where
    T: LevelMasks,
{
    #[inline]
    pub fn new(container: &'a T) -> Self {
        let level0_iter = container.level0_mask().into_owned().into_bits_iter();
        Self{
            container,
            level0_iter,
            level1_iter: BitQueue::empty(),
            level0_index: 0,
        }
    }
}

impl<'a, T> Iterator for SimpleBlockIter<'a, T>
where
    T: LevelMasks,
{
    type Item = (usize/*index*/, T::DataBlock<'a>);

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
                        self.container.level1_mask(index)
                    };
                    self.level1_iter = level1_mask.into_owned().into_bits_iter();
                } else {
                    return None;
                }
            }
        };

        let data_block = unsafe {
            self.container.data_block(self.level0_index, level1_index)
        };

        let block_index =
            (self.level0_index << T::Level1MaskType::SIZE_POT_EXPONENT)
            + level1_index;

        Some((block_index, data_block))
    }
}