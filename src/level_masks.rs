use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use crate::{BitBlock, IntoOwned};
use crate::bit_block::is_empty_bitblock;
use crate::level_block::LevelBlock;

/// 
/// TODO: Change description
/// 
/// Basic interface for accessing level_block masks. Can work with `SimpleIter`.
///
/// # Level bypass
/// 
/// TODO
/// 
// We need xxxxType for each concrete level_block/mask type to avoid the need for use `for<'a>`,
// which is still not working (at Rust level) in cases, where we need it most. 
pub trait SparseHierarchy {
    const EXACT_HIERARCHY: bool; 
    
    type Level0MaskType: BitBlock;
    type Level0Mask<'a>: Borrow<Self::Level0MaskType> + IntoOwned<Self::Level0MaskType>
        where Self: 'a;
    fn level0_mask(&self) -> Self::Level0Mask<'_>;

    type Level1MaskType: BitBlock;
    type Level1Mask<'a>: Borrow<Self::Level1MaskType> + IntoOwned<Self::Level1MaskType>
        where Self: 'a;
    /// # Safety
    ///
    /// index is not checked
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask<'_>;
    
    type Level2MaskType: BitBlock;
    type Level2Mask<'a>: Borrow<Self::Level2MaskType> + IntoOwned<Self::Level2MaskType>
        where Self: 'a;
    /// # Safety
    ///
    /// index is not checked
    unsafe fn level2_mask(&self, level0_index: usize, level1_index: usize) -> Self::Level2Mask<'_>;
    
    type DataBlockType;
    type DataBlock<'a>: Borrow<Self::DataBlockType> + IntoOwned<Self::DataBlockType> 
        where Self: 'a;
    /// # Safety
    ///
    /// indices are not checked
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize, level2_index: usize)
        -> Self::DataBlock<'_>;
    
    type State: SparseHierarchyState<This = Self>;
    
    #[inline]
    /*const*/ fn max_range() -> usize {
        Self::Level0MaskType::size() 
        * Self::Level1MaskType::size() 
        * Self::Level2MaskType::size()
    }
}

/// Stateful [SparseHierarchy] interface.
/// 
/// Having state allows implementations to cache level meta-info.
/// If level block changed seldom and not sporadically (like during iteration) -
/// this can get a significant performance boost, especially in generative [SparseHierarchy]'ies.
/// 
/// # is_not_empty
/// 
/// select_levelN returns is_not_empty flag, because sometimes you may have
/// faster ways of checking emptiness, then checking simd register (bitblock) for
/// zero, in general case.
/// For example, in [SparseArray] - it is done by checking of level_block indirection index for zero.
/// 
/// [Fold] with [SKIP_EMPTY_HIERARCHIES] rely heavily on that optimization.
pub trait SparseHierarchyState{
    type This: SparseHierarchy;
    
    fn new(this: &Self::This) -> Self;

    // TODO: select_level1_block
    /// Returns (level_mask, is_not_empty).
    /// 
    /// `is_not_empty` - mask not empty flag. Allowed to be false-positive.
    /// 
    /// # Safety
    /// 
    /// level index is not checked 
    unsafe fn select_level1<'a>(
        &mut self,
        this: &'a Self::This,
        level0_index: usize
    ) -> (<Self::This as SparseHierarchy>::Level1Mask<'a>, bool);        

    /// Returns (level_mask, is_not_empty).
    /// 
    /// `is_not_empty` - mask not empty flag. Allowed to be false-positive.
    /// 
    /// # Safety
    /// 
    /// level index is not checked
    unsafe fn select_level2<'a>(
        &mut self,
        this: &'a Self::This,
        level1_index: usize
    ) -> (<Self::This as SparseHierarchy>::Level2Mask<'a>, bool);               

    /// # Safety
    /// 
    /// level index is not checked
    unsafe fn data_block<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::DataBlock<'a>;    
}

/// Redirect to [SparseHierarchy] stateless methods.
pub struct DefaultState<This>{
    level0_index: usize,
    level1_index: usize,
    phantom_data: PhantomData<This>
}
impl<This: SparseHierarchy> SparseHierarchyState for DefaultState<This>{
    type This = This;

    #[inline]
    fn new(_: &Self::This) -> Self {
        Self{
            level0_index: 0,
            level1_index: 0,
            phantom_data: Default::default(),
        }
    }

