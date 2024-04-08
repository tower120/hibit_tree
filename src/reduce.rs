use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use std::ptr::NonNull;
use arrayvec::ArrayVec;
use crate::IntoOwned;
use crate::level_masks::{LevelMasks, LevelMasksBorrow, LevelMasksIter, LevelMasksIterState};

pub struct Reduce<Op, ArrayIter, Array>{
    pub(crate) op: Op,
    pub(crate) array_iter: ArrayIter,
    pub(crate) phantom: PhantomData<(Array)>,
}

impl<Op, ArrayIter, Array> LevelMasks for Reduce<Op, ArrayIter, Array>
where
    ArrayIter: Iterator + Clone,
    ArrayIter::Item: Borrow<Array>,
    Array: LevelMasks,

    Op: crate::apply::Op<
        Level0Mask = Array::Level0MaskType,
        Level1Mask = Array::Level1MaskType,
        Level2Mask = Array::Level2MaskType,
        DataBlock  = Array::DataBlockType,
    >,
{
    type Level0MaskType = Array::Level0MaskType;
    type Level0Mask<'a> = Self::Level0MaskType where Self: 'a;

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
            first.borrow().level0_mask().into_owned(), 
            |acc, array|{
                self.op.lvl0_op(acc, array.borrow().level0_mask())
            }
        )
    }

    type Level1MaskType = Op::Level1Mask;
    type Level1Mask<'a> where Self: 'a = Op::Level1Mask;

    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask<'_> {
        todo!()
    }

    type Level2MaskType = Op::Level2Mask;
    type Level2Mask<'a> where Self: 'a = Op::Level2Mask;

    unsafe fn level2_mask(&self, level0_index: usize, level1_index: usize) -> Self::Level2Mask<'_> {
        todo!()
    }

    type DataBlockType =  Op::DataBlock;
    type DataBlock<'a> where Self: 'a = Op::DataBlock;

    unsafe fn data_block(&self, level0_index: usize, level1_index: usize, level2_index: usize) -> Self::DataBlock<'_> {
        todo!()
    }
}

const N: usize = 32;

pub struct ReduceIterState<Op, ArrayIter, Array>
where
    Array: LevelMasksIter
{
    states: ArrayVec<Array::IterState, N>,
    
    lvl1_non_empty_states: ArrayVec<(NonNull<Array>, usize), N>,
    //lvl2_non_empty_states: ArrayVec<(NonNull<Array>, usize), N>,
    
    phantom: PhantomData<(Op, ArrayIter, Array)>
}

impl<Op, ArrayIter, Array> LevelMasksIterState for ReduceIterState<Op, ArrayIter, Array>
where
    ArrayIter: Iterator + Clone,
    ArrayIter::Item: Borrow<Array>,     // TODO: Should be exactly &Array - we store it's pointer.
    Array: LevelMasksIter,

    Op: crate::apply::Op<
        Level0Mask = Array::Level0MaskType,
        Level1Mask = Array::Level1MaskType,
        Level2Mask = Array::Level2MaskType,
        DataBlock  = Array::DataBlockType,
    >,
{
    type Container = Reduce<Op, ArrayIter, Array>;

    fn make(container: &Self::Container) -> Self {
        let mut states = ArrayVec::new();
        for a in container.array_iter.clone(){
            unsafe{
                states.push_unchecked(
                    /*(
                        /*NonNull::from(a.borrow())*/(),
                        Array::IterState::make(a.borrow())
                    )*/
                        Array::IterState::make(a.borrow())
                );
            }
        }
        
        Self{
            states,
            lvl1_non_empty_states: Default::default(),
            phantom: Default::default(),
        }
    }

    fn drop(container: &Self::Container, this: &mut ManuallyDrop<Self>) {
        // TODO
    }
}


impl<Op, ArrayIter, Array> LevelMasksIter for Reduce<Op, ArrayIter, Array>
where
    ArrayIter: Iterator + Clone,
    ArrayIter::Item: Borrow<Array>,
    Array: LevelMasksIter,

    Op: crate::apply::Op<
        Level0Mask = Array::Level0MaskType,
        Level1Mask = Array::Level1MaskType,
        Level2Mask = Array::Level2MaskType,
        DataBlock  = Array::DataBlockType,
    >,
{
    type IterState = ReduceIterState<Op, ArrayIter, Array>;

    unsafe fn init_level1_block_meta(&self, state: &mut Self::IterState, level0_index: usize) -> (Self::Level1Mask<'_>, bool) {
        let mut array_iter = self.array_iter.clone();
        //let mut states_iter = state.states.iter_mut().map(|(ptr, s)| s);
        let mut states_iter = state.states.iter_mut();
        
        let first_array = array_iter.next().unwrap_unchecked();
        let first_state = states_iter.next().unwrap_unchecked();
        
        let (acc_mask, mut acc_v) = first_array.borrow().init_level1_block_meta(first_state, level0_index);
        let mut acc_mask = acc_mask.into_owned();
        
        state.lvl1_non_empty_states.clear();
        let mut i = 0;
        for array in array_iter{
            let array = array.borrow();
            {
                state.lvl1_non_empty_states.push_unchecked((NonNull::from(array), i));
                i += 1;
            }
             
            let state = states_iter.next().unwrap_unchecked();
            let (mask, v) = array.init_level1_block_meta(state, level0_index);
            
            acc_mask = self.op.lvl1_op(acc_mask, mask);
            acc_v = acc_v | v;      // not empty if at least one not-empty TODO: This is approximation!!!
        }
        
        (acc_mask, acc_v)
    }

    unsafe fn init_level2_block_meta(&self, state: &mut Self::IterState, level1_index: usize) -> (Self::Level2Mask<'_>, bool) {
        todo!()
    }

    unsafe fn data_block_from_meta(&self, state: &Self::IterState, level_index: usize) -> Self::DataBlock<'_> {
        // TODO: take bypass into account ?
        
        let mut states = state.lvl1_non_empty_states.iter()
            .map(|(array_ptr, i)| (array_ptr, state.states.get_unchecked(*i)));
        
        /*let mut states =
            self.array_iter.clone()
                .enumerate()
                .map(|(i, a)|{
                    (
                        NonNull::from(a.borrow()),
                        state.states.get_unchecked(i)
                    )
                });*/
        
        //let mut states = state.states.iter();
        
/*        let mut arrays = self.array_iter.clone();
        
        let mut acc = 
            arrays.next().unwrap_unchecked().borrow()
            .data_block_from_meta(
                //&states.next().unwrap_unchecked().1,
                &states.next().unwrap_unchecked(),
                level_index
            )
            .into_owned();*/
//            first.0.as_ref().data_block_from_meta(&first.1, level_index).into_owned();
        
        let first = states.next().unwrap_unchecked();
        let mut acc = first.0.as_ref().data_block_from_meta(&first.1, level_index).into_owned();
        
        
        for (array, state) in states{
            let array = array.as_ref();
            //let array = arrays.next().unwrap_unchecked();
            //let state = state.1;
            let data = array.borrow().data_block_from_meta(state, level_index);
            acc = self.op.data_op(acc, data);
        }
        acc
    }
}