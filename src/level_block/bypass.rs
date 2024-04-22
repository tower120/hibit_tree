use std::marker::PhantomData;
use crate::bit_block::{EmptyBitBlock, IEmptyBitBlock, is_empty_bitblock};
use crate::level_block::{HiBlock, LevelBlock};
use crate::level_block::meta_ptr::EmptyPtr;
use crate::BitBlock;

pub struct BypassBlock<Mask/* : IEmptyBitBlock = EmptyBitBlock */>(PhantomData<Mask>);

impl<Mask/* : IEmptyBitBlock */> LevelBlock for BypassBlock<Mask>{
    fn empty() -> Self {
        Self(PhantomData)
    }

    fn is_empty(&self) -> bool {
        unreachable!()
    }

    fn as_u64_mut(&mut self) -> &mut u64 {
        unreachable!()
    }

    fn restore_empty_u64(&mut self) {
        unreachable!()
    }
}

impl<Mask: BitBlock/* : IEmptyBitBlock */> HiBlock for BypassBlock<Mask>{
    type Meta = EmptyPtr<Self>;
    type Mask = Mask;

    fn mask(&self) -> &Self::Mask {
        unreachable!()
    }

    unsafe fn mask_mut(&mut self) -> &mut Self::Mask {
        unreachable!()
    }

    type Item = u8;

    unsafe fn get_or_zero(&self, index: usize) -> Self::Item {
        unreachable!()
    }

    unsafe fn get_or_insert(&mut self, index: usize, f: impl FnMut() -> Self::Item) -> Self::Item {
        unreachable!()
    }

    unsafe fn remove_unchecked(&mut self, index: usize) {
        unreachable!()
    }
}

pub(crate) /*const*/ fn is_bypass_block<T: HiBlock>() -> bool {
    is_empty_bitblock::<T::Mask>()
}