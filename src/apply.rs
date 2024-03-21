use std::marker::PhantomData;
use std::mem;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::addr_of_mut;
use crate::bit_block::BitBlock;
use crate::{LevelMasks, ref_or_val, RefOrVal};
use crate::level_masks::{LevelMasksIter, LevelMasksIterState, NoState};

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

pub struct ApplyIterState<Op, S1, S2>
where
    S1: RefOrVal,
    S1::Type: LevelMasksIter,

    S2: RefOrVal,
    S2::Type: LevelMasksIter
{
    s1: <S1::Type as LevelMasksIter>::IterState, 
    s2: <S2::Type as LevelMasksIter>::IterState,
    phantom: PhantomData<Op>
}

impl<Op, S1, S2> LevelMasksIterState for ApplyIterState<Op, S1, S2>
where
    S1: RefOrVal,
    S1::Type: LevelMasksIter,

    S2: RefOrVal,
    S2::Type: LevelMasksIter/*<
        Level0Mask = <S1::Type as LevelMasks>::Level0Mask, 
        Level1Mask = <S1::Type as LevelMasks>::Level1Mask,
        DataBlock  = <S1::Type as LevelMasks>::DataBlock,
    >*/,

    /*Op: self::Op<
        Level0Mask = <S1::Type as LevelMasks>::Level0Mask, 
        Level1Mask = <S1::Type as LevelMasks>::Level1Mask,
        DataBlock  = <S1::Type as LevelMasks>::DataBlock,
    >*/
{
    type Container = Apply<Op, S1, S2>;

    #[inline]
    fn make(container: &Self::Container) -> Self {
        Self{
            s1: <S1::Type as LevelMasksIter>::IterState::make(container.s1.as_ref()),
            s2: <S2::Type as LevelMasksIter>::IterState::make(container.s2.as_ref()),
            phantom: PhantomData
        }
    }

    #[inline]
    fn drop(container: &Self::Container, this: &mut ManuallyDrop<Self>) {
        unsafe{
            <S1::Type as LevelMasksIter>::IterState::drop(container.s1.as_ref(), mem::transmute(&mut this.s1));
            <S2::Type as LevelMasksIter>::IterState::drop(container.s2.as_ref(), mem::transmute(&mut this.s2));
        }
    }
}

impl<Op, S1, S2> LevelMasksIter for Apply<Op, S1, S2>
where
    S1: RefOrVal,
    S1::Type: LevelMasksIter,

    S2: RefOrVal,
    S2::Type: LevelMasksIter<
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
    type IterState = ApplyIterState<Op, S1, S2>;
    type Level1BlockInfo = (
        <S1::Type as LevelMasksIter>::Level1BlockInfo, 
        <S2::Type as LevelMasksIter>::Level1BlockInfo
    );

    #[inline]
    unsafe fn init_level1_block_info(
        &self, 
        state: &mut Self::IterState, 
        level1_block_data: &mut MaybeUninit<Self::Level1BlockInfo>, 
        level0_index: usize
    ) -> (Self::Level1Mask, bool) {
        // &mut MaybeUninit<(T0, T1)> = (&mut MaybeUninit<T0>, &mut MaybeUninit<T1>) 
        let (level1_block_data0, level1_block_data1) = {
            let ptr = level1_block_data.as_mut_ptr();
            let ptr0 = addr_of_mut!((*ptr).0);
            let ptr1 = addr_of_mut!((*ptr).1);
            (
                &mut*mem::transmute::<_, *mut MaybeUninit<<S1::Type as LevelMasksIter>::Level1BlockInfo>>(ptr0), 
                &mut*mem::transmute::<_, *mut MaybeUninit<<S2::Type as LevelMasksIter>::Level1BlockInfo>>(ptr1)
            )
        };   
        
        let (mask1, v1) = self.s1.as_ref().init_level1_block_info(
            &mut state.s1, level1_block_data0, level0_index
        );
        let (mask2, v2) = self.s2.as_ref().init_level1_block_info(
            &mut state.s2, level1_block_data1, level0_index
        );
        
        let mask = Op::lvl1_op(mask1, mask2);
        (mask, v1 | v2)
    }

    #[inline]
    unsafe fn data_block_from_info(
        level1_block_info: &Self::Level1BlockInfo, 
        level1_index: usize
    ) -> Self::DataBlock {
        let m0 = <S1::Type as LevelMasksIter>::data_block_from_info(
            &level1_block_info.0, level1_index
        );
        let m1 = <S2::Type as LevelMasksIter>::data_block_from_info(
            &level1_block_info.1, level1_index
        ); 
        Op::data_op(m0, m1)
    }
}

ref_or_val!(impl<Op, S1, S2> for Apply<Op, S1, S2>);

// TODO: other array read operations