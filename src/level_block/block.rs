use std::mem::{MaybeUninit, size_of};
use crate::bit_block::BitBlock;
use crate::level_block::HiBlock;
use crate::{Empty, MaybeEmptyIntrusive};
use crate::utils::Array;
use crate::utils::primitive::Primitive;

/// TODO: Description
#[derive(Clone)]
pub struct Block<Mask, BlockIndices> {
    mask: Mask,
    /// Next level level_block indices
    block_indices: BlockIndices,
}

impl<Mask, BlockIndices> Empty for Block<Mask, BlockIndices>
where
    Mask: BitBlock,
    BlockIndices: Array
{
    #[inline]
    fn empty() -> Self {
        Self {
            mask: Mask::zero(),
            // All indices 0.
            block_indices: unsafe{MaybeUninit::zeroed().assume_init()}
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.mask.is_zero()
    }
}

impl<Mask, BlockIndices> MaybeEmptyIntrusive for Block<Mask, BlockIndices>
where
    Mask: BitBlock,
    BlockIndices: Array
{
    #[inline]
    fn as_u64_mut(&mut self) -> &mut u64 {
        unsafe{
            self.mask.as_array_mut().as_mut().get_unchecked_mut(0)
        }
    }

    #[inline]
    fn restore_empty(&mut self) {
        *self.as_u64_mut() = 0;
    }
}

impl<Mask, BlockIndices> HiBlock for Block<Mask, BlockIndices>
where
    Mask: BitBlock,
    BlockIndices: Array<Item: Primitive>
{
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
    unsafe fn get_or_insert(&mut self, index: usize, mut f: impl FnOnce() -> Self::Item) 
        -> (Self::Item, bool) 
    {
        let block_index_mut = self.block_indices.as_mut().get_unchecked_mut(index);
        let block_index = *block_index_mut; 
        if !block_index.is_zero(){
            (block_index, false)
        } else {
            self.mask.set_bit::<true>(index);
            
            let block_index = f();
            *block_index_mut = block_index; 
            (block_index, true)
        }
    }

    #[inline]
    unsafe fn insert(
        &mut self,
        index: usize,
        item: Self::Item
    ) {
        self.mask.set_bit::<true>(index);
        *self.block_indices.as_mut().get_unchecked_mut(index) = item; 
    }

    #[inline]
    unsafe fn remove_unchecked(&mut self, index: usize) {
        self.mask.set_bit::<false>(index);
        *self.block_indices.as_mut().get_unchecked_mut(index) = Primitive::ZERO;
    }

    #[inline]
    unsafe fn set_unchecked(&mut self, index: usize, item: Self::Item) {
        let block_indices = self.block_indices.as_mut();
        *block_indices.get_unchecked_mut(index) = item;
    }
}