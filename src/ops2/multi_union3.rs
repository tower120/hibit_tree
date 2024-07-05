use std::marker::PhantomData;
use std::borrow::Borrow;
use std::slice;
use arrayvec::ArrayVec;
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::BitBlock;
use crate::utils::{Array, Borrowable};

pub struct MultiUnion<Iter, F, T> {
    array_iter: Iter,
    f: F,
    phantom_data: PhantomData<T>
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Borrowable>::Borrowed;

impl<Iter, F, T> SparseHierarchy2 for MultiUnion<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> F: Fn(MultiUnionResolveIter<'a, Iter>) -> T,
{
    const EXACT_HIERARCHY: bool = <IterItem<Iter> as SparseHierarchy2>::EXACT_HIERARCHY;
    
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

    type State = MultiUnion2State<Iter, F, T>;
}

// TODO: Configurable State storage 
const N: usize = 32;
type StateIndex = u8;
type StatesItem<Iter> = (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy2>::State);

/// [S::Mask; S::DEPTH]
type Masks<S> = ConstArrayType<
    <<S as Borrowable>::Borrowed as SparseHierarchy2>::LevelMaskType,
    <<S as Borrowable>::Borrowed as SparseHierarchy2>::LevelCount,
>;

pub struct MultiUnion2State<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>,
{
    states: ArrayVec<
        (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy2>::State),
        N
    >,
    
    /// [ArrayVec<usize, N>; Array::LevelCount - 1]
    lvls_non_empty_states: ConstArrayType<
        ArrayVec<StateIndex, N>,
        <<IterItem<Iter> as SparseHierarchy2>::LevelCount as ConstInteger>::Dec,
    >,
    
    phantom_data: PhantomData<(Iter, F, T)>
}

impl<Iter, F, T> SparseHierarchyState2
for 
    MultiUnion2State<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> F: Fn(MultiUnionResolveIter<'a, Iter>) -> T
{
    type This = MultiUnion<Iter, F, T>;

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
        if <Self::This as SparseHierarchy2>::LevelCount::VALUE == 1 {
            todo!("TODO: compile-time special case for 1-level SparseHierarchy");
        }
        
        let lvl_non_empty_states = self.lvls_non_empty_states.as_ref()
                                   .last().unwrap_unchecked();
        if lvl_non_empty_states.is_empty(){
            return None;
        }
        
        Some(
            (this.f)(MultiUnionResolveIter {
                lvl_non_empty_states: lvl_non_empty_states.iter(),
                states: &self.states,
                level_index,
            })
        )
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy2>::Data<'a> 
    {
        self.data(this, level_index).unwrap_unchecked()
    }
}

impl<ArrayIter, Init, F> Borrowable for MultiUnion<ArrayIter, Init, F>{ type Borrowed = Self; }

/// This iterator is **GUARANTEED** to be initially non-empty. 
/// 
/// Prefer [fold]-dependent[^1] operations whenever possible, instead of just
/// [next]ing items.
/// 
/// [^1]: All [Iterator] operations that redirect to `fold` under the hood: 
/// [for_each], [sum], etc.
pub struct MultiUnionResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>
{
    lvl_non_empty_states: slice::Iter<'a, StateIndex>,
    states: &'a [StatesItem<I>],
    level_index: usize,
}

impl<'a, I> Iterator for MultiUnionResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>
{
    /// <I::Item as SparseHierarchy2>::Data<'a>
    type Item = <IterItem<I> as SparseHierarchy2>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.lvl_non_empty_states
            .find_map(|&i| unsafe {
                let (array, array_state) = self.states.get_unchecked(i as usize);
                if let Some(data) = array_state.data(array.borrow(), self.level_index) {
                    Some(data)
                } else {
                    None
                }
            })
    }

    #[inline]
    fn fold<B, F>(self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let level_index = self.level_index;
        for &i in self.lvl_non_empty_states {
            let (array, array_state) = unsafe{ self.states.get_unchecked(i as usize) };
            if let Some(data) = unsafe{ array_state.data(array.borrow(), level_index) } {
                init = f(init, data);
            }
        }
        init
    }

    /// Thou it is not state in `size_hint`, freshly constructed iterator
    /// has minimal bound 1. That not tracked for performance reasons. 
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.lvl_non_empty_states.len()))
    }
}

#[inline]
pub fn multi_union<Iter, F, T>(array_iter: Iter, resolve: F) 
    -> MultiUnion<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> F: Fn(MultiUnionResolveIter<'a, Iter>) -> T
{
    MultiUnion { array_iter, f: resolve, phantom_data: PhantomData }
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array2::CompactSparseArray;
    use crate::sparse_hierarchy2::SparseHierarchy2;
    use crate::ops2::multi_union3::multi_union;

    #[test]
    fn smoke_test(){
        type Array = CompactSparseArray<usize, 3>;
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
        
        let union = multi_union( arrays.iter(), |is| is.sum() ); 
        
        assert_equal(union.iter(), [(10, 10), (15, 45), (100, 100), (200, 400), (300, 300)]);
    }

}
