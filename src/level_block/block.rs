use std::mem::{MaybeUninit, size_of};
use std::ptr::NonNull;
use crate::bit_block::BitBlock;
use crate::level_block::{HiBlock, LevelBlock};
use crate::level_block::meta_ptr::Ptr;
use crate::primitive::Primitive;
use crate::primitive_array::PrimitiveArray;

/// TODO: Description
#[derive(Clone)]
pub struct Block<Mask, BlockIndices> {
    mask: Mask,
    /// Next level level_block indices
    block_indices: BlockIndices,
}

impl<Mask, BlockIndices> LevelBlock for Block<Mask, BlockIndices>
where
    Mask: BitBlock,
    BlockIndices: PrimitiveArray
{
    #[inline]
    fn empty() -> Self {
        Self {
            mask: Mask::zero(),
            // All indices 0.
            block_indices: unsafe{MaybeUninit::zeroed().assume_init()}
        }
    }

    fn is_empty(&self) -> bool {
        todo!()
    }

    #[inline]
    fn as_u64_mut(&mut self) -> &mut u64 {
        unsafe{
            self.mask.as_array_mut().as_mut().get_unchecked_mut(0)
        }
    }

    #[inline]
    fn restore_empty_u64(&mut self) {
        *self.as_u64_mut() = 0;
    }
}

impl<Mask, BlockIndices> HiBlock for Block<Mask, BlockIndices>
where
    Mask: BitBlock,
    BlockIndices: PrimitiveArray
{
    type Meta = Ptr<Self>;
    type Mask = Mask;

    #[inline]
    fn mask(&self) -> &Self::Mask {
        &self.mask
    }

    #[inline]
    unsafe fn mask_mut(&mut self) -> &mut Self::Mask {
        &mut self.mask
    }

    type Item = BlockIndices::Item;

    /// Same as `get_unchecked`
    #[inline]
    unsafe fn get_or_zero(&self, index: usize) -> Self::Item {
        let block_indices = self.block_indices.as_ref();
        *block_indices.get_unchecked(index)
    }

    #[inline]
    unsafe fn get_or_insert(&mut self, index: usize, mut f: impl FnMut() -> Self::Item) -> Self::Item {
        // mask
        let exists = self.mask.set_bit::<true>(index);

        // indices
        let block_indices = self.block_indices.as_mut();
        if exists {
            *block_indices.get_unchecked(index)
        } else {
            let block_index = f();
            *block_indices.get_unchecked_mut(index) = block_index;
            block_index
        }
    }

    #[inline]
    unsafe fn remove_unchecked(&mut self, index: usize) {
        // mask
        self.mask.set_bit::<false>(index);
        // If we have block_indices section (compile-time check)
        if !size_of::<BlockIndices>().is_zero(){
            let block_indices = self.block_indices.as_mut();
            *block_indices.get_unchecked_mut(index) = Primitive::ZERO;
        }
    }
}