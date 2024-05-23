use std::borrow::Borrow;
use std::marker::PhantomData;
use arrayvec::ArrayVec;
use crate::BitBlock;
use crate::const_utils::const_bool::ConstBool;
use crate::const_utils::const_int::ConstInteger;
use crate::const_utils::const_array::{ConstArray, ConstArrayType};
use crate::const_utils::ConstUsize;
use crate::MaybeEmpty;
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::utils::{Borrowable, IntoOwned, array};

pub struct Fold<Op, Init, ArrayIter>{
    pub(crate) op: Op,
    pub(crate) init: Init,
    pub(crate) array_iter: ArrayIter,
}

type Array<ArrayIter> = <<ArrayIter as Iterator>::Item as Borrowable>::Borrowed;

impl<Op, Init, ArrayIter> SparseHierarchy for Fold<Op, Init, ArrayIter>
where
    Init: Borrowable< 
        Borrowed: SparseHierarchy<
            LevelCount    = <Array<ArrayIter> as SparseHierarchy>::LevelCount,
            LevelMaskType = <Array<ArrayIter> as SparseHierarchy>::LevelMaskType,
        >
    >,

    ArrayIter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,

    Op: crate::apply::Op<
        LevelMask  = <Array<ArrayIter> as SparseHierarchy>::LevelMaskType,
        DataBlockL = <Init::Borrowed as SparseHierarchy>::DataType,
        DataBlockR = <Array<ArrayIter> as SparseHierarchy>::DataType,
        DataBlockO = <Init::Borrowed as SparseHierarchy>::DataType,
    >,
{
    const EXACT_HIERARCHY: bool = Op::EXACT_HIERARCHY;
    type LevelCount = <Array<ArrayIter> as SparseHierarchy>::LevelCount;

    type LevelMaskType = <Array<ArrayIter> as SparseHierarchy>::LevelMaskType;
    type LevelMask<'b> = Self::LevelMaskType where Self: 'b;

    #[inline]
    unsafe fn level_mask<I>(&self, level_indices: I) -> Self::LevelMask<'_>
    where 
        I: ConstArray<Item=usize> + Copy
    {
        self.array_iter.clone().fold(
            self.init.borrow().level_mask(level_indices).into_owned(), 
            |acc, array|{
                self.op.lvl_op(acc, array.borrow().level_mask(level_indices))
            }
        )
    }

    type DataType = Op::DataBlockO;
    type Data<'b> where Self: 'b = Op::DataBlockO;

    #[inline]
    unsafe fn data_block<I>(&self, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        self.array_iter.clone().fold(
            self.init.borrow().data_block(level_indices).into_owned(),
            |acc, array|{
                self.op.data_op(acc, array.borrow().data_block(level_indices))
            }
        )
    }

    #[inline]
    fn empty_data(&self) -> Self::Data<'_> {
        <Op::DataBlockO as MaybeEmpty>::empty()
    }

    type State = FoldState<Op, Init, ArrayIter>;
}

const N: usize = 32;

pub struct FoldState<Op, Init, ArrayIter>
where
    Init: Borrowable<Borrowed: SparseHierarchy>,
    ArrayIter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>,
    Op: crate::apply::Op
{
    init_state: <Init::Borrowed as SparseHierarchy>::State,
    states: ArrayVec<
        (<ArrayIter as Iterator>::Item, <Array<ArrayIter> as SparseHierarchy>::State),
        N
    >,
    
    /// In-use only when `Op::SKIP_EMPTY_HIERARCHIES` raised.
    /// 
    /// [ArrayVec<usize, N>; Array::LevelCount - 1]
    /// 
    /// [ArrayVec<usize, N>; 0] - otherwise
    lvls_non_empty_states: ConstArrayType<
        ArrayVec<usize, N>,
        <Op::SKIP_EMPTY_HIERARCHIES as ConstBool>::ConditionalInt<
            <<Array<ArrayIter> as SparseHierarchy>::LevelCount as ConstInteger>::Dec,
            ConstUsize<0>
        >
    >,
    
    phantom_data: PhantomData<Fold<Op, Init, ArrayIter>>
}

