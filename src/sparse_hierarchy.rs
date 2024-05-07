use std::borrow::Borrow;
use crate::{Array, BitBlock};
use crate::sparse_array::level_indices;
use crate::const_utils::const_int::ConstInteger;
use crate::const_utils::const_array::{ConstArray, ConstArrayType, ConstCopyArrayType};
use crate::level_block::MaybeEmpty;
use crate::utils::IntoOwned;

/// 
/// TODO: Change description
///
/// # Infinity
/// 
/// SparseHierarchy will act as infinite-size data source, if you try to
/// access elements outside of [max_range] in a safe manner ([get]/[contains]).   
/// 
// We need xxxxType for each concrete level_block/mask type to avoid the need for use `for<'a>`,
// which is still not working (at Rust level) in cases, where we need it most. 
pub trait SparseHierarchy {
    /// TODO: Decription form hi_sparse_bitset TRUSTED_HIERARCHY
    const EXACT_HIERARCHY: bool;
    
    /// Hierarchy levels count (without a data level).
    type LevelCount: ConstInteger;
    
    type LevelMaskType: BitBlock;
    type LevelMask<'a>: Borrow<Self::LevelMaskType> + IntoOwned<Self::LevelMaskType>
        where Self: 'a;
    
    /// Returns mask for level `I::CAP`. 
    /// 
    /// Each `level_indices` array elements corresponds to each level, skipping root.
    /// Root level skipped, for performance reasons, since root block is always one.
    /// 
    /// # Safety
    ///
    /// `level_indices` are not checked.
    unsafe fn level_mask<I>(&self, level_indices: I) -> Self::LevelMask<'_>
    where
        I: ConstArray<Item=usize> + Copy;
    
    // TODO: Try to remove IntoOwned here. This requires Data to impl Clone. 
    type DataType: MaybeEmpty;
    type Data<'a>: Borrow<Self::DataType> + IntoOwned<Self::DataType>
        where Self: 'a;
    /// # Safety
    ///
    /// indices are not checked.
    unsafe fn data_block<I>(&self, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy;
    
    // We need this, because Data may return reference.
    // And we can't have a non-const constructible static in rust,
    // for taking reference from arbitrary DataBlockType::empty() 
    // without overhead.
    //
    // Used by get().
    fn empty_data(&self) -> Self::Data<'_>;
    
    /// Same as [may_contain], but without range checks.
    /// 
    /// # Safety
    ///
    /// `index` must be in [max_range].
    #[inline]
    unsafe fn may_contain_unchecked(&self, index: usize) -> bool {
        let indices = level_indices::<Self::LevelMaskType, Self::LevelCount>(index);
        let (level_indices, mask_index) = indices.split_last();
        let mask = self.level_mask(level_indices);
        mask.borrow().get_bit(mask_index)
    }
    
    /// Returns true if element at `index` is non-empty.
    /// 
    /// Faster than [get] + [is_empty], since output is based on hierarchy data only.
    /// May return false positives with non-[EXACT_HIERARCHY].
    /// 
    /// Returns false if `index` outside of range.
    /// 
    /// This makes SparseHierarchy basically infinite.
    #[inline]
    fn may_contain(&self, index: usize) -> bool {
        if index > Self::max_range(){
            false
        } else {
            unsafe{ self.may_contain_unchecked(index) }
        }        
    }
    
    /// Same as [contains], but without range checks.
    ///
    /// # Safety
    ///
    /// `index` must be in [max_range].
    #[inline]
    unsafe fn contains_unchecked(&self, index: usize) -> bool {
        if Self::EXACT_HIERARCHY {
            self.may_contain_unchecked(index)
        } else {
            self.get_unchecked(index).borrow().is_empty()
        }
    }
    
    /// Returns true if element at `index` is non-empty.
    /// 
    /// If [EXACT_HIERARCHY] - faster than [get] + [is_empty].
    /// Otherwise - just do the job.
    /// 
    /// Returns false if `index` outside of range.
    #[inline]
    fn contains(&self, index: usize) -> bool {
        if index > Self::max_range() {
            false
        } else {
            unsafe{ self.contains_unchecked(index) }
        }
    }
    
    /// # Safety
    ///
    /// `index` must be in [max_range].
    #[inline]
    unsafe fn get_unchecked(&self, index: usize) -> Self::Data<'_> {
        let indices = level_indices::<Self::LevelMaskType, Self::LevelCount>(index);
        self.data_block(indices)
    }
    
