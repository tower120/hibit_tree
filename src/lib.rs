mod primitive;
mod primitive_array;
mod array;
mod level;
mod bit_utils;
mod bit_block;
mod apply;

pub mod bit_queue;
pub mod block;
pub mod simple_iter;
mod ref_or_val;

use std::marker::PhantomData;
pub use ref_or_val::*;
pub use bit_block::BitBlock;
pub use primitive::Primitive;
pub use primitive_array::PrimitiveArray;
pub use array::SparseBlockArray;
pub use apply::{Apply, Op};


// TODO: rename
/// Basic interface for accessing block masks. Can work with `SimpleIter`.
pub trait LevelMasks{
    type Level0Mask: BitBlock;
    fn level0_mask(&self) -> Self::Level0Mask;

    type Level1Mask: BitBlock;
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

#[inline]
pub fn apply<Op, S1, S2>(op: Op, s1: S1, s2: S2) -> Apply<Op, S1, S2>
where
    Op: apply::Op,

    S1: RefOrVal,
    S1::Type: LevelMasks,

    S2: RefOrVal,
    S2::Type: LevelMasks,
{
    Apply{s1, s2, phantom: PhantomData}
}