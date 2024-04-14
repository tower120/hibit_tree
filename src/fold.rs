use std::borrow::Borrow;
use std::marker::PhantomData;
use arrayvec::ArrayVec;
use crate::{BitBlock, IntoOwned};
use crate::sparse_hierarchy::{DefaultState, level_bypass, LevelBypass, SparseHierarchy, SparseHierarchyState};

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

pub struct FoldState<'a, Op, Init, ArrayIter, Array>
where
    Init: SparseHierarchy,
    Array: SparseHierarchy,
{
    init_state: Init::State,
    states: ArrayVec<(&'a Array, Array::State), N>,
    
    // TODO: ZST when not in use 
    /// In-use only when `Op::SKIP_EMPTY_HIERARCHIES` raised.
    lvl1_non_empty_states: ArrayVec<usize, N>,
    lvl2_non_empty_states: ArrayVec<usize, N>,

    phantom_data: PhantomData<Fold<'a, Op, Init, ArrayIter, Array>>
}

impl<'a, Op, Init, ArrayIter, Array> SparseHierarchyState for FoldState<'a, Op, Init, ArrayIter, Array>
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
    type This = Fold<'a, Op, Init, ArrayIter, Array>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        let mut states = ArrayVec::new();
        for array in this.array_iter.clone(){
            states.push((array, SparseHierarchyState::new(array))); 
        }
        
        // TODO: reserve lvl1_non_empty_states, lvl2_non_empty_states
        
        Self{
            init_state: SparseHierarchyState::new(this.init.borrow()),
            states,
            lvl1_non_empty_states: Default::default(),
            lvl2_non_empty_states: Default::default(),
            phantom_data: PhantomData,
        }
    }

    #[inline]
    unsafe fn select_level1<'this>(&mut self, this: &'this Self::This, level0_index: usize) 
        -> (<Self::This as SparseHierarchy>::Level1Mask<'this>, bool) 
    {
        let (acc_mask, _) = self.init_state.select_level1(this.init.borrow(), level0_index);
        let mut acc_mask = acc_mask.into_owned();
        
        if Op::SKIP_EMPTY_HIERARCHIES{
            self.lvl1_non_empty_states.clear();
            for i in 0..self.states.len(){
                let (array, array_state) = self.states.get_unchecked_mut(i);
                let (mask, v) = array_state.select_level1(array, level0_index);
                acc_mask = this.op.lvl1_op(acc_mask, mask);
                
                if v{
                    self.lvl1_non_empty_states.push_unchecked(i);
                }
            }
        } else {
            for (array, array_state) in self.states.iter_mut(){
                let (mask, _) = array_state.select_level1(array, level0_index);
                acc_mask = this.op.lvl1_op(acc_mask, mask);
            }            
        }
        
        let is_empty = acc_mask.is_zero(); 
        (acc_mask, !is_empty)
    }

    #[inline]
    unsafe fn select_level2<'this>(&mut self, this: &'this Self::This, level1_index: usize) 
        -> (<Self::This as SparseHierarchy>::Level2Mask<'this>, bool) 
    {
        let (acc_mask, _) = self.init_state.select_level2(this.init.borrow(), level1_index);
        let mut acc_mask = acc_mask.into_owned();
        
        if Op::SKIP_EMPTY_HIERARCHIES{
            self.lvl2_non_empty_states.clear();
            for &i in &self.lvl1_non_empty_states{
                let (array, array_state) = self.states.get_unchecked_mut(i);
                let (mask, v) = array_state.select_level2(array, level1_index);
                acc_mask = this.op.lvl2_op(acc_mask, mask);
                
                if v{
                    self.lvl2_non_empty_states.push_unchecked(i);
                }
            }
        } else {
            for (array, array_state) in self.states.iter_mut(){
                let (mask, _) = array_state.select_level2(array, level1_index);
                acc_mask = this.op.lvl2_op(acc_mask, mask);
            }
        }
        
        let is_empty = acc_mask.is_zero(); 
        (acc_mask, !is_empty)
    }
    
    #[inline]
    unsafe fn data_block<'this>(&self, this: &'this Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::DataBlock<'this> 
    {
        let mut acc = self.init_state.data_block(this.init.borrow(), level_index).into_owned(); 
        
        if Op::SKIP_EMPTY_HIERARCHIES
        && level_bypass::<Self::This>() != LevelBypass::Level1Level2
        {
            let state_indices =
                if LevelBypass::Level2 == level_bypass::<Self::This>(){
                    self.lvl1_non_empty_states.iter()
                } else {
                    debug_assert!(LevelBypass::None == level_bypass::<Self::This>());
                    self.lvl2_non_empty_states.iter()
                };
            
            for &i in state_indices {
                let (array, array_state) = self.states.get_unchecked(i);
                let data = array_state.data_block(array, level_index);
                acc = this.op.data_op(acc, data);
            }
        } else {
            for (array, array_state) in &self.states {
                let data = array_state.data_block(array, level_index);
                acc = this.op.data_op(acc, data);
            }
        }
        
        acc
    }    
}