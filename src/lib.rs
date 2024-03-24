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
pub mod caching_iter;
mod ref_or_val;
mod level_masks;

use std::marker::PhantomData;
pub use ref_or_val::*;
pub use bit_block::BitBlock;
pub use primitive::Primitive;
pub use primitive_array::PrimitiveArray;
pub use array::SparseBlockArray;
pub use apply::{Apply, Op};
pub use level_masks::LevelMasks;
use std::borrow::Borrow;

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

pub trait LevelMasksBorrow: Borrow<Self::Type>{
    type Type: LevelMasks;
}