impl<Op, Init, ArrayIter> SparseHierarchyState 
for 
    FoldState<Op, Init, ArrayIter>
where
    Init: Borrowable< 
        Borrowed: SparseHierarchy<
            LevelCount    = <Array<ArrayIter> as SparseHierarchy>::LevelCount,
            LevelMaskType = <Array<ArrayIter> as SparseHierarchy>::LevelMaskType,
        >
    >,

    ArrayIter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,

    Op: crate::apply::Op<
        LevelMask  = <Array<ArrayIter> as SparseHierarchy>::LevelMaskType,
        DataBlockL = <Init::Borrowed as SparseHierarchy>::DataType,
        DataBlockR = <Array<ArrayIter> as SparseHierarchy>::DataType,
        DataBlockO = <Init::Borrowed as SparseHierarchy>::DataType,
    >,
{
    type This = Fold<Op, Init, ArrayIter>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        let states = ArrayVec::from_iter(
            this.array_iter.clone()
                .map(|array|{
                    let state = SparseHierarchyState::new(array.borrow()); 
                    (array, state)
                })
        );
        
        Self{
            init_state: SparseHierarchyState::new(this.init.borrow()),
            states,
            lvls_non_empty_states: array::Array::from_fn(|_|ArrayVec::new()),
            phantom_data: PhantomData,
        }
    }

    #[inline]
    unsafe fn select_level_bock<'t, N: ConstInteger>(
        &mut self, this: &'t Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'t> {
        let mut acc_mask = self.init_state
                          .select_level_bock(this.init.borrow(), level_n, level_index)
                          .into_owned();
        
        if Op::SKIP_EMPTY_HIERARCHIES::VALUE
        && N::VALUE != 0 
        {
            let lvl_non_empty_states = self.lvls_non_empty_states.as_mut()
                                      .get_unchecked_mut(level_n.value()-1); 
            lvl_non_empty_states.clear();
            for i in 0..self.states.len(){
                let (array, array_state) = self.states.get_unchecked_mut(i);
                let mask = array_state.select_level_bock(
                    (&*array).borrow(), level_n, level_index
                );
                acc_mask = this.op.lvl_op(acc_mask, mask);
                
                if !acc_mask.is_zero() {
                    lvl_non_empty_states.push_unchecked(i);
                }
            }
        } else {
            for (array, array_state) in self.states.iter_mut() {
                let mask = array_state.select_level_bock(
                    (&*array).borrow(), level_n, level_index
                );
                acc_mask = this.op.lvl_op(acc_mask, mask);
            }
        }
        
        acc_mask
    }
    
    #[inline]
    unsafe fn data_block<'this>(&self, this: &'this Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::Data<'this> 
    {
        let mut acc = self.init_state.data_block(this.init.borrow(), level_index).into_owned();
        
        if Op::SKIP_EMPTY_HIERARCHIES::VALUE {
            let lvl_non_empty_states = self.lvls_non_empty_states.as_ref()
                                       .last().unwrap_unchecked();
            for &i in lvl_non_empty_states {
                let (array, array_state) = self.states.get_unchecked(i);
                let data = array_state.data_block(array.borrow(), level_index);
                acc = this.op.data_op(acc, data);
            }
        } else {
            for (array, array_state) in &self.states {
                let data = array_state.data_block(array.borrow(), level_index);
                acc = this.op.data_op(acc, data);
            }
        }
        
        acc
    }    
}

impl<Op, Init, ArrayIter> Borrowable for Fold<Op, Init, ArrayIter>{
    type Borrowed = Fold<Op, Init, ArrayIter>;
}
impl<Op, Init, ArrayIter> Borrowable for &Fold<Op, Init, ArrayIter>{
    type Borrowed = Fold<Op, Init, ArrayIter>;
}