use std::borrow::Borrow;
use crate::BitBlock;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::iter2::Iter2;
use crate::sparse_array::level_indices;
use crate::utils::{Borrowable, Take};

/// 
/// TODO: Change description
///
// We need xxxxType for each concrete level_block/mask type to avoid the need for use `for<'a>`,
// which is still not working (at Rust level) in cases, where we need it most.
pub trait SparseHierarchy2: Sized + Borrowable<Borrowed=Self> {
    /// TODO: Decription form hi_sparse_bitset TRUSTED_HIERARCHY
    const EXACT_HIERARCHY: bool;
    
    /// Hierarchy levels count (without a data level).
    type LevelCount: ConstInteger;
    
    type LevelMaskType: BitBlock;
    type LevelMask<'a>: Borrow<Self::LevelMaskType> + Take<Self::LevelMaskType>
        where Self: 'a;
 
    type DataType;
    type Data<'a>: Borrow<Self::DataType> + Take<Self::DataType>
        where Self: 'a;
 
    /// Element may not exists, but `level_indices` must be in range.
    unsafe fn data<I>(&self, level_indices: I) -> Option<Self::Data<'_>>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy;
 
    /// pointed element must exists
    unsafe fn data_unchecked<I>(&self, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy;
    
    type State: SparseHierarchyState2<This = Self>; 
    
    #[inline]
    fn iter(&self) -> Iter2<Self>{
        Iter2::new(self)
    }
    
    /// # Panics
    /// 
    /// Will panic if `index` is outside [max_range()].
    #[inline]
    fn get(&self, index: usize) -> Option<Self::Data<'_>> {
        assert!(index <= Self::max_range(), "index out of range!");
        let indices = level_indices::<Self::LevelMaskType, Self::LevelCount>(index);
        unsafe{ self.data(indices) }
    }
    
    /// # Safety
    ///
    /// Item at `index` must exist.
    #[inline]
    unsafe fn get_unchecked(&self, index: usize) -> Self::Data<'_> {
        let indices = level_indices::<Self::LevelMaskType, Self::LevelCount>(index);
        self.data_unchecked(indices)
    }    
    
    /// Max index this SparseHierarchy can contain.
    /// 
    /// Act as `const` - noop.
    #[inline]
    /*const*/ fn max_range() -> usize {
        Self::LevelMaskType::SIZE.pow(Self::LevelCount::VALUE as _) - 1 
    }    
}

/// Stateful [SparseHierarchy2] interface.
/// 
/// Having state allows implementations to have cache level meta-info.
/// If level block changed seldom and not sporadically (like during iteration) -
/// this can get a significant performance boost, especially in generative [SparseHierarchy2]'ies.
/// 
/// Block levels must be selected top(0) to bottom(last N) level.
/// When you [select_level_node], all levels below considered **not** selected.
/// For example, for 3-level hierarchy you select level 0, 1, 2 and then you can
/// access data level. But if after that, you change/select level 1 block - 
/// you should select level 2 block too, before accessing data level again. 
/// Imagine that you are traversing a tree.    
///
/// # Example
/// 
/// For 2-level 64bit hierarchy:
/// ```
/// // Select the only level0 block (corresponds to array indices [0..4096))
/// let mask0 = state.select_level_node(array, ConstInt::<0>, 0);
/// // Select 4th level1 block (corresponds to array indices [192..256))
/// let mask1 = state.select_level_node(array, ConstInt::<1>, 3);
/// // Select 9th data block (array index 201)
/// let data = state.data(array, 9);
/// ``` 
pub trait SparseHierarchyState2 {
    type This: SparseHierarchy2;
    
    fn new(this: &Self::This) -> Self;
    
    /// Item at index may not exist. Will return empty mask in such case.
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self,
        this: &'a Self::This,
        level_n: N, 
        level_index: usize,
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a>;
    
    /// Pointed node must exists
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self,
        this: &'a Self::This,
        level_n: N, 
        level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a>;
    
    /// Item at index may not exist.
    unsafe fn data<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> Option<<Self::This as SparseHierarchy2>::Data<'a>>;      
 
    /// Pointed data must exists
    unsafe fn data_unchecked<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> <Self::This as SparseHierarchy2>::Data<'a>;        
}