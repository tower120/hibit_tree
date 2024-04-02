use std::marker::PhantomData;
use crate::BitBlock;
use crate::level_block::{HiBlock, LevelBlock};
use crate::level_block::meta_ptr::EmptyPtr;

pub struct BypassBlock<Mask>(PhantomData<Mask>);

impl<Mask> LevelBlock for BypassBlock<Mask>{
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

impl<Mask: BitBlock> HiBlock for BypassBlock<Mask>{
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