    /// Returns [empty_data_block()] if `index` outside of range.
    /// 
    /// This makes SparseHierarchy basically infinite.
    #[inline]
    fn get(&self, index: usize) -> Self::Data<'_>{
        if index > Self::max_range(){
            self.empty_data()
        } else {
            unsafe{ self.get_unchecked(index) }
        }
    }    
    
    /// Use [DefaultState] as default, if you don't want to implement 
    /// stateful SparseHierarchy.
    type State: SparseHierarchyState<This = Self>;
    
    /// Max index this SparseHierarchy can contain.
    /// 
    /// Act as `const` - noop.
    #[inline]
    /*const*/ fn max_range() -> usize {
        Self::LevelMaskType::size().pow(Self::LevelCount::VALUE as _) - 1 
    }
}

/// Stateful [SparseHierarchy] interface.
/// 
/// Having state allows implementations to have cache level meta-info.
/// If level block changed seldom and not sporadically (like during iteration) -
/// this can get a significant performance boost, especially in generative [SparseHierarchy]'ies.
/// 
/// Block levels must be selected top(0) to bottom(last N) level.
/// When you [select_level_bock], all levels below considered **not** selected.
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
/// let (mask0, _) = state.select_level_bock(array, ConstInt::<0>, 0);
/// // Select 4th level1 block (corresponds to array indices [192..256))
/// let (mask1, _) = state.select_level_bock(array, ConstInt::<1>, 3);
/// // Select 9th data block (array index 201)
/// let data = state.data_block(array, 9);
/// ``` 
/// 
/// # is_not_empty
/// 
/// [select_level_bock] returns is_not_empty flag, because sometimes you may have
/// faster ways of checking emptiness, then checking simd register (bitblock) for
/// zero, in general case.
/// For example, in [SparseArray] - it is done by checking of level_block 
/// indirection index for zero.
/// 
/// [Fold] with [SKIP_EMPTY_HIERARCHIES] rely heavily on that optimization.
pub trait SparseHierarchyState{
    type This: SparseHierarchy;
    
    fn new(this: &Self::This) -> Self;
    
    /// Select block at `level_n` with `level_index`. Where `level_index` is index
    /// in block pointing to `level_n` (which was previously selected). 
    /// 
    /// Returns `(level_mask, is_not_empty)`.
    /// `is_not_empty` - mask not empty flag. Allowed to be false-positive.
    /// 
    /// All levels below, considered **not** selected.
    /// 
    /// # Safety
    /// 
    /// - `level_index` is not checked.
    /// - All previous levels must be selected. 
    unsafe fn select_level_bock<'a, N: ConstInteger>(
        &mut self,
        this: &'a Self::This,
        level_n: N, 
        level_index: usize
    ) -> (<Self::This as SparseHierarchy>::LevelMask<'a>, bool);        
    
    /// # Safety
    /// 
    /// - `level_index` is not checked.
    /// - All hierarchy levels must be selected.
    unsafe fn data_block<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::Data<'a>;    
}

/// Redirect to [SparseHierarchy] stateless methods.
pub struct DefaultState<This>
where
    This: SparseHierarchy
{
    /// [usize; This::LevelCount - 1]
    level_indices: ConstArrayType<
        usize,
        <This::LevelCount as ConstInteger>::Dec   
    >
}

impl<This: SparseHierarchy> SparseHierarchyState for DefaultState<This>{
    type This = This;

    #[inline]
    fn new(_: &Self::This) -> Self {
        Self{
            level_indices: Array::from_fn(|_| 0)
        }
    }

    #[inline]
    unsafe fn select_level_bock<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> (<Self::This as SparseHierarchy>::LevelMask<'a>, bool) {
        if /*const*/ level_n.value() == 0 {
            debug_assert!(level_index == 0);
        } else {
            self.level_indices.as_mut()[level_n.dec().value()] = level_index;
        }
        
        let indices: ConstCopyArrayType<usize, N> 
            = Array::from_fn(|/*const*/ i| {
                if /*const*/ N::VALUE-1 == i {
                    level_index
                } else {
                    self.level_indices.as_ref()[i]    
                }
            });
        let mask     = this.level_mask(indices);
        let is_empty = mask.borrow().is_zero(); 
        (mask, is_empty)
    }

    #[inline]
    unsafe fn data_block<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::Data<'a> 
    {
        let indices: ConstCopyArrayType<usize, This::LevelCount> 
            = Array::from_fn(|/*const*/ i| {
                if /*const*/ This::LevelCount::VALUE-1 == i {
                    level_index
                } else {
                    self.level_indices.as_ref()[i]    
                }
            });
        this.data_block(indices)
    }
}
