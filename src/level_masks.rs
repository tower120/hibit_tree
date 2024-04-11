use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use crate::{BitBlock, IntoOwned};
use crate::bit_block::is_empty_bitblock;
use crate::level_block::LevelBlock;

/// Basic interface for accessing level_block masks. Can work with `SimpleIter`.
///
/// # Level bypass
/// 
/// TODO
/// 
// We need xxxxType for each concrete level_block/mask type to avoid the need for use `for<'a>`,
// which is still not working (at Rust level) in cases, where we need it most. 
pub trait LevelMasks{
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
    
    // TODO: remove LevelBlock bound
    type DataBlockType: LevelBlock;
    type DataBlock<'a>: Borrow<Self::DataBlockType> + IntoOwned<Self::DataBlockType> 
        where Self: 'a;
    /// # Safety
    ///
    /// indices are not checked
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize, level2_index: usize)
        -> Self::DataBlock<'_>;
}

#[derive(Eq, PartialEq)]
pub enum LevelBypass {
    None,
    Level2,
    Level1Level2
}

// TODO: move to LevelMasks
/// This acts as `const`.
pub /*const*/ fn level_bypass<T: LevelMasks>() -> LevelBypass {
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

/// More sophisticated masks interface, optimized for iteration speed of 
/// generative/lazy bitset.
/// 
/// For example, in [Reduce] this achieved through
/// caching level1 level_block pointers of all sets. Which also allows to discard
/// bitsets with empty level1 blocks in final stage of getting data blocks.
/// Properly implementing this gave [Reduce] and [Apply] 25-100% performance boost.  
///
pub trait LevelMasksIter: LevelMasks{
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
}

// TODO: As long as iterator works with &LevelMasks - we can just
//       use Borrow<impl LevelMasks> everywhere

pub trait LevelMasksBorrow: Borrow<Self::Type>{
    type Type: LevelMasks;
}
impl<T: LevelMasks> LevelMasksBorrow for T{
    type Type = T;
}