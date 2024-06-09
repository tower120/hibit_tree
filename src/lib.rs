#![feature(associated_type_bounds)]
#![feature(inline_const)]

//! The core of the lib is [SparseArray] container and [SparseHierarchy] 
//! interface. They represent concept of data structure that filled
//! with "empty" elements across whole range, and populated with values.    
//! 
//! All elements that are not actually stored in [SparseArray], 
//! considered to be [Empty::empty()]. Accessing such elements
//! does not involve branching, and as fast as accessing the real data.
//! 
//! Also inter container intersection and merging possible. All merged/intersected
//! element indices are become known basically instantly, since they obtained in bulk 
//! as bitmasks primitive operations(AND/OR). So intersection is basically free.
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
//! With bitmasks, instead of searching non-empty node child in childs array,
//! we just iterate bitmask population.
//! Also, bitmasks allows **FAST** container-to-container intersections.
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
//! "Exact hierarchy" - is hierarchy that DOES NOT have nodes pointing to 
//! empty elements or nodes. Hence, it's bitmasks contains "exact" emptiness info.
//! 
//! If you can guarantee that your ![EXACT_HIERARCHY] SparseHierarchy is 
//! actually exact - you can use [ExactHierarchy]. 
//! 
//! Speeds up following operations:
//! - [Eq]
//! - [is_empty()]
//! - [contains()]
//! - TODO From<impl SparseHierarchy>
//! - iterated elements are guaranteed to be ![is_empty].
//! 
//! N.B. In order to meet "exact hierarchy" constraints, [SparseArray] would have
//! to check data for emptiness after each mutated access. Since you may never
//! actually use operations that speed-up by [EXACT_HIERARCHY] (or performance 
//! gains does not matter) - we made [SparseArray] non-exact.
//! TODO Use [ExactSparseArray] for "exact hierarchy" version.

mod sparse_array;
mod sparse_array_levels;
mod bit_utils;
mod bit_block;
mod apply;
mod fold;
//mod empty;
mod exact_hierarchy;
mod sparse_hierarchy;
mod ops;
mod op;

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
pub use apply::Apply;
pub use fold::Fold;
//pub use empty::Empty;
pub use sparse_hierarchy::*;
pub use exact_hierarchy::ExactHierarchy;
pub use ops::*;
pub use op::*;

use std::borrow::Borrow;
use std::ops::BitAnd;
pub use op::BinaryOp;
use crate::const_utils::const_int::{ConstInteger, ConstIntVisitor};
use utils::primitive::Primitive;
use utils::array::Array;
use crate::level::IntrusiveListLevel;
use crate::utils::Borrowable;

pub trait Empty {
    fn empty() -> Self;
    fn is_empty(&self) -> bool;
}

impl<T> Empty for Option<T>{
    #[inline]
    fn empty() -> Self {
        None
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_none()
    }
}

/// [Empty] that can be used as a node in intrusive list.
/// 
/// Implementing this will allow your [Empty] struct in an empty state 
/// to be used as a LinkedList node with [IntrusiveListLevel]. 
pub(crate) trait MaybeEmptyIntrusive: Empty {
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

/// Apply [BinaryOp] between two [SparseHierarchy]ies.
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

/// Fold [SparseHierarchy]ies into virtual one using [BinaryOp]. 
/// 
/// # Arguments
/// 
/// * `Op`::[data_op] in form of `(Init, ArrayIter::Item) -> Init`.
/// * All `LevelMask`s and `LevelCount`s must match (have same hierarchy configurations).
/// * `init`'s [DataType] must be [Clone]able. This restriction may be lifted in the future.
/// * `array_iter` will be cloned multiple times. Use cheaply cloneable iterator.
#[inline]
pub fn fold<Op, Init, ArrayIter>(op: Op, init: Init, array_iter: ArrayIter) 
    -> Fold<Op, Init, ArrayIter>
/*where
    Init: Borrowable<Borrowed:SparseHierarchy<DataType: Clone>>,
    ArrayIter: Iterator<Item:Borrowable<Borrowed:SparseHierarchy>> + Clone,
    Op: BinaryOp*/
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