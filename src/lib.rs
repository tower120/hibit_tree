#![feature(dropck_eyepatch)]

#![cfg_attr(docsrs, feature(doc_cfg))]

//! {TODO: This is changed}
//! The core of the lib is [SparseArray] container and [SparseHierarchy] 
//! interface. They represent concept of data structure that filled
//! with "empty" elements across whole range, and populated with values.    
//! 
//! {TODO: This is changed}
//! All elements that are not actually stored in [SparseArray], 
//! considered to be [Empty::empty()]. Accessing such elements
//! does not involve branching, and as fast as accessing the real data.
//! 
//! Also inter container intersection and merging possible. All merged/intersected
//! element indices are become known basically instantly, since they obtained in bulk 
//! as bitmasks primitive operations(AND/OR). So intersection is very, very cheap.
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
//! Unordered iteration is as fast as it can possibly be. It is just plain Vec iteration.
//! 
//! ## Benchmarks data
//! 
//! At any default configuration random access and ordered iteration 
//! is always faster then no_hash HashMap with usize uniformly distributed keys 
//! (ideal HashMap scenario). [config::sbo] is faster up to 4 levels.
//! Shallow trees (2-3 levels) are up to x3 faster.
//! 
//! In general, performance does not depends on data distribution across index range.
//!
//! Insertion is not benchmarked, but it can be viewed as special case of random access.
//!
//! Iteration of intersection between N [SparseArray]s in worst case scenario,
//! where all elements intersects, took N times of usual ordered iteration. In best
//! case scenario where nothing intersects - basically free. Finding intersected
//! sub-trees costs almost nothing by itself. [SparseArray] acts as acceleration
//! structure for intersection.
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
//! Speeds up following operations:
//! - TODO [Eq]
//! - TODO [is_empty()]
//! - TODO [contains()]
//! - TODO From<impl SparseHierarchy>
//! - iterated elements are guaranteed to be ![is_empty].

mod sparse_array;
mod sparse_array_levels;
mod compact_sparse_array;
mod bit_utils;
mod bit_block;
mod sparse_hierarchy;
mod iter;
mod level;
mod level_block;
mod req_default;

pub mod ops;
pub mod bit_queue;
//mod ref_or_val;
pub mod const_utils;
pub mod utils;
pub mod config;

//pub use ref_or_val::*;
pub use bit_block::BitBlock;
pub use req_default::ReqDefault;
pub use sparse_array::SparseArray;
pub use compact_sparse_array::CompactSparseArray;
pub use sparse_hierarchy::*;
pub use iter::*;
pub use ops::map::map;
pub use ops::intersection::intersection;
pub use ops::union::union;
pub use ops::multi_union3::multi_union;
pub use ops::multi_intersection2::multi_intersection;

use std::borrow::Borrow;
use std::ops::BitAnd;
use const_utils::const_int::{ConstInteger, ConstIntVisitor};
use utils::primitive::Primitive;
use utils::array::Array;
use level::IntrusiveListLevel;
use utils::Borrowable;

// TODO: move to sparse_array / level_block ?
pub(crate) trait Empty {
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
pub(crate) fn data_block_index<LevelCount: ConstInteger, LevelMaskType: BitBlock>(
    level_indices: &impl Array<Item=usize>,
    data_index: usize
) -> usize {
    let level_count = LevelCount::VALUE;
    let mut acc = data_index;
    for N in 0..level_count - 1 {
        acc += level_indices.as_ref()[N] << (LevelMaskType::SIZE.ilog2() as usize * (level_count - N - 1));
    }
    acc
}