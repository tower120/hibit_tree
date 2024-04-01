use std::any::TypeId;
use crate::block::{HiBlock, LevelBlock};

pub struct BypassBlock;

impl LevelBlock for BypassBlock{
    fn empty() -> Self {
        Self
    }

    fn is_empty(&self) -> bool {
        todo!()
    }

    fn as_u64_mut(&mut self) -> &mut u64 {
        todo!()
    }

    fn restore_empty_u64(&mut self) {
        todo!()
    }
}

impl HiBlock for BypassBlock{
    // TODO: BypassMask
    type Mask = ();

    fn mask(&self) -> &Self::Mask {
        todo!()
    }

    unsafe fn mask_mut(&mut self) -> &mut Self::Mask {
        todo!()
    }

    type Item = u8;

    unsafe fn get_or_zero(&self, index: usize) -> Self::Item {
        todo!()
    }

    unsafe fn get_or_insert(&mut self, index: usize, f: impl FnMut() -> Self::Item) -> Self::Item {
        todo!()
    }

    unsafe fn remove_unchecked(&mut self, index: usize) {
        todo!()
    }
}

pub(crate) fn is_bypass_block<T: HiBlock>() -> bool {
    TypeId::of::<T::Mask>() == TypeId::of::<()>()
} 