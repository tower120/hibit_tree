mod cluster_block;
mod small_block;
mod block;

pub use small_block::*;
pub use cluster_block::*;
pub use block::*;

use crate::{BitBlock, MaybeEmpty, MaybeEmptyIntrusive, Primitive};

// TODO: rename to LevelBlock
/// Hierarchy level level_block
pub trait HiBlock: MaybeEmptyIntrusive {
    type Mask: BitBlock;
    
    fn mask(&self) -> &Self::Mask;
    
    // TODO: this is probably not needed
    unsafe fn mask_mut(&mut self) -> &mut Self::Mask;
    
    // TODO: BlockIndex, IndexPointer ?
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
    
    /// # Safety
    ///
    /// * `index` must be set
    /// * `index` is not checked for out-of-bounds.
    unsafe fn remove_unchecked(&mut self, index: usize);

    /// # Safety
    ///
    /// * `index` must be set
    /// * `index` is not checked for out-of-bounds.
    /// * `item` emptiness must correspond to mask's `index` bit. 
    unsafe fn set_unchecked(&mut self, index: usize, item: Self::Item);
}