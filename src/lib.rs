#![feature(maybe_uninit_array_assume_init)]

#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(feature = "may_dangle", feature(dropck_eyepatch))]

//! # Hibit tree[^hibit]
//!
//! Fixed-depth, K-ary[^k_ary] tree[^trie] with integer keys,
//! that form bitmap hierarchy[^bitmap_hierarchy].
//! 
//! * Branchless O(1) access.
//! * No tree balancing.
//! * Ordered.
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
//! Ordered iteration is fast. Traversing each hierarchy node is fast O(1)
//! operation, which basically is just BMI's pop_cnt/trail_cnt. There is no "scan"
//! across node child items, for finding non-empty child/sub-tree.
//! 
//! Iteration of intersection between N trees in worst case scenario,
//! where trees have keys located nearby (fit same terminal blocks), 
//! take N/block_width times of usual ordered iteration. In the best
//! case scenario where nothing intersects, and this happens at the first levels - 
//! basically free. 
//! Hierarchical bitmap acts as acceleration structure for intersection.
//! Branches/sub-trees that have no keys in common index-range discarded early.
//! 
//! ## Inter HibitTree operations
//! 
//! As you can see [HibitTree] is a form of set/map, and hence, can be used for
//! inter set operations, such as [intersection], [union], etc. 
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
//! - TODO `Eq`
//! - TODO `is_empty()`
//! - TODO `contains()`
//! 
//! [EXACT_HIERARCHY]: HibitTree::EXACT_HIERARCHY
//! 
//! ## Flags
//! 
//! ### simd
//! 
//! Enabled by default. Allow to use 128, 256 bit configurations.
//! 
//! ### may_dangle
//! 
//! Requires nightly. Allow to store references in containers (TODO: not implemented).
//! See [rustonomicon](https://doc.rust-lang.org/nomicon/dropck.html#an-escape-hatch).

mod bit_utils;

mod bit_block;
pub use bit_block::BitBlock;

mod index;
pub use index::*;

mod hibit_tree;
pub use hibit_tree::*;

mod iter;
pub use iter::*;

mod req_default;
pub use req_default::ReqDefault;

pub mod ops;
pub use ops::map::{map, map_w_default};
pub use ops::intersection::intersection;
pub use ops::union::{union, union_w_default};
pub use ops::_multi_intersection::multi_intersection;
pub use ops::_multi_union::{multi_union, multi_union_w_default};

pub mod bit_queue;
pub mod const_utils;
pub mod utils;
pub mod config;

mod tree;
pub use tree::Tree;

// Just for macros examples
//mod ref_or_val;
//pub use ref_or_val::*;