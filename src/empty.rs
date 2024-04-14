use std::marker::PhantomData;
use crate::BitBlock;
use crate::level_block::LevelBlock;
use crate::level_masks::{SparseHierarchy};

// TODO: Full
/// Empty array. ZST.
pub struct Empty<Level0Mask, Level1Mask, Level2Mask, Data>(PhantomData<(Level0Mask, Level1Mask, Level2Mask, Data)>);

impl<Level0Mask, Level1Mask, Level2Mask, Data> Default for Empty<Level0Mask, Level1Mask, Level2Mask, Data>{
    #[inline]
    fn default() -> Self { Self(PhantomData) }
}

impl<Level0Mask, Level1Mask, Level2Mask, Data> SparseHierarchy for Empty<Level0Mask, Level1Mask, Level2Mask, Data>
where
    Level0Mask: BitBlock,
    Level1Mask: BitBlock,
    Level2Mask: BitBlock,
    Data: LevelBlock
{
    const EXACT_HIERARCHY: bool = true;
    
    type Level0MaskType = Level0Mask;
    type Level0Mask<'a> where Self: 'a = Level0Mask;

    #[inline]
    fn level0_mask(&self) -> Self::Level0Mask<'_> {
        Level0Mask::zero()
    }

    type Level1MaskType = Level1Mask;
    type Level1Mask<'a> where Self: 'a = Level1Mask;

    #[inline]
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask<'_> {
        Level1Mask::zero()
    }

    type Level2MaskType = Level2Mask;
    type Level2Mask<'a> where Self: 'a = Level2Mask;

    #[inline]
    unsafe fn level2_mask(&self, level0_index: usize, level1_index: usize) -> Self::Level2Mask<'_> {
        Level2Mask::zero()
    }

    type DataBlockType = Data;
    type DataBlock<'a> where Self: 'a = Data;

    #[inline]
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize, level2_index: usize) -> Self::DataBlock<'_> {
        Data::empty()
    }
}

impl<Level0Mask, Level1Mask, Level2Mask, Data> LevelMasksIter for Empty<Level0Mask, Level1Mask, Level2Mask, Data>
where
    Level0Mask: BitBlock,
    Level1Mask: BitBlock,
    Level2Mask: BitBlock,
    Data: LevelBlock
{
    type IterState = ();

    #[inline]
    fn make_state(&self) -> Self::IterState { () }

    #[inline]
    unsafe fn init_level1_block_meta(&self, state: &mut Self::IterState, level0_index: usize) -> (Self::Level1Mask<'_>, bool) {
        (Level1Mask::zero(), false)
    }

    #[inline]
    unsafe fn init_level2_block_meta(&self, state: &mut Self::IterState, level1_index: usize) -> (Self::Level2Mask<'_>, bool) {
        (Level2Mask::zero(), false)
    }

    #[inline]
    unsafe fn data_block_from_meta(&self, state: &Self::IterState, level_index: usize) -> Self::DataBlock<'_> {
        Data::empty()
    }
}