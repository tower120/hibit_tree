use std::marker::PhantomData;
use std::borrow::Borrow;
use arrayvec::ArrayVec;
use crate::BitBlock;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::utils::{Array, Borrowable, Take};

pub struct MultiIntersection<Iter, Init, F> {
    iter: Iter,
    init_value: Init,
    f: F,
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Borrowable>::Borrowed;

impl<Iter, Init, F> SparseHierarchy2 for MultiIntersection<Iter, Init, F>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    Init: Clone,
    F: Fn(Init, &<IterItem<Iter> as SparseHierarchy2>::DataType) -> Init
{
    type LevelCount = <IterItem<Iter> as SparseHierarchy2>::LevelCount;

    type LevelMaskType = <IterItem<Iter> as SparseHierarchy2>::LevelMaskType;
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

    type State = MultiIntersectionState<Iter, Init, F>;
}

const N: usize = 32;

pub struct MultiIntersectionState<Iter, Init, F>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
{
    states: ArrayVec<
        (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy2>::State),
        N
    >,    
    phantom_data: PhantomData<(Iter, Init, F)>
}

impl<Iter, Init, F> SparseHierarchyState2 for MultiIntersectionState<Iter, Init, F>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    Init: Clone,
    F: Fn(Init, &<IterItem<Iter> as SparseHierarchy2>::DataType) -> Init
{
    type This = MultiIntersection<Iter, Init, F>;

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
            phantom_data: PhantomData,
        }        
    }

    unsafe fn select_level_node<'a, N: ConstInteger>(&mut self, this: &'a Self::This, level_n: N, level_index: usize) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        todo!()
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

    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) -> Option<<Self::This as SparseHierarchy2>::Data<'a>> {
        todo!()
    }

    #[inline]
    unsafe fn data_unchecked<'a>(
        &self, this: &'a Self::This, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::Data<'a> {
        let mut acc = this.init_value.clone();
        for (array, array_state) in &self.states {
            let data = array_state.data_unchecked(array.borrow(), level_index);
            acc = (this.f)(acc, data.borrow());    
        }
        acc
    }
}

impl<Iter, Init, F> Borrowable for MultiIntersection<Iter, Init, F>{ type Borrowed = Self; }

#[inline]
pub fn multi_intersection<Iter, Init, F>(iter: Iter, init_value: Init, f: F) 
    -> MultiIntersection<Iter, Init, F>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    Init: Clone,
    F: Fn(Init, &<IterItem<Iter> as SparseHierarchy2>::DataType) -> Init
{
    MultiIntersection{ iter, init_value, f }
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array2::CompactSparseArray2;
    use crate::ops2::multi_intersection::multi_intersection;
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
        
        let intersection = multi_intersection(arrays.iter(), 0, |acc, v| acc + v); 
        
        assert_equal(intersection.iter(), [(15, 45)]);
    }

}
