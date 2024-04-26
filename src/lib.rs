#![feature(associated_type_bounds)]

mod primitive;
mod primitive_array;
mod sparse_array;
pub mod level;
mod bit_utils;
mod bool_type;
mod bit_block;
//mod apply;
//mod fold;
//mod empty;

pub mod sparse_hierarchy;
pub mod bit_queue;
//pub mod simple_iter;
pub mod caching_iter;
//mod ref_or_val;
pub mod level_block;
pub mod const_int;

//pub use ref_or_val::*;
pub use bit_block::{BitBlock, IEmptyBitBlock, EmptyBitBlock};
pub use primitive::Primitive;
pub use primitive_array::{Array, PrimitiveArray};
pub use sparse_array::{SparseArray, SparseArrayLevels};
//pub use apply::{Apply, Op};
//pub use fold::Fold;
//pub use empty::Empty;


use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::ControlFlow;
use bool_type::BoolType;
use sparse_hierarchy::SparseHierarchy;
use crate::const_int::{const_for, ConstInt, ConstInteger, ConstIntVisitor};
//use crate::primitive_array::Array;
//use crate::sparse_hierarchy::{level_bypass, LevelBypass};

// Compile-time loop inside. Ends up with N ADDs.
#[inline]
pub(crate) fn data_block_index<T: SparseHierarchy>(
    level_indices: &impl Array<Item=usize>,
    data_index: usize
) -> usize {
    let level_count = T::LevelCount::VALUE;
    let mut acc = data_index;
    for N in 0..level_count - 1{
        acc += level_indices.as_ref()[N] << (T::LevelMaskType::SIZE_POT_EXPONENT* (level_count - N - 1));
    }
    acc
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

/*#[inline]
pub fn apply<Op, B1, B2, T1, T2>(op: Op, s1: B1, s2: B2) -> Apply<Op, B1, B2, T1, T2>
where
    Op: apply::Op,
    B1: Borrow<T1>,
    B2: Borrow<T2>,
{
    Apply{op, s1, s2, phantom: PhantomData}
}

#[inline]
pub fn fold<'a, Op, Init, ArrayIter, Array>(op: Op, init: &'a Init, array_iter: ArrayIter) -> Fold<'a, Op, Init, ArrayIter, Array>
where
    Op: apply::Op,
    ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: SparseHierarchy,
    Init: SparseHierarchy,
{
    Fold{op, init, array_iter, phantom: PhantomData}
}

pub type Reduce<'a, Op, ArrayIter, Array> = Fold<'a, Op, Array, ArrayIter, Array>;
#[inline]
pub fn reduce<'a, Op, ArrayIter, Array>(op: Op, mut array_iter: ArrayIter) -> Option<Reduce<'a, Op, ArrayIter, Array>>
where
    Op: apply::Op,
    ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: SparseHierarchy,
{
    if let Some(init) = array_iter.next(){
        Some(fold(op, init, array_iter))
    } else {
        None
    }
}*/