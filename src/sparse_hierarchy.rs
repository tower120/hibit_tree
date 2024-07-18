use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::RangeTo;
use crate::BitBlock;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::iter::Iter;
use crate::sparse_array::level_indices;
use crate::utils::{Borrowable, Take};

// Should be just <const WIDTH: usize, const DEPTH: usize>, but RUST not yet
// support that for our case.
/// Range checked index. 
/// 
/// Known to be in range for `SparseHierarchy<LevelMaskType, LevelCount>`.
/// 
/// Whenever you see `impl Into<Index<_, _>>` - you can just use your `usize` index
/// as usual.
///  
/// Index range check is very cheap, and is just one assert_eq with constant value.
/// But in tight loops you may want to get rid of that check - and that's the sole
/// purpose of `Index`.  
///
/// ```
/// #use hi_sparse_array::Index;
///  
/// // use it just as usize
/// array.get(12);
/// 
/// // zero-cost unsafe construction
/// array.get(unsafe{ Index::new_unchecked(12) });
/// 
/// // safe construct once, then reuse
/// {
///     let i = Index::from(12);
///     array.get(i);
///     array2.get(i);
/// }
/// ``` 
#[derive(Copy, Clone)]
pub struct Index<LevelMaskType: BitBlock, LevelCount: ConstInteger>(
    usize, PhantomData<(LevelMaskType, LevelCount)>
);

impl<LevelMaskType: BitBlock, LevelCount: ConstInteger> 
    Index<LevelMaskType, LevelCount>
{
    /// # Safety
    ///
    /// You must guarantee that index is in SparseHierarchy<LevelMaskType, LevelCount> range.
    #[inline]
    pub unsafe fn new_unchecked(index: usize) -> Self {
        Self(index, Default::default())
    }
}

/// usize -> SparseHierarchyIndex
impl<LevelMaskType: BitBlock, LevelCount: ConstInteger> From<usize>
for
    Index<LevelMaskType, LevelCount>
{
    /// # Panic
    ///
    /// Panics if index is not in SparseHierarchy<LevelMaskType, LevelCount> range.
    #[inline]
    fn from(index: usize) -> Self {
        let range_end = LevelMaskType::SIZE.pow(LevelCount::VALUE as _);
        assert!(index < range_end, "Index {index} is out of SparseHierarchy range.");
        unsafe{ Self::new_unchecked(index) }
    }
}

/// SparseHierarchyIndex -> usize 
impl<LevelMaskType: BitBlock, LevelCount: ConstInteger> 
    From<Index<LevelMaskType, LevelCount>>
for usize
{
    #[inline]
    fn from(value: Index<LevelMaskType, LevelCount>) -> Self {
        value.0
    }
}

/// 
/// TODO: Change description
///
// We need xxxxType for each concrete level_block/mask type to avoid the need for use `for<'a>`,
// which is still not working (at Rust level) in cases, where we need it most.
pub trait SparseHierarchy: Sized + Borrowable<Borrowed=Self> {
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
 
    /// Element may not exists, but `index` must be in range, and `level_indices` must
    /// corresponds to `index`.
    unsafe fn data<I>(&self, index: usize, level_indices: I) -> Option<Self::Data<'_>>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy;
 
    /// pointed element must exists,  and `level_indices` must
    /// corresponds to `index`.
    unsafe fn data_unchecked<I>(&self, index: usize, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy;
    
    type State: SparseHierarchyState<This = Self>; 
    
    #[inline]
    fn iter(&self) -> Iter<Self>{
        Iter::new(self)
    }

    /// You can use `usize` or [Index] for `index`.
    #[inline]
    fn get(&self, index: impl Into<Index<Self::LevelMaskType, Self::LevelCount>>) 
        -> Option<Self::Data<'_>> 
    {
        let index: usize = index.into().into();
        let indices = level_indices::<Self::LevelMaskType, Self::LevelCount>(index);
        unsafe{ self.data(index, indices) }
    }

    /// # Safety
    ///
    /// Item at `index` must exist.
    #[inline]
    unsafe fn get_unchecked(&self, index: usize) -> Self::Data<'_> {
        let indices = level_indices::<Self::LevelMaskType, Self::LevelCount>(index);
        self.data_unchecked(index, indices)
    }
    
    /// Index range this SparseHierarchy can handle - `0..width^depth`.
    /// 
    /// Indices outside of this range considered to be invalid.
    /// 
    /// Act as `const`.
    #[inline]
    /*const*/ fn index_range() -> RangeTo<usize> {
        RangeTo{ end: Self::LevelMaskType::SIZE.pow(Self::LevelCount::VALUE as _) }
    }
}

/// Stateful [SparseHierarchy] interface.
/// 
/// Having state allows implementations to have cache level meta-info.
/// If level block changed seldom and not sporadically (like during iteration) -
/// this can get a significant performance boost, especially in generative [SparseHierarchy]'ies.
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
pub trait SparseHierarchyState {
    type This: SparseHierarchy;
    
    fn new(this: &Self::This) -> Self;
    
    /// Item at index may not exist. Will return empty mask in such case.
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self,
        this: &'a Self::This,
        level_n: N, 
        level_index: usize,
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a>;
    
    /// Pointed node must exists
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self,
        this: &'a Self::This,
        level_n: N, 
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a>;
    
    /// Item at index may not exist.
    unsafe fn data<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> Option<<Self::This as SparseHierarchy>::Data<'a>>;      
 
    /// Pointed data must exists
    unsafe fn data_unchecked<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::Data<'a>;        
}