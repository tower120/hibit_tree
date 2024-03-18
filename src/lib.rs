mod primitive;
mod primitive_array;
mod block;
mod array;
mod level;
//mod simple_iter;

pub use primitive::Primitive;
pub use block::{Block, HiBlock};
pub use array::SparseBlockArray;

// TODO: rename
/// Basic interface for accessing block masks. Can work with `SimpleIter`.
pub trait LevelMasks{
    type Level0Mask;
    fn level0_mask(&self) -> Self::Level0Mask;

    type Level1Mask;
    /// # Safety
    ///
    /// index is not checked
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask;
    
    type DataBlock;
    /// # Safety
    ///
    /// indices are not checked
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize)
        -> Self::DataBlock;
}