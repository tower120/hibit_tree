#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "may_dangle", feature(dropck_eyepatch))]

//! # Hibit tree[^hibit]
//!
//! The core of the lib is [SparseTree] and [DenseTree] containers with [HibitTree] 
//! interface. These are fixed-depth, K-ary[^k_ary] trees[^trie] with integer keys,
//! that form bitmap hierarchy[^bitmap_hierarchy].
//! 
//! * Branchless O(1) access.
//! * No tree balancing.
//! * Ordered.
//! * Unordered contiguous storage.
//! * Tree act as a bitset/bitmap hierarchy. Bitmap hierarchy is a natural
//!   acceleration structure for intersection. Allows super-fast set-like operations: 
//!   intersection, merge, etc.   
//!
//! [^hibit]: Hibit stands for **hi**erarchical **bit**map.
//! [^k_ary]: A.k.a. "N-ary", a.k.a. "M-ary". 
//! [^trie]: Also, may be considered as form of radix tree, a.k.a. "prefix tree", a.k.a. "trie".
//! [^bitmap_hierarchy]: Bitmap hierarchy - is a hierarchy of bitmasks, where each
//! raised bit in bitmask means, that child at corresponding bit index have data.
//! See [hi_sparse_bitset](https://crates.io/crates/hi_sparse_bitset), 
//! [hibitset](https://docs.rs/hibitset/0.6.4/hibitset).     
//! 
//! ## Data structure
//! 
//! See readme.
//! 
//! ## Performance
//! 
//! Accessing element by index act as dereferencing N pointers (where N - number
//! of levels in hierarchy). This is significantly faster then traversing tree 
//! with dynamic depth, since it does not involve any kind of branching.
//! 
//! Random insert have the same logic as random access, but with branching at each level.
//!
//! Ordered (by index) iteration is fast. Traversing each hierarchy node is fast O(1)
//! operation, which basically is just BMI's pop_cnt/trail_cnt. There is no "scan"
//! across node child items, for finding non-empty child/sub-tree.
//! 
//! Unordered iteration is as fast as a plain Vec iteration.
//! 
//! Iteration of intersection between N trees in worst case scenario,
//! where trees have keys located nearby (fit same terminal blocks), 
//! take N/block_width[^block_width] times of usual ordered iteration. In the best
//! case scenario where nothing intersects, and this happens at the first levels - 
//! basically free. 
//! Hierarchical bitmap acts as acceleration structure for intersection.
//! Branches/sub-trees that have no keys in common index-range discarded early.
//! 
//! [^block_width]: 64 for [DenseTree]. Can be up to 256 for [SparseTree]. 
//! 
//! ### Benchmarks data
//! 
//! #### Against HashMap
//! 
//! Comparing random access against `no_hash` HashMap with usize uniformly distributed keys 
//! (ideal HashMap scenario):
//! * 4-6 levels (u32 range) both containers are faster.
//! * 8 levels (u64 range) [SparseTree] with 256bit have the same performance.
//! 
//! TODO: add graphic image
//! 
//! Against `ahash` - both containers always faster.
//! 
//! TODO: add graphic image
//! 
//! In general, performance does not depends on data distribution across index range.
//!
//! Random insert is not benchmarked yet.
//! 
//! Bulk insert with [materialize] is not benchmarked yet. Should be **significantly** 
//! faster then random insert.
//! 
//! Intersection is order of magnitudes faster then HashMap's per-element "contains". 
//!
//! Unordered iteration is faster then current [hashbrown](https://crates.io/crates/hashbrown)
//! implementation.   
//! 
//! ## Inter HibitTree operations
//! 
//! As you can see [HibitTree] is a form of set/map, and hence, can be used for
//! inter set operations, such as [intersection], [merge], etc. 
//! 
//! Due to the fact, that each hierarchy block supplemented with bitmask, finding
//! intersection is just a matter of ANDing bitmasks.
//! 
//! ## Laziness
//! 
//! All [ops] are [LazyHibitTree]s. Which means that tree is computed on the fly.
//! If you need to iterate the result once or twice, or [get()] a few times - there
//! is no need to save result into a concrete container. Otherwise - you may want 
//! to [materialize] it. Obviously, there is no need to materialize [map] that just
//! return reference to object field.
//!
//! Due to current limitations you can't materialize references[^store_ref].
//!
//! [^store_ref]: But you can have &T containers in general. You'll need 
//! [may_dangle](#may_dangle) flag.
//! 
//! [get()]: HibitTree::get
//! [materialize]: LazyHibitTree::materialize
//! [map]: crate::map 
//! 
//! ## Exact hierarchy
//! 
//! "Exact hierarchy" - is a bitmap hierarchy, where each bitmask have
//! exact emptiness info. All raised bits in bitmasks corresponds to non-empty childs.
//! Or from the tree view: there can be no empty node in tree, except root. 
//! 
//! You can have non-[EXACT_HIERARCHY] in [LazyHibitTree]. For example, lazy 
//! intersection.  
//! 
//! Speeds up following operations:
//! - [FromHibitTree]
//! - TODO [Eq]
//! - TODO [is_empty()]
//! - TODO [contains()]
//! 
//! [EXACT_HIERARCHY]: HibitTree::EXACT_HIERARCHY
//! 
//! ## Flags
//! 
//! ### simd
//! 
//! Enabled by default. Allow to use 128, 256 bit configurations in [SparseTree].
//! 
//! ### may_dangle
//! 
//! Requires nightly. Allow to store references in containers.
//! See [rustonomicon](https://doc.rust-lang.org/nomicon/dropck.html#an-escape-hatch).

