mod primitive;
mod primitive_array;
mod array;
pub mod level;
mod bit_utils;
mod bool_type;
mod bit_block;
mod apply;

pub mod level_masks;
pub mod bit_queue;
//pub mod simple_iter;
pub mod caching_iter;
//mod ref_or_val;
pub mod level_block;

//pub use ref_or_val::*;
pub use bit_block::{BitBlock, IEmptyBitBlock};
pub use primitive::Primitive;
pub use primitive_array::PrimitiveArray;
pub use array::SparseBlockArray;
pub use apply::{Apply, Op};

use std::borrow::Borrow;
use bool_type::BoolType;
use level_masks::LevelMasks;
use crate::level_masks::{level_bypass, LevelBypass, LevelMasksBorrow, LevelMasksIter};

#[inline]
pub(crate) fn data_block_index<T: LevelMasks>(
    level0_index: usize, 
    level1_index: usize,
    index: usize
) -> usize {
    match level_bypass::<T>(){
        LevelBypass::None => {
            (level0_index << (T::Level1MaskType::SIZE_POT_EXPONENT + T::Level2MaskType::SIZE_POT_EXPONENT))
            + (level1_index << T::Level2MaskType::SIZE_POT_EXPONENT)    
            + index
        }
        LevelBypass::Level2 => {
            (level0_index << T::Level1MaskType::SIZE_POT_EXPONENT)
            + index
        }
        LevelBypass::Level1Level2 => {
            index
        }
    }
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