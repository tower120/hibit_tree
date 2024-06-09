use std::borrow::Borrow;
use crate::const_utils::ConstBool;
use crate::{BitBlock, Empty, SparseHierarchy};
use crate::utils::Take;

// We need more advanced GAT in Rust to make `DataBlock<'a>` work here 
// in a meaningful way.
// For now, should be good enough as-is.
/// Operation applied between two [SparseHierarchy]ies.
/// 
/// Define both hierarchical structure and items of resulting [SparseHierarchy].
pub trait BinaryOp {
    const EXACT_HIERARCHY: bool;
    
    /// Check and skip empty hierarchies? Any value is safe. Use `false` as default.
    /// 
    /// This incurs some performance overhead, but can greatly reduce
    /// algorithmic complexity of some [Fold] operations.
    /// 
    /// # In-depth
    /// 
    /// For example, merge operation will OR level1 mask, and some of the
    /// raised bits of resulting bitmask will not correspond to the raised bits
    /// of each source bitmask:
    /// ```text
    /// L 01100001      
    /// R 00101000
    /// ----------
    /// O 01101001    
    /// ```
    /// R's second bit is 0, but resulting bitmask's bit is 1.
    /// This means that in level2 R's second block's mask will be loaded, 
    /// thou its empty and can be skipped.
    /// 
    /// [Fold] cache hierarchy blocks for faster traverse. And when this flag
    /// is raised - it checks and does not add empty blocks to the cache list. 
    ///
    /// Notice though, that such thing cannot happen with intersection. 
    /// So trying to apply such optimization there, would be a waste of resources.   
    /// 
    type SKIP_EMPTY_HIERARCHIES: ConstBool;
    
    type LevelMask: BitBlock;
    
    /// Operation applied to level masks.
    fn lvl_op(&self,
        left : impl Borrow<Self::LevelMask> + Take<Self::LevelMask>,
        right: impl Borrow<Self::LevelMask> + Take<Self::LevelMask>
    ) -> Self::LevelMask;
    
    type Left;
    type Right;
    type Out: Empty;
    
    /// Operation applied to data items.
    fn data_op(&self,
       left : impl Borrow<Self::Left>  + Take<Self::Left>,
       right: impl Borrow<Self::Right> + Take<Self::Right>
    ) -> Self::Out;
}
