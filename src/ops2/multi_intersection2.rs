use std::marker::PhantomData;
use std::borrow::Borrow;
use std::slice;
use arrayvec::ArrayVec;
use crate::BitBlock;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::utils::{Array, Borrowable, Take};

pub struct MultiIntersection<Iter, F, T> {
    iter: Iter,
    f: F,
    phantom_data: PhantomData<T>
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Borrowable>::Borrowed;

impl<Iter, F, T> SparseHierarchy2 for MultiIntersection<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> F: Fn(DataIter<'a, Iter>) -> T
{
    type LevelCount = <IterItem<Iter> as SparseHierarchy2>::LevelCount;

    type LevelMaskType = <IterItem<Iter> as SparseHierarchy2>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self: 'a;
    
    type DataType = T;
    type Data<'a> = T where Self: 'a;

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

    type State = MultiIntersectionState<Iter, F, T>;
}

const N: usize = 32;
type StatesItem<Iter> = (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy2>::State);

pub struct MultiIntersectionState<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
{
    states: ArrayVec<
        (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy2>::State),
        N
    >,    
    empty_below_n: usize,
    phantom_data: PhantomData<(Iter, F, T)>
}

impl<Iter, F, T> SparseHierarchyState2 for MultiIntersectionState<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> F: Fn(DataIter<'a, Iter>) -> T
{
    type This = MultiIntersection<Iter, F, T>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        let states = ArrayVec::from_iter(
            this.iter.clone()
                .map(|array|{
                    let state = SparseHierarchyState2::new(array.borrow()); 
                    (array, state)
                })
        );
        
        Self {
            states,
            empty_below_n: usize::MAX,
            phantom_data: PhantomData,
        }        
    }

    #[inline]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        // if we know that upper levels returned empty - return early.
        if N > self.empty_below_n {
            return BitBlock::zero(); 
        }
        
        let mut states_iter = self.states.iter_mut();
        
        let mut acc_mask = 
            if let Some((array, array_state)) = states_iter.next(){
                array_state.select_level_node(
                    (&*array).borrow(), level_n, level_index
                ).take_or_clone()
            } else {
                return BitBlock::zero();
            };
        
        for (array, array_state) in states_iter {
            let mask = array_state.select_level_node(
                (&*array).borrow(), level_n, level_index
            );
            acc_mask &= mask.borrow();
        }
        
        self.empty_below_n = if acc_mask.is_zero(){
             N
        } else {
            usize::MAX
        };
        
        acc_mask
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger> (
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        let mut states_iter = self.states.iter_mut();
        
        let mut acc_mask = 
            if let Some((array, array_state)) = states_iter.next(){
                array_state.select_level_node_unchecked(
                    (&*array).borrow(), level_n, level_index
                ).take_or_clone()
            } else {
                return BitBlock::zero();
            };
        
        for (array, array_state) in states_iter {
            let mask = array_state.select_level_node_unchecked(
                (&*array).borrow(), level_n, level_index
            );
            acc_mask &= mask.borrow();
        }            
        acc_mask
    }

    #[inline]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> Option<<Self::This as SparseHierarchy2>::Data<'a>> 
    {
        if N > self.empty_below_n {
            return None; 
        }
        
        todo!("Unimplementable without generic configurable DataIter")
    }

    #[inline]
    unsafe fn data_unchecked<'a>(
        &self, this: &'a Self::This, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::Data<'a> {
        (this.f)(DataIter{ level_index, states_iter: self.states.iter() })
    }
}

// States slice to Data iterator adapter.
pub struct DataIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>
{
    level_index: usize,
    states_iter: slice::Iter<'a, StatesItem<I>>
}

impl<'a, I> Iterator for DataIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>
{
    /// <I::Item as SparseHierarchy2>::Data<'a>
    type Item = <IterItem<I> as SparseHierarchy2>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Compiler optimizes away additional branching here.
        self.states_iter
            .next()
            .map(|(array, array_state)| unsafe { 
                array_state.data_unchecked(array.borrow(), self.level_index)
            })
    }
}

impl<Iter, Init, F> Borrowable for MultiIntersection<Iter, Init, F>{ type Borrowed = Self; }

#[inline]
pub fn multi_intersection2<Iter, F, T>(iter: Iter, f: F) 
    -> MultiIntersection<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> F: Fn(DataIter<'a, Iter>) -> T
{
    MultiIntersection{ iter, f, phantom_data: Default::default() }
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array2::CompactSparseArray2;
    use crate::ops2::multi_intersection2::multi_intersection2;
    use crate::sparse_hierarchy2::SparseHierarchy2;

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
        
        let intersection = multi_intersection2(arrays.iter(), |vs| vs.sum() ); 
        
        assert_equal(intersection.iter(), [(15, 45)]);
    }

}
