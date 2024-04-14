use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;
use arrayvec::ArrayVec;
use crate::{BitBlock, IntoOwned};
use crate::sparse_hierarchy::{DefaultState, level_bypass, LevelBypass, SparseHierarchy};

// TODO: We can go without ArrayIter being Clone!

pub struct Fold<'a, Op, Init, ArrayIter, Array>{
    pub(crate) op: Op,
    pub(crate) init: &'a Init,
    pub(crate) array_iter: ArrayIter,
    pub(crate) phantom: PhantomData<&'a Array>,
}

impl<'a, Op, Init, ArrayIter, Array> SparseHierarchy for Fold<'a, Op, Init, ArrayIter, Array>
where
    Init: SparseHierarchy<
        Level0MaskType = Array::Level0MaskType,
        Level1MaskType = Array::Level1MaskType,
        Level2MaskType = Array::Level2MaskType,
        DataBlockType  = Array::DataBlockType,
    >,

    ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: SparseHierarchy,

    Op: crate::apply::Op<
        Level0Mask = Array::Level0MaskType,
        Level1Mask = Array::Level1MaskType,
        Level2Mask = Array::Level2MaskType,
        DataBlock  = Array::DataBlockType,
    >,
{
    const EXACT_HIERARCHY: bool = true;
    
    type Level0MaskType = Array::Level0MaskType;
    type Level0Mask<'b> = Self::Level0MaskType where Self: 'b;

    #[inline]
    fn level0_mask(&self) -> Self::Level0Mask<'_> {
        self.array_iter.clone().fold(
            self.init.level0_mask().into_owned(), 
            |acc, array|{
                self.op.lvl0_op(acc, array.level0_mask())
            }
        )
    }

    type Level1MaskType = Op::Level1Mask;
    type Level1Mask<'b> where Self: 'b = Op::Level1Mask;

    #[inline]
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask<'_> {
        self.array_iter.clone().fold(
            self.init.level1_mask(level0_index).into_owned(), 
            |acc, array|{
                self.op.lvl1_op(acc, array.level1_mask(level0_index))
            }
        )
    }

    type Level2MaskType = Op::Level2Mask;
    type Level2Mask<'b> where Self: 'b = Op::Level2Mask;

    #[inline]
    unsafe fn level2_mask(&self, level0_index: usize, level1_index: usize) 
        -> Self::Level2Mask<'_> 
    {
        self.array_iter.clone().fold(
            self.init.level2_mask(level0_index, level1_index).into_owned(), 
            |acc, array|{
                self.op.lvl2_op(acc, array.level2_mask(level0_index, level1_index))
            }
        )
    }

    type DataBlockType =  Op::DataBlock;
    type DataBlock<'b> where Self: 'b = Op::DataBlock;

    #[inline]
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize, level2_index: usize) 
        -> Self::DataBlock<'_> 
    {
        self.array_iter.clone().fold(
            self.init.data_block(level0_index, level1_index, level2_index).into_owned(), 
            |acc, array|{
                self.op.data_op(acc, array.data_block(level0_index, level1_index, level2_index))
            }
        )
    }
    
    type State = DefaultState<Self>;
}

const N: usize = 32;

