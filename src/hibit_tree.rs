use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::RangeTo;
use crate::{multi_map_fold, BitBlock};
use crate::const_utils::{ConstArray, ConstInteger};
use crate::iter::Iter;
use crate::level_indices;
use crate::ops::{Map, MapFunction, MultiMapFold};
use crate::utils::{BinaryFunction, Borrowable, NullaryFunction, Take};

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
for 
    usize
{
    #[inline]
    fn from(value: Index<LevelMaskType, LevelCount>) -> Self {
        value.0
    }
}

pub trait HibitTreeTypes<'this, ImplicitBounds = &'this Self>{
    type Data;
    type DataUnchecked;
    type Cursor: HibitTreeCursor<'this, Src=Self>;
}

/// Hierarchical bitmap tree interface.
/// 
/// HibitTree is a base trait for [RegularHibitTree] and [MultiHibitTree],
/// which you will use most of the time. Only multi_* operations over non-[RegularHibitTree]'ies
/// return bare `HibitTree`.
///
/// This split is needed, because multi_* operations ([MultiHibitTree]'ies) 
/// return Iterators, that produce values on the fly. [data()], [data_unchecked()] 
/// and [iter()] - all get source data from different functions, and also
/// process it in different ways. Alternative to this would be collect all items
/// into intermediate container, and then return it to the user. That is what
/// [multi_map_fold] do - aggregates iterator into one value, and thus makes 
/// [MultiHibitTree] Regular again(!).
///
/// # HibitTreeTypes
/// 
/// HibitTree "inherits" HibitTreeTypes with lifetime argument. 
/// This is workaround for Rust's basically non-usable GATs[^gat_problems].
/// 
/// If you need concrete types - use [HibitTreeData] and 
/// [MultiHibitTreeIterItem] helpers. Or get them from [HibitTreeTypes],
/// as if it were super-trait:
/// ```
/// let i: <MySparseContainer as HibitTreeTypes>::Data = my_sparse_container.get(1).unwrap();
/// ```
/// ```
/// type MyData<'i> = <MySparseContainer as HibitTreeTypes<'i>>::Data; 
/// ```
/// 
/// Same technique used for other HibitTree related traits.
/// 
/// [^gat_problems]: With GAT's we always end up with this <https://blog.rust-lang.org/2022/10/28/gats-stabilization.html#implied-static-requirement-from-higher-ranked-trait-bounds>
/// error.
/// We use this technique <https://sabrinajewson.org/blog/the-better-alternative-to-lifetime-gats#the-better-gats> 
/// as a workaround.
pub trait HibitTree: Sized + Borrowable<Borrowed=Self>
where
	Self: for<'this> HibitTreeTypes<'this>,
{
    /// TODO: Decription form hi_sparse_bitset TRUSTED_HIERARCHY
    const EXACT_HIERARCHY: bool;
    
    /// Hierarchy levels count (without a data level).
    type LevelCount: ConstInteger;
    type LevelMask : BitBlock;
 
    /// # Safety
    /// 
    /// Element may not exist, but `index` must be in range, and `level_indices` must
    /// correspond to `index`.
    /// 
    /// `level_indices` must be [LevelCount] size[^1].
    /// 
    /// [^1]: It is not just `[usize; LevelCount::VALUE]` due to troublesome 
    ///       Rust const expressions in generic context. 
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as HibitTreeTypes<'_>>::Data>;
 
    /// # Safety
    /// 
    /// pointed element must exist, and `level_indices` must
    /// corresponds to `index`.
    /// 
    /// `level_indices` must be [LevelCount] size[^1].
    /// 
    /// [^1]: It is not just `[usize; LevelCount::VALUE]` due to troublesome 
    ///       Rust const expressions in generic context. 
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked;
    
    #[inline]
    fn iter(&self) -> Iter<Self>{
        Iter::new(self)
    }

    /// You can use `usize` or [Index] for `index`.
    #[inline]
    fn get(&self, index: impl Into<Index<<Self as HibitTree>::LevelMask, Self::LevelCount>>) 
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
    {
        let index: usize = index.into().into();
        let indices = level_indices::<Self::LevelMask, Self::LevelCount>(index);
        unsafe{ self.data(index, indices.as_ref()) }
    }

    /// # Safety
    ///
    /// Item at `index` must exist.
    #[inline]
    unsafe fn get_unchecked(&self, index: usize) 
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked 
    {
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


/// [HibitTree] that is not a concrete collection.
/// 
/// Most results of operations are.
pub trait LazyHibitTree: HibitTree {
    /// Make a concrete collection from a lazy/virtual one.
    #[inline]
    fn materialize<T>(self) -> T
    where
        T: FromHibitTree<Self>
    {
        T::from_sparse_hierarchy(self)
    }
}

/// Construct a [HibitTree] collection from any [HibitTree].
pub trait FromHibitTree<From: HibitTree> {
    fn from_sparse_hierarchy(from: From) -> Self;
}

pub trait HibitTreeCursorTypes<'this, ImplicitBounds = &'this Self>{
    type Data;
    // Looks like we don't need DataUnchecked in State yet.
    // (unchecked versions return Data)
}

/// Stateful [HibitTree] traverse interface.
/// 
/// Having state allows implementations to have cache level meta-info.
/// If level block changed seldom and not sporadically (like during iteration) -
/// this can get a significant performance boost, especially in generative [HibitTree]'ies.
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
pub trait HibitTreeCursor<'src>
where
	Self: for<'this> HibitTreeCursorTypes<'this>,
{
    type Src: HibitTree;
    
    fn new(src: &'src Self::Src) -> Self;
    
    /// Item at index may not exist. Will return empty mask in such case.
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self,
        src: &'src Self::Src,
        level_n: N,
        level_index: usize,
    ) -> <Self::Src as HibitTree>::LevelMask;
    
    /// Pointed node must exists
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self,
        src: &'src Self::Src,
        level_n: N,
        level_index: usize
    ) -> <Self::Src as HibitTree>::LevelMask;
    
    /// Item at index may not exist.
    unsafe fn data<'a>(
        &'a self,
        src: &'src Self::Src,
        level_index: usize
    ) -> Option<<Self as HibitTreeCursorTypes<'a>>::Data>;      
 
    /// Pointed data must exists
    unsafe fn data_unchecked<'a>(
        &'a self,
        src: &'src Self::Src,
        level_index: usize
    ) -> <Self as HibitTreeCursorTypes<'a>>::Data;        
}

