mod cluster_block;
mod small_block;
mod block;

pub use small_block::*;
pub use cluster_block::*;
pub use block::*;

use crate::{BitBlock, Primitive};

// TODO: move to crate level
pub trait MaybeEmpty {
    fn empty() -> Self;
    fn is_empty(&self) -> bool;
}

/// Implementing this will allow your struct in an empty state 
/// to be used as a LinkedList node with [IntrusiveListLevel]. 
pub trait IntrusiveMaybeEmptyNode: MaybeEmpty {
    fn as_u64_mut(&mut self) -> &mut u64;
    /// Restore [empty()] state, after [as_u64_mut()] mutation.
    fn restore_empty(&mut self);
}

// TODO: rename to LevelBlock
/// Hierarchy level level_block
pub trait HiBlock: /*LevelBlock*/ IntrusiveMaybeEmptyNode {
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
    
    /// Return previous mask bit.
    /// 
    /// # Safety
    ///
    /// * `index` must be set
    /// * `index` is not checked for out-of-bounds.
    unsafe fn remove_unchecked(&mut self, index: usize);
}