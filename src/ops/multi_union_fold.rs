use std::marker::PhantomData;
use std::borrow::Borrow;
use arrayvec::ArrayVec;
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::{BitBlock, SparseHierarchy, SparseHierarchyState};
use crate::utils::{Array, Borrowable};

// TODO: Init as fn not value?

pub struct MultiUnion2<ArrayIter, Init, F> {
    array_iter: ArrayIter,
    init_value: Init,
    f: F,
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Borrowable>::Borrowed;

impl<ArrayIter, Init, F> SparseHierarchy2 for MultiUnion2<ArrayIter, Init, F>
where
    ArrayIter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    Init: Clone,
    F: Fn(Init, &<IterItem<ArrayIter> as SparseHierarchy2>::DataType) -> Init
{
    type LevelCount = <IterItem<ArrayIter> as SparseHierarchy2>::LevelCount;

    type LevelMaskType = <IterItem<ArrayIter> as SparseHierarchy2>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self: 'a;
    
    type DataType = Init;
    type Data<'a> = Init where Self: 'a;

    unsafe fn data<I>(&self, level_indices: I) -> Option<Self::Data<'_>>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        todo!()
    }

    unsafe fn data_unchecked<I>(&self, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        todo!()
    }

    type State = MultiUnion2State<ArrayIter, Init, F>;
}

// TODO: Configurable State storage 
const N: usize = 32;
type StateIndex = u8;

/// [S::Mask; S::DEPTH]
type Masks<S> = ConstArrayType<
    <<S as Borrowable>::Borrowed as SparseHierarchy2>::LevelMaskType,
    <<S as Borrowable>::Borrowed as SparseHierarchy2>::LevelCount,
>;

pub struct MultiUnion2State<ArrayIter, Init, F>
where
    ArrayIter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>,
{
    states: ArrayVec<
        (<ArrayIter as Iterator>::Item, <IterItem<ArrayIter> as SparseHierarchy2>::State),
        N
    >,
    
    /// [ArrayVec<usize, N>; Array::LevelCount - 1]
    lvls_non_empty_states: ConstArrayType<
        ArrayVec<StateIndex, N>,
        <<IterItem<ArrayIter> as SparseHierarchy2>::LevelCount as ConstInteger>::Dec,
    >,
    
    phantom_data: PhantomData<(ArrayIter, Init, F)>
}

impl<ArrayIter, Init, F> SparseHierarchyState2
for 
    MultiUnion2State<ArrayIter, Init, F>
where
    ArrayIter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    Init: Clone,
    F: Fn(Init, &<IterItem<ArrayIter> as SparseHierarchy2>::DataType) -> Init
{
    type This = MultiUnion2<ArrayIter, Init, F>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        let states = ArrayVec::from_iter(
            this.array_iter.clone()
                .map(|array|{
                    let state = SparseHierarchyState2::new(array.borrow()); 
                    (array, state)
                })
        );
        
        Self {
            states,
            lvls_non_empty_states: Array::from_fn(|_|ArrayVec::new()),
            phantom_data: PhantomData,
        }
    }

    #[inline]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        // unchecked version already deal with non-existent elements
        self.select_level_node_unchecked(this, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        let mut acc_mask = BitBlock::zero();
        
        if N::VALUE == 0 {
            for (array, array_state) in self.states.iter_mut() {
                let mask = array_state.select_level_node(
                    (&*array).borrow(), level_n, level_index
                );
                acc_mask |= mask.borrow();
            }            
            return acc_mask;
        } 
        
        let lvl_non_empty_states = self.lvls_non_empty_states.as_mut()
                                  .get_unchecked_mut(level_n.value()-1);
        lvl_non_empty_states.clear();
        for i in 0..self.states.len() {
            let (array, array_state) = self.states.get_unchecked_mut(i);
            let mask = array_state.select_level_node(
                (&*array).borrow(), level_n, level_index
            );
            let mask = mask.borrow();
            acc_mask |= mask;
            if !mask.is_zero() {
                lvl_non_empty_states.push_unchecked(i as _);
            }
        }        
        
        acc_mask
    }

    #[inline]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> Option<<Self::This as SparseHierarchy2>::Data<'a>> 
    {
        // TODO: compile-time special case for 1-level SparseHierarchy.
        
        let lvl_non_empty_states = self.lvls_non_empty_states.as_ref()
                                   .last().unwrap_unchecked();
        if lvl_non_empty_states.is_empty(){
            return None;
        }        
        
        let mut acc = this.init_value.clone();
        for &i in lvl_non_empty_states {
            let (array, array_state) = self.states.get_unchecked(i as usize);
            if let Some(data) = array_state.data(array.borrow(), level_index){
                acc = (this.f)(acc, data.borrow());    
            }
        }
        Some(acc)        
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy2>::Data<'a> 
    {
        self.data(this, level_index).unwrap_unchecked()
    }
}

impl<ArrayIter, Init, F> Borrowable for MultiUnion2<ArrayIter, Init, F>{ type Borrowed = Self; }

#[inline]
pub fn multi_union<Iter, Init, F>(array_iter: Iter, init_value: Init, f: F) 
    -> MultiUnion2<Iter, Init, F>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    Init: Clone,
    F: Fn(Init, &<IterItem<Iter> as SparseHierarchy2>::DataType) -> Init
{
    MultiUnion2 { array_iter, init_value, f }
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array2::CompactSparseArray2;
    use crate::sparse_hierarchy2::SparseHierarchy2;
    use crate::ops2::multi_union2::multi_union;

    #[test]
    fn smoke_test(){
        type Array = CompactSparseArray2<usize, 3>;
        let mut a1 = Array::default();
        let mut a2 = Array::default();
        let mut a3 = Array::default();
        
        *a1.get_or_insert(10) = 10;
        *a1.get_or_insert(15) = 15;
        *a1.get_or_insert(200) = 200;
        
        *a2.get_or_insert(100) = 100;
        *a2.get_or_insert(15)  = 15;
        *a2.get_or_insert(200) = 200;
        
        *a3.get_or_insert(300) = 300;
        *a3.get_or_insert(15)  = 15;
        
        let arrays = [a1, a2, a3];
        
        let union = multi_union(arrays.iter(), 0, |acc, v| acc + v); 
        
        assert_equal(union.iter(), [(10, 10), (15, 45), (100, 100), (200, 400), (300, 300)]);
    }

}