pub type HibitTreeData<'a, T> = <T as HibitTreeTypes<'a>>::Data;
pub type MultiHibitTreeIterItem<'a, T> = <T as MultiHibitTreeTypes<'a>>::IterItem;

pub trait RegularHibitTreeTypes<'this, ImplicitBounds = &'this Self>
    : HibitTreeTypes<'this, ImplicitBounds,
        DataUnchecked = <Self as HibitTreeTypes<'this, ImplicitBounds>>::Data, 
        Cursor: for<'a> HibitTreeCursorTypes<'a, 
            Data = Self::Data
        >,
    >
{}

// TODO: better naming?

/// [HibitTree] where all operations return same type - [Self::Data].
/// 
/// Think of it as of "the usual" [HibitTree].  
/// 
/// All containers and all "non-multi" operations results are MonoSparseHierarchy. 
pub trait RegularHibitTree: HibitTree
where
    Self: for<'this> RegularHibitTreeTypes<'this>
{
    /// See [crate::map]
    #[inline]
    fn map<F>(self, f: F) -> Map<Self, F>
    where
        F: for<'a> MapFunction<'a, <Self as HibitTreeTypes<'a>>::Data>
    {
        crate::map(self, f)
    }
    
    /// See [crate::map]
    #[inline]
    fn map_ref<F>(&self, f: F) -> Map<&Self, F>
    where
        F: for<'a> MapFunction<'a, <Self as HibitTreeTypes<'a>>::Data>
    {
        crate::map(self, f)
    }
}

impl<'this, T> RegularHibitTreeTypes<'this> for T
where
    T: HibitTreeTypes<'this,
        DataUnchecked = <Self as HibitTreeTypes<'this>>::Data,
        Cursor: for<'a> HibitTreeCursorTypes<'a, 
            Data = Self::Data
        >,
    >
{} 

// TODO: impl manually?
impl<T> RegularHibitTree for T
where
    T: HibitTree,
    T: for<'this> RegularHibitTreeTypes<'this>
{}

pub trait MultiHibitTreeTypes<'this, ImplicitBounds = &'this Self>
    : HibitTreeTypes<'this, ImplicitBounds, 
        Data: Iterator<Item=Self::IterItem>,
        DataUnchecked: Iterator<Item=Self::IterItem>,
        Cursor: for<'a> HibitTreeCursorTypes<'a, 
            Data: Iterator<Item=Self::IterItem>
        >,
    >
{
    type IterItem;
}

/// [HibitTree], that returns `impl Iterator<Self::IterItem>`
/// for all operations.
/// 
/// `multi_*` operations return [MultiHibitTree]'ies.
/// 
/// You can convert `MultiHibitTree` to [RegularHibitTree],
/// with [multi_map_fold()]. 
pub trait MultiHibitTree: HibitTree
where
    Self: for<'this> MultiHibitTreeTypes<'this>
{
    /// See [crate::multi_map_fold].
    #[inline]
    fn map_fold<I, F>(self, init: I, f: F) -> MultiMapFold<Self, I, F>
    where 
        I: NullaryFunction,
        F: for<'a> BinaryFunction<
            I::Output, 
            <Self as MultiHibitTreeTypes<'a>>::IterItem,
            Output = I::Output
        >,    
    {
        multi_map_fold(self, init, f)
    }
}