mod sparse_tree;
mod sparse_tree_levels;
mod dense_tree;
mod bit_utils;
mod bit_block;
mod hibit_tree;
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
pub use sparse_tree::SparseTree;
pub use dense_tree::DenseTree;
pub use hibit_tree::*;
pub use iter::*;
pub use ops::map::map;
pub use ops::multi_map_fold::multi_map_fold;
pub use ops::intersection::intersection;
pub use ops::union::union;
pub use ops::_multi_intersection::multi_intersection;
pub use ops::_multi_union::multi_union;

use std::borrow::Borrow;
use std::ops::BitAnd;
use const_utils::const_int::{ConstInteger, ConstIntVisitor};
use utils::Primitive;
use utils::Array;
use level::IntrusiveListLevel;
use utils::Borrowable;
use crate::const_utils::{ConstCopyArrayType, ConstUsize};

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

// TODO: try replace with accumulated key on fly calculation.
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

// TODO: make public
// Compile-time loop inside. Ends up with N (AND + SHR)s.
#[inline]
pub(crate) fn level_indices<LevelMask, LevelsCount>(index: usize)
     -> ConstCopyArrayType<usize, LevelsCount>
where
    LevelMask: BitBlock,
    LevelsCount: ConstInteger,
{
    // TODO: need uninit?
    let mut level_indices = ConstCopyArrayType::<usize, LevelsCount>::from_fn(|_|0);
    
    let mut level_remainder = index;
    let level_count = LevelsCount::VALUE;
    for level in 0..level_count - 1 {
        // LevelMask::SIZE * 2^(level_count - level - 1)
        let level_capacity_exp = LevelMask::SIZE.ilog2() as usize * (level_count - level - 1);
        let level_capacity = 1 << level_capacity_exp;
        
        // level_remainder / level_capacity_exp
        let level_index = level_remainder >> level_capacity_exp;
        
        // level_remainder % level_capacity_exp
        level_remainder = level_remainder & (level_capacity - 1);
        
        level_indices.as_mut()[level] = level_index; 
    }
    
    *level_indices.as_mut().last_mut().unwrap() = level_remainder; 
    
    level_indices
}

#[cfg(test)]
#[test]
fn test_level_indices_new(){
    {
        let indices = level_indices::<u64, ConstUsize<2>>(65);
        assert_eq!(indices, [1, 1]);
    }
    {
        let lvl0 = 262_144; // Total max capacity
        let lvl1 = 4096;
        let lvl2 = 64;
        let indices = level_indices::<u64, ConstUsize<3>>(lvl1*2 + lvl2*3 + 4);
        assert_eq!(indices, [2, 3, 4]);
    }
    {
        let indices = level_indices::<u64, ConstUsize<3>>(32);
        assert_eq!(indices, [0, 0, 32]);
    }
    {
        let indices = level_indices::<u64, ConstUsize<2>>(32);
        assert_eq!(indices, [0, 32]);
    }    
    {
        let indices = level_indices::<u64, ConstUsize<1>>(32);
        assert_eq!(indices, [32]);
    }
}