use std::mem::{MaybeUninit, size_of};
use crate::bit_block::BitBlock;
use crate::primitive::Primitive;
use crate::primitive_array::PrimitiveArray;

pub trait Block: Sized {
    fn empty() -> Self; 
    fn is_empty(&self) -> bool;
    
/*    // Does this needed?
    type Data;
    fn data(&self) -> &Self::Data; 
    unsafe fn data_mut(&mut self) -> &mut Self::Data;*/
    
    fn as_u64_mut(&mut self) -> &mut u64;
    
    /// Restore empty state, after as_u64_mut() mutation.
    fn restore_empty_u64(&mut self);
}

/// Hierarchy block
pub trait HiBlock: Block {
    type Mask: BitBlock;
    
    fn mask(&self) -> &Self::Mask;
    
    // TODO: this is probably not needed
    unsafe fn mask_mut(&mut self) -> &mut Self::Mask;
    
    // TODO: BlockIndex
    type Item: Primitive;
    
    /*/// # Safety
    ///
    /// - index is not checked for out-of-bounds.
    /// - index is not checked for validity (must exist).
    unsafe fn get_unchecked(&self, index: usize) -> Self::Item;*/
    
    /// Returns 0 if item does not exist at `index`.
    /// 
    /// # Safety
    /// 
    /// index is not checked for out-of-bounds.
    unsafe fn get_or_zero(&self, index: usize) -> Self::Item;
    
    /// # Safety
    ///
    /// `index` is not checked.
    unsafe fn get_or_insert(
        &mut self,
        index: usize,
        f: impl FnMut() -> Self::Item
    ) -> Self::Item;
    
    /// Return previous mask bit.
    /// 
    /// # Safety
    ///
    /// * `index` must be set
    /// * `index` is not checked for out-of-bounds.
    unsafe fn remove_unchecked(&mut self, index: usize);
    
/*    #[inline]
    fn is_empty(&self) -> bool {
        todo!()
        //Self::Mask::is_zero(self.mask())
    }*/
}

#[derive(Clone)]
pub struct FixedHiBlock<Mask, BlockIndices> {
    mask: Mask,
    /// Next level block indices
    block_indices: BlockIndices,
}

impl<Mask, BlockIndices> Block for FixedHiBlock<Mask, BlockIndices>
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

impl<Mask, BlockIndices> HiBlock for FixedHiBlock<Mask, BlockIndices>
where
    Mask: BitBlock,
    BlockIndices: PrimitiveArray
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