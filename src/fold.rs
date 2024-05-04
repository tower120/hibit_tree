use std::borrow::Borrow;
use std::marker::PhantomData;
use arrayvec::ArrayVec;
use crate::{BitBlock, IntoOwned, primitive_array};
use crate::const_int::ConstInteger;
use crate::level_block::LevelBlock;
use crate::primitive_array::{ConstArray, ConstArrayType};
use crate::sparse_hierarchy::{DefaultState, SparseHierarchy, SparseHierarchyState};

// TODO: We can go without ArrayIter being Clone!
pub struct Fold<'a, Op, Init, /*ArrayIter, */Array>{
    pub(crate) op: Op,
    pub(crate) init: &'a Init,
    pub(crate) arrays: ArrayVec<&'a Array, N>,
    //pub(crate) array_iter: ArrayIter,
    //pub(crate) phantom: PhantomData<&'a Array>,
}

impl<'a, Op, Init, /*ArrayIter, */Array> SparseHierarchy for Fold<'a, Op, Init, /*ArrayIter, */Array>
where
    Init: SparseHierarchy<
        LevelCount    = Array::LevelCount,
        LevelMaskType = Array::LevelMaskType,
    >,

    //ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: SparseHierarchy,

    Op: crate::apply::Op<
        LevelMask = Array::LevelMaskType,
        DataBlockL = Init::DataBlockType,
        DataBlockR = Array::DataBlockType,
        DataBlockO = Init::DataBlockType,
    >,
{
    const EXACT_HIERARCHY: bool = Op::EXACT_HIERARCHY;
    type LevelCount = Array::LevelCount;

    type LevelMaskType = Array::LevelMaskType;
    type LevelMask<'b> = Self::LevelMaskType where Self: 'b;

    #[inline]
    unsafe fn level_mask<I>(&self, level_indices: I) -> Self::LevelMask<'_>
    where 
        I: ConstArray<Item=usize> + Copy
    {
        //self.array_iter.clone()
        self.arrays.iter()
            .fold(
            self.init.level_mask(level_indices).into_owned(), 
            |acc, array|{
                self.op.lvl_op(acc, array.level_mask(level_indices))
            }
        )
    }

    type DataBlockType = Op::DataBlockO;
    type DataBlock<'b> where Self: 'b = Op::DataBlockO;

    #[inline]
    unsafe fn data_block<I>(&self, level_indices: I) -> Self::DataBlock<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        //self.array_iter
        self.arrays.iter()
            .clone().fold(
            self.init.data_block(level_indices).into_owned(), 
            |acc, array|{
                self.op.data_op(acc, array.data_block(level_indices))
            }
        )
    }

    #[inline]
    fn empty_data_block(&self) -> Self::DataBlock<'_> {
        <Op::DataBlockO as LevelBlock>::empty()
    }

    type State = FoldState<'a, Op, Init, /*ArrayIter, */Array>;
    //type State = DefaultState<Self>;
}

const N: usize = 32;

pub struct FoldState<'a, Op, Init, /*ArrayIter,*/ Array>
where
    Init: SparseHierarchy,
    Array: SparseHierarchy,

    Init: SparseHierarchy<
        LevelCount    = Array::LevelCount,
        LevelMaskType = Array::LevelMaskType,
    >,
{
    init_state: Init::State,
    // TODO: len inside of ArrayVecs unnecessary, since we have it in Fold 
    states: ArrayVec</*(&'a Array,*/ Array::State/*)*/, N>,
    
    // TODO: ZST when not in use 
    /// In-use only when `Op::SKIP_EMPTY_HIERARCHIES` raised.
    /// 
    /// [ArrayVec<usize, N>; Array::LevelCount::N - 1]
    lvls_non_empty_states: ConstArrayType<
        ArrayVec<usize, N>,
        <Array::LevelCount as ConstInteger>::Dec
    >,
    
    phantom_data: PhantomData<Fold<'a, Op, Init, /*ArrayIter, */Array>>
}

impl<'a, Op, Init, /*ArrayIter, */Array> SparseHierarchyState 
for 
    FoldState<'a, Op, Init, /*ArrayIter,*/ Array>
where
    Init: SparseHierarchy<
        LevelCount    = Array::LevelCount,
        LevelMaskType = Array::LevelMaskType,
    >,

    // ArrayIter: Iterator<Item = &'a Array> + Clone,
    Array: SparseHierarchy,

    Op: crate::apply::Op<
        LevelMask = Array::LevelMaskType,
        DataBlockL = Init::DataBlockType,
        DataBlockR = Array::DataBlockType,
        DataBlockO = Init::DataBlockType,
    >,
{
    type This = Fold<'a, Op, Init, /*ArrayIter, */Array>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        /*let states = ArrayVec::from_iter(
            this.array_iter.clone()
                .map(|array| (array, SparseHierarchyState::new(array)))
        );*/
        
        let states = ArrayVec::from_iter(
            this.arrays.iter()
                .map(|&array| SparseHierarchyState::new(array))
        );
        
        Self{
            init_state: SparseHierarchyState::new(this.init.borrow()),
            states,
            lvls_non_empty_states: primitive_array::Array::from_fn(|_|ArrayVec::new()),
            phantom_data: PhantomData,
        }
    }

    #[inline]
    unsafe fn select_level_bock<'t, N: ConstInteger>(
        &mut self, this: &'t Self::This, level_n: N, level_index: usize
    ) -> (<Self::This as SparseHierarchy>::LevelMask<'t>, bool) {
        let (acc_mask, _) = self.init_state.select_level_bock(this.init.borrow(), level_n, level_index);
        let mut acc_mask = acc_mask.into_owned();
        
        if Op::SKIP_EMPTY_HIERARCHIES
        && N::VALUE != 0 
        {
            todo!()
            /*let lvl_non_empty_states = self.lvls_non_empty_states.as_mut().get_unchecked_mut(level_n.value()-1); 
            lvl_non_empty_states.clear();
            for i in 0..self.states.len(){
                let (array, array_state) = self.states.get_unchecked_mut(i);
                let (mask, v) = array_state.select_level_bock(array, level_n, level_index);
                acc_mask = this.op.lvl_op(acc_mask, mask);
                
                if v{
                    lvl_non_empty_states.push_unchecked(i);
                }
            }*/
        } else {
            //for (array, array_state) in self.states.iter_mut(){
            for i in 0..this.arrays.len(){
                let array = this.arrays.get_unchecked(i);
                let array_state = self.states.get_unchecked_mut(i);
            
                let (mask, _) = array_state.select_level_bock(array, level_n, level_index);
                acc_mask = this.op.lvl_op(acc_mask, mask);
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
        
        if Op::SKIP_EMPTY_HIERARCHIES {
            todo!()
            /*let lvl_non_empty_states = self.lvls_non_empty_states.as_ref().last().unwrap_unchecked();
            for &i in lvl_non_empty_states {
                let (array, array_state) = self.states.get_unchecked(i);
                let data = array_state.data_block(array, level_index);
                acc = this.op.data_op(acc, data);
            }*/
        } else {
            //for (array, array_state) in &self.states {
            for i in 0..this.arrays.len(){
                let array = this.arrays.get_unchecked(i);
                let array_state = self.states.get_unchecked(i);
            
                let data = array_state.data_block(array, level_index);
                acc = this.op.data_op(acc, data);
            }
        }
        
        acc
    }    
}