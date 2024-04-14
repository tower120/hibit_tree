use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;
use arrayvec::ArrayVec;
use crate::{BitBlock, IntoOwned};
use crate::level_block::LevelBlock;
use crate::level_masks::{level_bypass, LevelBypass, SparseHierarchy, LevelMasksBorrow, LevelMasksIter/*, LevelMasksIterState*/};

pub struct Reduce<'a, Op, ArrayIter, Array>{
    pub(crate) op: Op,
    pub(crate) array_iter: ArrayIter,
    pub(crate) phantom: PhantomData<&'a Array>,
}

impl<'a, Op, ArrayIter, Array> Reduce<'a, Op, ArrayIter, Array>
where
    Array: LevelMasksIter,

    Op: crate::apply::Op<
        Level0Mask = Array::Level0MaskType,
        Level1Mask = Array::Level1MaskType,
        Level2Mask = Array::Level2MaskType,
        DataBlock  = Array::DataBlockType,
    >,
{
    // Should be inside data_block_from_meta. But not in current Rust.
    #[inline]
    unsafe fn do_data_block_from_meta<'b, States>(mut states: States, op: &Op, level_index: usize)
        -> Op::DataBlock 
    where
        States: Iterator<Item = (&'a Array, &'b Array::IterState)>,
        Array::IterState: 'b
    {
        let first = states.next().unwrap_unchecked();
        let mut acc = first.0.data_block_from_meta(&first.1, level_index).into_owned();
        
        for (array, state) in states{
            let data = array.data_block_from_meta(state, level_index);
            acc = op.data_op(acc, data);
        }
        acc
    }
}

impl<'a, Op, ArrayIter, Array> SparseHierarchy for Reduce<'a, Op, ArrayIter, Array>
where
    ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: SparseHierarchy,

    Op: crate::apply::Op<
        Level0Mask = Array::Level0MaskType,
        Level1Mask = Array::Level1MaskType,
        Level2Mask = Array::Level2MaskType,
        DataBlock  = Array::DataBlockType,
    >,
{
    const EXACT_HIERARCHY: bool = Op::EXACT_HIERARCHY;
    
    type Level0MaskType = Array::Level0MaskType;
    type Level0Mask<'b> = Self::Level0MaskType where Self: 'b;

    #[inline]
    fn level0_mask(&self) -> Self::Level0Mask<'_> {
        /*let mut arrays_iter = self.arrays.clone();
        let mut first = unsafe{ arrays_iter.next().unwrap_unchecked() };
        let mut out = first.borrow().level0_mask().into_owned();
        
        while let Some(array) = arrays_iter.next(){
            out = self.op.lvl0_op(out, array.borrow().level0_mask());
        }
        
        out*/
        
        let mut array_iter = self.array_iter.clone();
        let mut first = unsafe{ array_iter.next().unwrap_unchecked() };
        array_iter.fold(
            first.level0_mask().into_owned(), 
            |acc, array|{
                self.op.lvl0_op(acc, array.level0_mask())
            }
        )
    }

    type Level1MaskType = Op::Level1Mask;
    type Level1Mask<'b> where Self: 'b = Op::Level1Mask;

    #[inline]
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask<'_> {
        todo!()
    }

    type Level2MaskType = Op::Level2Mask;
    type Level2Mask<'b> where Self: 'b = Op::Level2Mask;

    #[inline]
    unsafe fn level2_mask(&self, level0_index: usize, level1_index: usize) -> Self::Level2Mask<'_> {
        todo!()
    }

    type DataBlockType =  Op::DataBlock;
    type DataBlock<'b> where Self: 'b = Op::DataBlock;

    #[inline]
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize, level2_index: usize) -> Self::DataBlock<'_> {
        todo!()
    }
}

const N: usize = 32;

pub struct ReduceIterState<'a, Array>
where
    Array: LevelMasksIter
{
    states: ArrayVec<(&'a Array, Array::IterState), N>,
    
    // TODO: ZST when not in use 
    /// In-use only when `Op::SKIP_EMPTY_HIERARCHIES` raised.
    lvl1_non_empty_states: ArrayVec<usize, N>,
    lvl2_non_empty_states: ArrayVec<usize, N>,
}

impl<'a, Op, ArrayIter, Array> LevelMasksIter for Reduce<'a, Op, ArrayIter, Array>
where
    ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: LevelMasksIter,

    Op: crate::apply::Op<
        Level0Mask = Array::Level0MaskType,
        Level1Mask = Array::Level1MaskType,
        Level2Mask = Array::Level2MaskType,
        DataBlock  = Array::DataBlockType,
    >,
{
    type IterState = ReduceIterState<'a, Array>;
    
    #[inline]
    fn make_state(&self) -> Self::IterState{
        let mut states = ArrayVec::new();
        for array in self.array_iter.clone(){
            unsafe{ 
                states.push_unchecked((array, array.make_state())); 
            }
        }
        
        ReduceIterState{
            states,
            lvl1_non_empty_states: Default::default(),
            lvl2_non_empty_states: Default::default(),
        }
    }
    
    #[inline]
    unsafe fn init_level1_block_meta(&self, state: &mut Self::IterState, level0_index: usize) -> (Self::Level1Mask<'_>, bool) {
        if Op::SKIP_EMPTY_HIERARCHIES{
            state.lvl1_non_empty_states.clear();
        }
        
        let mut states_iter = state.states.iter_mut();
        let (first_array, first_state) = states_iter.next().unwrap_unchecked();
        let (acc_mask, v) = first_array.init_level1_block_meta(first_state, level0_index);
        if Op::SKIP_EMPTY_HIERARCHIES{
            if v{
                state.lvl1_non_empty_states.push_unchecked(0);
            }
        }
        
        let mut i = 1;
        let mut acc_mask = acc_mask.into_owned();
        for (array, array_state) in states_iter{
            let (mask, v) = array.init_level1_block_meta(array_state, level0_index);
            acc_mask = self.op.lvl1_op(acc_mask, mask);
            
            if Op::SKIP_EMPTY_HIERARCHIES{
                if v{
                    state.lvl1_non_empty_states.push_unchecked(i);
                }
                i += 1;
            }
        }
        
        let is_empty = acc_mask.is_zero(); 
        (acc_mask, !is_empty)
    }

    #[inline]
    unsafe fn init_level2_block_meta(&self, state: &mut Self::IterState, level1_index: usize) -> (Self::Level2Mask<'_>, bool) {
        todo!()
    }

    #[inline]
    unsafe fn data_block_from_meta(&self, state: &Self::IterState, level_index: usize) -> Self::DataBlock<'_> {
        if Op::SKIP_EMPTY_HIERARCHIES
        && level_bypass::<Self>() != LevelBypass::Level1Level2
        {
            let states =
                if LevelBypass::Level2 == level_bypass::<Self>(){
                    state.lvl1_non_empty_states.iter()
                } else {
                    debug_assert!(LevelBypass::None == level_bypass::<Self>());
                    state.lvl2_non_empty_states.iter()
                }
                .map(|i|{
                    let s = state.states.get_unchecked(*i);
                    (s.0, &s.1)
                });
            if states.len() == 0{
                return Self::DataBlock::empty();
            }
            return Self::do_data_block_from_meta(states, &self.op, level_index);
        }
        
        let states = state.states.iter().map(|s|
            (s.0, &s.1)
        );        
        Self::do_data_block_from_meta(states, &self.op, level_index)
    }
}