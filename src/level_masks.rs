use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use crate::BitBlock;

/// Basic interface for accessing block masks. Can work with `SimpleIter`.
pub trait LevelMasks{
    type Level0Mask: BitBlock;
    fn level0_mask(&self) -> Self::Level0Mask;

    type Level1Mask: BitBlock;
    /// # Safety
    ///
    /// index is not checked
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask;
    
    type DataBlock;
    /// # Safety
    ///
    /// indices are not checked
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize)
        -> Self::DataBlock;
}

/// Iterator state for LevelMasksIter.
/// 
/// Constructed at the start of iteration, dropped at the end.
/// You'll need it, if you'd want to use heavy-to-construct/heavy-to-drop
/// data in Level1BlockInfo, like Vec. Since Level1BlockInfo dropped on each
/// block - you'd want to create Vec in state, and then use pointer, which points
/// to state Vec.  
/// 
/// Use `NoState<Self>` for stateless.
pub trait LevelMasksIterState {
    type Container: LevelMasksIter;
    
    fn make(container: &Self::Container) -> Self;
    
    /// Having separate function for drop not strictly necessary, since
    /// IterState can actually drop itself. But! This allows not to store cache
    /// size within IterState. Which makes FixedCache CacheData ZST, if its childs
    /// are ZSTs, and which makes cache construction and destruction noop. Which is
    /// important for short iteration sessions.
    /// 
    /// P.S. This can be done at compile-time by opting out "len" counter,
    /// but stable Rust does not allow to do that yet.
    /// 
    /// # Safety
    /// 
    /// - `state` must not be used after this.
    /// - Must be called exactly once for each `state`.    
    fn drop(container: &Self::Container, this: &mut ManuallyDrop<Self>);
}

pub struct NoState<C: LevelMasksIter>(PhantomData<C>);
impl<C: LevelMasksIter> LevelMasksIterState for NoState<C> {
    type Container = C;

    #[inline]
    fn make(_: &Self::Container) -> Self {Self(PhantomData)}

    #[inline]
    fn drop(_: &Self::Container, _: &mut ManuallyDrop<Self>) {}
}


/// More sophisticated masks interface, optimized for iteration speed of 
/// generative/lazy bitset.
/// 
/// For example, in [Reduce] this achieved through
/// caching level1 block pointers of all sets. Which also allows to discard
/// bitsets with empty level1 blocks in final stage of getting data blocks.
/// Properly implementing this gave [Reduce] and [Apply] 25-100% performance boost.  
///
pub trait LevelMasksIter: LevelMasks{
    type IterState: LevelMasksIterState<Container = Self>;
    
    /// Level1 block related data, used to speed up data block access.
    ///
    /// Prefer POD, or any kind of drop-less, since it will be dropped
    /// before the iteration of each next level1 block.
    /// 
    /// In library, used to cache Level1Block pointers for faster DataBlock access,
    /// without traversing whole hierarchy for getting each block during iteration.
    type Level1BlockInfo: Default;
    
    
    /// Init `level1_block_data` and return (Level1Mask, is_not_empty).
    /// 
    /// Called by iterator for each traversed level1 block.
    /// 
    /// - `level1_block_data` will come in undefined state - rewrite it completely.
    /// - `is_not_empty` is not used by iterator itself, but can be used by other 
    /// generative bitsets (namely [Reduce]). We expect compiler to optimize away non-used code.
    /// It exists - because sometimes you may have faster ways of checking emptiness,
    /// then checking simd register (bitblock) for zero in general case.
    /// For example, in BitSet - it is done by checking of block indirection index for zero.
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
    unsafe fn init_level1_block_info(
        &self,
        state: &mut Self::IterState,
        level1_block_data: &mut MaybeUninit<Self::Level1BlockInfo>,
        level0_index: usize
    ) -> (Self::Level1Mask, bool);    

    /// Called by iterator for each traversed data block.
    /// 
    /// # Safety
    ///
    /// indices are not checked.
    unsafe fn data_block_from_info(
        level1_block_data: &Self::Level1BlockInfo, level1_index: usize
    ) -> Self::DataBlock;
}