    #[inline]
    unsafe fn select_level1<'a>(&mut self, this: &'a Self::This, level0_index: usize) 
        -> (<Self::This as SparseHierarchy>::Level1Mask<'a>, bool) 
    {
        self.level0_index = level0_index;
        let mask = this.level1_mask(level0_index);
        let is_empty = mask.borrow().is_zero(); 
        (mask, !is_empty)
    }

    #[inline]
    unsafe fn select_level2<'a>(&mut self, this: &'a Self::This, level1_index: usize) 
        -> (<Self::This as SparseHierarchy>::Level2Mask<'a>, bool) 
    {
        self.level1_index = level1_index;
        let mask = this.level2_mask(self.level0_index, level1_index);
        let is_empty = mask.borrow().is_zero(); 
        (mask, !is_empty)
    }

    #[inline]
    unsafe fn data_block<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::DataBlock<'a> 
    {
        match level_bypass::<Self::This>(){
            LevelBypass::None => this.data_block(self.level0_index, self.level1_index, level_index),
            LevelBypass::Level2 => this.data_block(self.level0_index, level_index, 0),
            LevelBypass::Level1Level2 => this.data_block(level_index, 0, 0),
        } 
    }
} 

#[derive(Eq, PartialEq)]
pub enum LevelBypass {
    /// All 4 levels used. 
    None,
    
    /// Level2 skipped. Only 3 levels used: Level0, Level1 and DataLevel.
    Level2,
    
    /// Level1-Level2 skipped. Only 2 levels used: Level0 and DataLevel.
    Level1Level2
}

// TODO: move to LevelMasks?
/// This acts as `const`.
pub /*const*/ fn level_bypass<T: SparseHierarchy>() -> LevelBypass {
    let lvl1 = is_empty_bitblock::<T::Level1MaskType>();
    let lvl2 = is_empty_bitblock::<T::Level2MaskType>();
    
    if lvl1{
        assert!(lvl2);
        LevelBypass::Level1Level2
    } else if lvl2 {
        LevelBypass::Level2
    } else {
        LevelBypass::None
    }
}

/*/// More sophisticated masks interface, optimized for iteration speed of 
/// generative/lazy bitset.
/// 
/// For example, in [Reduce] this achieved through
/// caching level1 level_block pointers of all sets. Which also allows to discard
/// bitsets with empty level1 blocks in final stage of getting data blocks.
/// Properly implementing this gave [Reduce] and [Apply] 25-100% performance boost.  
///
pub trait LevelMasksIter: SparseHierarchy {
    /// Constructed at the start of iteration with [make_state], dropped at the end.
    type IterState;
    
    fn make_state(&self) -> Self::IterState;
    
    // TODO: rename and adjust doc
    /// Init `level1_block_data` and return (Level1Mask, is_not_empty).
    /// 
    /// Called by iterator for each traversed level1 level_block.
    /// 
    /// - `level1_block_data` will come in undefined state - rewrite it completely.
    /// - `is_not_empty` is not used by iterator itself, but can be used by other 
    /// generative bitsets (namely [Reduce]). We expect compiler to optimize away non-used code.
    /// It exists - because sometimes you may have faster ways of checking emptiness,
    /// then checking simd register (bitblock) for zero in general case.
    /// For example, in BitSet - it is done by checking of level_block indirection index for zero.
    /// False positive is OK, though may incur unnecessary overhead.
    /// 
    /// # Safety
    ///
    /// indices are not checked.
    /// 
    /// [Reduce]: crate::Reduce
    // Performance-wise it is important to use this in-place construct style, 
    // instead of just returning Level1BlockData. Even if we return Level1BlockData,
    // and then immediately write it to MaybeUninit - compiler somehow still can't
    // optimize it as direct memory write without an intermediate bitwise copy.
    unsafe fn init_level1_block_meta(
        &self,
        state: &mut Self::IterState,
        level0_index: usize
    ) -> (Self::Level1Mask<'_>, bool);    
    
    
    // TODO: rename
    unsafe fn init_level2_block_meta(
        &self,
        state: &mut Self::IterState,
        level1_index: usize
    ) -> (Self::Level2Mask<'_>, bool);    

    // TODO: rename
    /// Called by iterator for each traversed data level_block.
    /// 
    /// `level_index` depending on LevelBypass could be either level0, level1 or level2 in-block index.
    /// 
    /// # Safety
    ///
    /// indices are not checked.
    unsafe fn data_block_from_meta(
        &self,
        state: &Self::IterState,
        level_index: usize
    ) -> Self::DataBlock<'_>;
}*/

// TODO: As long as iterator works with &LevelMasks - we can just
//       use Borrow<impl LevelMasks> everywhere

pub trait LevelMasksBorrow: Borrow<Self::Type>{
    type Type: SparseHierarchy;
}
impl<T: SparseHierarchy> LevelMasksBorrow for T{
    type Type = T;
}