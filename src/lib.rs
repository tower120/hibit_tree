#![feature(associated_type_bounds)]

//! The core of the lib is [SparseArray] container and [SparseHierarchy] 
//! interface. They represent concept of data structure that filled
//! with "empty" elements across whole range, and populated with values.    
//! 
//! All elements that are not actually stored in [SparseArray], 
//! considered to be [MaybeEmpty::empty()]. Accessing such elements
//! does not involve branching, and as fast as accessing the real data.
//! 
//! Also inter container intersection and merging possible. With fast O(1) 
//! intersected/merged element search.
//!
//! # Data structure
//! 
//! TODO: image of container structure from hi_sparse_bitset.
//! 
//! TODO: level block description from hi_sparse_bitset.
//! 
//! ## Bitmasks
//! 
//! Each node supplemented with bitmask, where raised bits corresponds to
//! sub-tree childs with data. All other node childs point to the empty data.
//! This allows to iterate non-empty nodes without search.
//! This also allows **FAST** container-to-container intersections.
//! 
//! # Performance
//! 
//! Accessing element by index act as dereferencing N pointers (where N - number
//! of levels in hierarchy). This is significantly faster then traversing tree 
//! with dynamic depth, since it does not involve any kind of branching.
//! 
//! Insert basically same as by index element access, plus some minor overhead.
//!
//! Ordered (by index) iteration is fast. Traversing each hierarchy node is fast O(1)
//! operation, which basically is just BMI's pop_cnt/trail_cnt. There is no "scan"
//! across node child items, for finding non-empty child/sub-tree.
//! 
//! Unordered iteration is as fast as it can possibly be. It just iterating Vec.
//! 
//! # Inter SparseHierarchy operations
//! 
//! As you can see SparseArray is a form of set/map, and hence, can be used for
//! inter set operations, such as intersection, merge, diff. 
//! 
//! Due to the fact, that each hierarchy block supplemented with bitmask, finding
//! intersection is just a matter of ANDing bitmasks.
//! 
//! # Exact hierarchy
//! 
//! TODO

mod sparse_array;
mod sparse_array_levels;
mod bit_utils;
mod bit_block;
mod apply;
mod fold;
//mod empty;
mod exact_sparse_hierarchy;
/*pub*/ mod sparse_hierarchy;

pub mod bit_queue;
//pub mod simple_iter;
pub mod caching_iter;
//mod ref_or_val;
pub mod level;
pub mod level_block;
pub mod const_utils;
pub mod utils;

//pub use ref_or_val::*;
pub use bit_block::BitBlock;
pub use sparse_array::SparseArray;
pub use sparse_array_levels::SparseArrayLevels;
pub use apply::{Apply, Op};
pub use fold::Fold;
//pub use empty::Empty;
pub use sparse_hierarchy::*;
pub use exact_sparse_hierarchy::ExactSparseHierarchy;

use std::borrow::Borrow;
//use sparse_hierarchy::SparseHierarchy;
use crate::const_utils::const_int::{ConstInteger, ConstIntVisitor};
use utils::primitive::Primitive;
use utils::array::Array;

pub trait MaybeEmpty {
    fn empty() -> Self;
    fn is_empty(&self) -> bool;
}

/// [MaybeEmpty] that can be used as a node in intrusive list.
/// 
/// Implementing this will allow your [MaybeEmpty] struct in an empty state 
/// to be used as a LinkedList node with [IntrusiveListLevel]. 
pub(crate) trait MaybeEmptyIntrusive: MaybeEmpty {
    fn as_u64_mut(&mut self) -> &mut u64;
    /// Restore [empty()] state, after [as_u64_mut()] mutation.
    fn restore_empty(&mut self);
}

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

#[inline]
pub fn apply<Op, B1, B2>(op: Op, s1: B1, s2: B2) -> Apply<Op, B1, B2>
// TODO: more detail bounds?/ no bounds?
/*where
    Op: apply::Op,
    B1: Borrowable<Borrowed: SparseHierarchy>,
    B2: Borrowable<
        Borrowed: SparseHierarchy<
            LevelCount    = <B1::Borrowed as SparseHierarchy>::LevelCount,
            LevelMaskType = <B1::Borrowed as SparseHierarchy>::LevelMaskType,
        >
    >,*/
{
    Apply{op, s1, s2}
}

#[inline]
pub fn fold<Op, Init, ArrayIter>(op: Op, init: Init, array_iter: ArrayIter) 
    -> Fold<Op, Init, ArrayIter>
/*where
    Op: apply::Op,
    ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: SparseHierarchy,
    Init: SparseHierarchy,*/
{
    Fold{op, init, array_iter}
}

/*pub type Reduce<'a, Op, ArrayIter, Array> = Fold<'a, Op, Array, ArrayIter, Array>;
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