/*pub struct ReduceIterState<'a, Init, Array>
where
    Init: LevelMasksIter,
    Array: LevelMasksIter,
{
    init_state: Init::IterState,
    states: ArrayVec<(&'a Array, Array::IterState), N>,
    
    // TODO: ZST when not in use 
    /// In-use only when `Op::SKIP_EMPTY_HIERARCHIES` raised.
    lvl1_non_empty_states: ArrayVec<usize, N>,
    lvl2_non_empty_states: ArrayVec<usize, N>,
}

impl<'a, Op, Init, ArrayIter, Array> LevelMasksIter for Fold<'a, Op, Init, ArrayIter, Array>
where
    Init: LevelMasksIter<
        Level0MaskType = Array::Level0MaskType,
        Level1MaskType = Array::Level1MaskType,
        Level2MaskType = Array::Level2MaskType,
        DataBlockType  = Array::DataBlockType,
    >,
    ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: LevelMasksIter,

    Op: crate::apply::Op<
        Level0Mask = Array::Level0MaskType,
        Level1Mask = Array::Level1MaskType,
        Level2Mask = Array::Level2MaskType,
        DataBlock  = Array::DataBlockType,
    >,
{
    type IterState = ReduceIterState<'a, Init, Array>;
    
    #[inline]
    fn make_state(&self) -> Self::IterState{
        let mut states = ArrayVec::new();
        for array in self.array_iter.clone(){
            unsafe{ 
                states.push_unchecked((array, array.make_state())); 
            }
        }
        
        ReduceIterState{
            init_state: self.init.make_state(),
            states,
            lvl1_non_empty_states: Default::default(),
            lvl2_non_empty_states: Default::default(),
        }
    }
    
    #[inline]
    unsafe fn init_level1_block_meta(&self, state: &mut Self::IterState, level0_index: usize) -> (Self::Level1Mask<'_>, bool) {
        let (acc_mask, _) = self.init.init_level1_block_meta(&mut state.init_state, level0_index);
        let mut acc_mask = acc_mask.into_owned();
        
        if Op::SKIP_EMPTY_HIERARCHIES{
            state.lvl1_non_empty_states.clear();
            for i in 0..state.states.len(){
                let (array, array_state) = state.states.get_unchecked_mut(i);
                let (mask, _) = array.init_level1_block_meta(array_state, level0_index);
                acc_mask = self.op.lvl1_op(acc_mask, mask);
                
                state.lvl1_non_empty_states.push_unchecked(i);
            }
        } else {
            for (array, array_state) in state.states.iter_mut(){
                let (mask, v) = array.init_level1_block_meta(array_state, level0_index);
                acc_mask = self.op.lvl1_op(acc_mask, mask);
            }            
        }
        
        let is_empty = acc_mask.is_zero(); 
        (acc_mask, !is_empty)
    }

    #[inline]
    unsafe fn init_level2_block_meta(&self, state: &mut Self::IterState, level1_index: usize) -> (Self::Level2Mask<'_>, bool) {
        let (acc_mask, _) = self.init.init_level2_block_meta(&mut state.init_state, level1_index);
        let mut acc_mask = acc_mask.into_owned();
        
        if Op::SKIP_EMPTY_HIERARCHIES{
            state.lvl2_non_empty_states.clear();
            for &i in &state.lvl1_non_empty_states{
                let (array, array_state) = state.states.get_unchecked_mut(i);
                let (mask, v) = array.init_level2_block_meta(array_state, level1_index);
                acc_mask = self.op.lvl2_op(acc_mask, mask);
                
                if v{
                    state.lvl2_non_empty_states.push_unchecked(i);
                }
            }
        } else {
            for (array, array_state) in state.states.iter_mut(){
                let (mask, _) = array.init_level2_block_meta(array_state, level1_index);
                acc_mask = self.op.lvl2_op(acc_mask, mask);
            }
        }
        
        let is_empty = acc_mask.is_zero(); 
        (acc_mask, !is_empty)
    }
    
    #[inline]
    unsafe fn data_block_from_meta(&self, state: &Self::IterState, level_index: usize) -> Self::DataBlock<'_> {
        let mut acc = self.init.data_block_from_meta(&state.init_state, level_index).into_owned();
        
        if Op::SKIP_EMPTY_HIERARCHIES
        && level_bypass::<Self>() != LevelBypass::Level1Level2
        {
            let state_indices =
                if LevelBypass::Level2 == level_bypass::<Self>(){
                    state.lvl1_non_empty_states.iter()
                } else {
                    debug_assert!(LevelBypass::None == level_bypass::<Self>());
                    state.lvl2_non_empty_states.iter()
                };
            
            for &i in state_indices {
                let (array, array_state) = state.states.get_unchecked(i);
                let data = array.data_block_from_meta(array_state, level_index);
                acc = self.op.data_op(acc, data);
            }
        } else {
            for (array, state) in &state.states {
                let data = array.data_block_from_meta(state, level_index);
                acc = self.op.data_op(acc, data);
            }
        }
        
        acc        
    }    
}*/