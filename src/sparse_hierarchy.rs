use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{BitBlock, IntoOwned, PrimitiveArray};
use crate::sparse_array::level_indices;
//use crate::array::level_indices_new;
use crate::bit_block::is_empty_bitblock;
use crate::const_int::ConstInteger;

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
    
    /// Hierarchy levels count (without a data level).
    type LevelCount: ConstInteger;
    
    type LevelMaskType: BitBlock;
    type LevelMask<'a>: Borrow<Self::LevelMaskType> + IntoOwned<Self::LevelMaskType>
        where Self: 'a;
    
    /*/// `I::CAP` - level number. Starting from 0.
    fn level_mask<I: PrimitiveArray<Item=usize>>(&self, level_indices: I) -> Self::LevelMask<'_>;*/
    
    // TODO: Try to remove IntoOwned here. This requires Data to impl Clone. 
    // We need to have Default here, because can't have it in PrimitiveArray,
    // because only max [T;32] implements Default. 
    /// Len/CAP = LEVELS_COUNT
    type DataBlockIndices : PrimitiveArray<Item = usize> + Default;
    type DataBlockType;
    type DataBlock<'a>: Borrow<Self::DataBlockType> + IntoOwned<Self::DataBlockType> 
        where Self: 'a;
    /// # Safety
    ///
    /// indices are not checked.
    unsafe fn data_block(&self, level_indices: Self::DataBlockIndices)
        -> Self::DataBlock<'_>;
    
    /*unsafe fn contains_unchecked(&self, index: usize) -> bool {
        let indices = level_indices_new::<Self::LevelMaskType, Self::DataBlockIndices>(index);
        // indices.split_last()
        
        self.level_mask(indices);
        todo!()
    }*/
    
    /// # Safety
    ///
    /// `index` must be in range.
    #[inline]
    unsafe fn get_unchecked(&self, index: usize) -> Self::DataBlock<'_>{
        let indices = level_indices::<Self::LevelMaskType, Self::DataBlockIndices>(index);
        self.data_block(indices)
    }
    
    /// Returns None if `index` outside range.
    #[inline]
    fn get(&self, index: usize) -> Option<Self::DataBlock<'_>>{
        if index > Self::max_range(){
            None
        } else {
            Some(unsafe{ self.get_unchecked(index) })
        }
    }    
    
    type State: SparseHierarchyState<This = Self>;
    
    #[inline]
    /*const*/ fn max_range() -> usize {
        Self::LevelMaskType::size().pow(Self::LevelCount::VALUE as _) 
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
    
    /// Returns (level_mask, is_not_empty).
    /// 
    /// `is_not_empty` - mask not empty flag. Allowed to be false-positive.
    /// 
    /// # Safety
    /// 
    /// level index is not checked 
    unsafe fn select_level_bock<'a, L: ConstInteger>(
        &mut self,
        level: L,   // TODO: find better name this is actually level hierarchy/depth index 
        this: &'a Self::This,
        level_index: usize
    ) -> (<Self::This as SparseHierarchy>::LevelMask<'a>, bool);        

    /// # Safety
    /// 
    /// level index is not checked
    unsafe fn data_block<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::DataBlock<'a>;    
}

/*
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
}*/