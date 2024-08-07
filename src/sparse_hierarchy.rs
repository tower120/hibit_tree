use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::RangeTo;
use crate::BitBlock;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::iter::Iter;
use crate::level_indices;
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
pub struct Index<LevelMask: BitBlock, LevelCount: ConstInteger>(
    usize, PhantomData<(LevelMask, LevelCount)>
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

pub trait SparseHierarchyTypes<'this, ImplicitBounds = &'this Self>{
    // TODO: Try remove DataType.
    type DataType;
    type Data: Borrow<Self::DataType> + Take<Self::DataType>;
}

/// 
/// TODO: Change description
///
/// # Design notes
/// 
/// As you can see, SparseHierarchy have lifetime parameter - this is workaround
/// for Rust's basically non-usable GATs [^gat_problems].
/// All it functions work with `&'a self` - so most of the time it will be just
/// auto-deducted.
/// 
/// [^gat_problems] With GAT's we always end up with this https://blog.rust-lang.org/2022/10/28/gats-stabilization.html#implied-static-requirement-from-higher-ranked-trait-bounds
/// error.
/// We use this technique https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats#hrtb-supertrait
/// as workaround. We don't use currently `self`, and it does not interference with type deduction 
/// (since we expect users to work heavily with [map] closures - that is ergonomically important).
// 
// We need xxxxType for each concrete level_block/mask type to avoid the need for use `for<'a>`,
// which is still not working (at Rust level) in cases, where we need it most.
pub trait SparseHierarchy: Sized + Borrowable<Borrowed=Self>
where
	Self: for<'this> SparseHierarchyTypes<'this>,
{
    /// TODO: Decription form hi_sparse_bitset TRUSTED_HIERARCHY
    const EXACT_HIERARCHY: bool;
    
    /// Hierarchy levels count (without a data level).
    type LevelCount: ConstInteger;
    
    type LevelMask: BitBlock;
 
    /*
    // TODO: We may not need it any more
    type DataType;
    type Data: Borrow<Self::DataType> + Take<Self::DataType>;*/
 
    /// # Safety
    /// 
    /// Element may not exist, but `index` must be in range, and `level_indices` must
    /// correspond to `index`.
    /// 
    /// `level_indices` must be [LevelCount] size[^1].
    /// 
    /// [^1]: It is not just `[usize; LevelCount::VALUE]` due to troublesome 
    ///       Rust const expressions in generic context. 
    unsafe fn data(&self, index: usize, level_indices: &[usize]) -> Option<<Self as SparseHierarchyTypes<'_>>::Data>;
 
    /// # Safety
    /// 
    /// pointed element must exist, and `level_indices` must
    /// corresponds to `index`.
    /// 
    /// `level_indices` must be [LevelCount] size[^1].
    /// 
    /// [^1]: It is not just `[usize; LevelCount::VALUE]` due to troublesome 
    ///       Rust const expressions in generic context. 
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) -> <Self as SparseHierarchyTypes<'_>>::Data;
    
    type State: SparseHierarchyState<This = Self>; 
    
    #[inline]
    fn iter(&self) -> Iter<Self>{
        Iter::new(self)
    }

    /// You can use `usize` or [Index] for `index`.
    #[inline]
    fn get(&self, index: impl Into<Index<<Self as SparseHierarchy>::LevelMask, Self::LevelCount>>) 
        -> Option<<Self as SparseHierarchyTypes<'_>>::Data> 
    {
        let index: usize = index.into().into();
        let indices = level_indices::<Self::LevelMask, Self::LevelCount>(index);
        unsafe{ self.data(index, indices.as_ref()) }
    }

    /// # Safety
    ///
    /// Item at `index` must exist.
    #[inline]
    unsafe fn get_unchecked(&self, index: usize) -> <Self as SparseHierarchyTypes<'_>>::Data {
        let indices = level_indices::<Self::LevelMask, Self::LevelCount>(index);
        self.data_unchecked(index, indices.as_ref())
    }
    
    /// Index range this SparseHierarchy can handle - `0..width^depth`.
    /// 
    /// Indices outside of this range considered to be invalid.
    /// 
    /// Act as `const`.
    #[inline]
    /*const*/ fn index_range() -> RangeTo<usize> {
        RangeTo{ end: Self::LevelMask::SIZE.pow(Self::LevelCount::VALUE as _) }
    }
}

/// [SparseHierarchy] that is not a concrete collection.
/// 
/// Most results of operations are.
pub trait LazySparseHierarchy: SparseHierarchy {
    /// Make a concrete collection from a lazy/virtual one.
    #[inline]
    fn materialize<T>(&self) -> T
    where
        T: FromSparseHierarchy,
        T: SparseHierarchy<
            LevelCount = Self::LevelCount,
            LevelMask  = Self::LevelMask,            
        >,
        for<'a> T: SparseHierarchyTypes<'a,
            
            DataType = <Self as SparseHierarchyTypes<'a>>::DataType
        >,
    {
        T::from_sparse_hierarchy(self)
    }
}

/// Construct a [SparseHierarchy] collection from any [SparseHierarchy].
pub trait FromSparseHierarchy: SparseHierarchy {
    fn from_sparse_hierarchy<T>(other: &T) -> Self
    where
        T: SparseHierarchy<
            LevelCount = Self::LevelCount,
            LevelMask  = Self::LevelMask,
        >,
        for<'a> T: SparseHierarchyTypes<'a,
            DataType = <Self as SparseHierarchyTypes<'a>>::DataType
        >;
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
    ) -> <Self::This as SparseHierarchy>::LevelMask;
    
    /// Pointed node must exists
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self,
        this: &'a Self::This,
        level_n: N, 
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask;
    
    /// Item at index may not exist.
    unsafe fn data<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> Option<<Self::This as SparseHierarchyTypes<'a>>::Data>;      
 
    /// Pointed data must exists
    unsafe fn data_unchecked<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> <Self::This as SparseHierarchyTypes<'a>>::Data;        
}