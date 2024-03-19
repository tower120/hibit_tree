use std::marker::PhantomData;
use crate::bit_block::BitBlock;
use crate::{LevelMasks, RefOrVal};

pub trait Op {
    type Level0Mask;
    fn lvl0_op(left: Self::Level0Mask, right: Self::Level0Mask) -> Self::Level0Mask;
    
    type Level1Mask;
    fn lvl1_op(left: Self::Level1Mask, right: Self::Level1Mask) -> Self::Level1Mask;

    type DataBlock;
    /// Operation applied to data block
    fn data_op(left: Self::DataBlock, right: Self::DataBlock) -> Self::DataBlock;
}

pub struct Apply<Op, S1, S2>{
    pub(crate) s1: S1,
    pub(crate) s2: S2,
    pub(crate) phantom: PhantomData<Op>
}

impl<Op, S1, S2> LevelMasks for Apply<Op, S1, S2>
where
    S1: RefOrVal,
    S1::Type: LevelMasks,

    S2: RefOrVal,
    S2::Type: LevelMasks<
        Level0Mask = <S1::Type as LevelMasks>::Level0Mask, 
        Level1Mask = <S1::Type as LevelMasks>::Level1Mask,
        DataBlock  = <S1::Type as LevelMasks>::DataBlock,
    >,

    Op: self::Op<
        Level0Mask = <S1::Type as LevelMasks>::Level0Mask, 
        Level1Mask = <S1::Type as LevelMasks>::Level1Mask,
        DataBlock  = <S1::Type as LevelMasks>::DataBlock,
    >
{
    type Level0Mask = <S1::Type as LevelMasks>::Level0Mask;
    #[inline]
    fn level0_mask(&self) -> Self::Level0Mask {
        let s1 = self.s1.as_ref(); 
        let s2 = self.s2.as_ref();
        Op::lvl0_op(s1.level0_mask(), s2.level0_mask())
    }

    type Level1Mask = <S1::Type as LevelMasks>::Level1Mask;
    #[inline]
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask {
        let s1 = self.s1.as_ref(); 
        let s2 = self.s2.as_ref();
        Op::lvl1_op(
            s1.level1_mask(level0_index),
            s2.level1_mask(level0_index)
        )
    }

    type DataBlock = <S1::Type as LevelMasks>::DataBlock;
    #[inline]
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize) -> Self::DataBlock {
        let s1 = self.s1.as_ref(); 
        let s2 = self.s2.as_ref();
        Op::data_op(
            s1.data_block(level0_index, level1_index),
            s2.data_block(level0_index, level1_index)
        )
    }
}

// TODO: other array read operations