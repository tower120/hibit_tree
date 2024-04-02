mod primitive;
mod primitive_array;
mod array;
pub mod level;
mod bit_utils;
mod bool_type;
mod bit_block;
mod apply;
mod level_masks;

pub mod bit_queue;
pub mod simple_iter;
pub mod caching_iter;
//mod ref_or_val;
pub mod level_block;

//pub use ref_or_val::*;
pub use bit_block::BitBlock;
pub use primitive::Primitive;
pub use primitive_array::PrimitiveArray;
pub use array::SparseBlockArray;
pub use apply::{Apply, Op};
pub use level_masks::LevelMasks;

use std::borrow::Borrow;
use bool_type::BoolType;

#[inline]
pub(crate) fn data_block_index<T: LevelMasks>(level0_index: usize, level1_index: usize) -> usize {
    let mut index = level1_index;
    if !T::Level1Bypass::VALUE {
        index += level0_index << T::Level1MaskType::SIZE_POT_EXPONENT; 
    }
    index
}

/// convert T to value. noop for value, clone - for reference.
///
/// # Note
///
/// Surprisingly, there is no such thing in `std`. The closest one
/// is `Cow` enum, with runtime overhead.
pub trait IntoOwned<T>{
    fn into_owned(self) -> T;
}
impl<T> IntoOwned<T> for T{
    #[inline]
    fn into_owned(self) -> T{
        self
    }
}
impl<T: Clone> IntoOwned<T> for &T{
    #[inline]
    fn into_owned(self) -> T{
        self.clone()
    }
}

#[inline]
pub fn apply<Op, S1, S2>(op: Op, s1: S1, s2: S2) -> Apply<Op, S1, S2>
where
    Op: apply::Op,
    S1: LevelMasksBorrow,
    S2: LevelMasksBorrow,
{
    Apply{op, s1, s2}
}

// TODO: As long as iterator works with &LevelMasks - we can just
//       use Borrow<impl LevelMasks> everywhere
pub trait LevelMasksBorrow: Borrow<Self::Type>{
    type Type: LevelMasks;
}


