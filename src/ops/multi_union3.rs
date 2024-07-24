use std::marker::PhantomData;
use std::borrow::Borrow;
use std::ptr::NonNull;
use std::slice;
use arrayvec::ArrayVec;
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::BitBlock;
use crate::ops::MultiIntersectionResolveIter;
use crate::utils::{Array, Borrowable};

// TODO: reuse somehow. macro_rules?
/// This iterator is **GUARANTEED** to be initially non-empty. 
/// 
/// Prefer [fold]-dependent[^1] operations whenever possible, instead of just
/// [next]ing items.
/// 
/// [^1]: All [Iterator] operations that redirect to `fold` under the hood: 
/// [for_each], [sum], etc.
pub enum MultiUnionResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{
    stateless_unchecked(ResolveIterUnchecked<'a, I>),
    stateless(ResolveIter<'a, I>),
    stateful(StateResolveIter<'a, I>)
}
impl<'a, Iter> Iterator for MultiUnionResolveIter<'a, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
{
    type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self{
            Self::stateless_unchecked(iter) => iter.next(),
            Self::stateless(iter) => iter.next(),
            Self::stateful(iter) => iter.next(),
        }
    }

    #[inline]
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        match self{
            Self::stateless_unchecked(iter) => iter.fold(init, f),
            Self::stateless(iter) => iter.fold(init, f),
            Self::stateful(iter) => iter.fold(init, f),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self{
            Self::stateless_unchecked(iter) => iter.size_hint(),
            Self::stateless(iter) => iter.size_hint(),
            Self::stateful(iter) => iter.size_hint(),
        }
    }
}

pub struct MultiUnion<Iter, F, T> {
    array_iter: Iter,
    f: F,
    phantom_data: PhantomData<T>
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Borrowable>::Borrowed;

impl<Iter, F, T> SparseHierarchy for MultiUnion<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
    for<'a> F: Fn(MultiUnionResolveIter<'a, Iter>) -> T,
{
    const EXACT_HIERARCHY: bool = <IterItem<Iter> as SparseHierarchy>::EXACT_HIERARCHY;
    
    type LevelCount = <IterItem<Iter> as SparseHierarchy>::LevelCount;

    type LevelMaskType = <IterItem<Iter> as SparseHierarchy>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self: 'a;
    
    type DataType = T;
    type Data<'a> = T where Self: 'a;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) -> Option<Self::Data<'_>> {
        // Use variant 2 from multi_intersection.
        // Gather items - then apply resolve function over.
        let mut datas: ArrayVec<_, N> = Default::default();
        for array in self.array_iter.clone(){
            let array = NonNull::from(array.borrow()); // drop borrow lifetime
            let data = unsafe{ array.as_ref().data(index, level_indices) };
            if let Some(data) = data {
                datas.push(data);
            }
        }
        if datas.is_empty(){
            return None;
        }
        let resolve = (self.f)(
            MultiUnionResolveIter::stateless(
                ResolveIter {
                    items: datas.into_iter()
                }
            )
        );
        Some(resolve)
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> Self::Data<'_> 
    {
        (self.f)(MultiUnionResolveIter::stateless_unchecked(
            ResolveIterUnchecked {
                array_iter: self.array_iter.clone(),
                index,
                level_indices,
            })
        )
    }

    type State = MultiUnionState<Iter, F, T>;
}

pub struct ResolveIter<'a, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
{
    pub items: arrayvec::IntoIter<<IterItem<Iter> as SparseHierarchy>::Data<'a>, N>
    //pub items: std::vec::IntoIter<<IterItem<Iter> as SparseHierarchy>::Data<'a>>
}
impl<'a, Iter> Iterator for ResolveIter<'a, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
{
    type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.items.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.items.size_hint()
    }
}

pub struct ResolveIterUnchecked<'a, I>{
    array_iter: I,
    index: usize, 
    level_indices: &'a [usize],
}
impl<'a, I> Iterator for ResolveIterUnchecked<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
{
    type Item = <IterItem<I> as SparseHierarchy>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.array_iter.find_map(|array|{
            unsafe{
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                array.as_ref().data(self.index, self.level_indices)
            }
        })
    }

    #[inline]
    fn fold<B, F>(self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        for array in self.array_iter {
            unsafe{
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                if let Some(item) = array.as_ref().data(self.index, self.level_indices){
                    init = f(init, item)    
                }
            }
        }
        init
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.array_iter.size_hint().1)
    }
}


// TODO: Configurable State storage 
const N: usize = 32;
type StateIndex = u8;
type StatesItem<Iter> = (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy>::State);

/// [S::Mask; S::DEPTH]
type Masks<S> = ConstArrayType<
    <<S as Borrowable>::Borrowed as SparseHierarchy>::LevelMaskType,
    <<S as Borrowable>::Borrowed as SparseHierarchy>::LevelCount,
>;

pub struct MultiUnionState<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>,
{
    states: ArrayVec<
        (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy>::State),
        N
    >,
    
    /// [ArrayVec<usize, N>; Array::LevelCount - 1]
    lvls_non_empty_states: ConstArrayType<
        ArrayVec<StateIndex, N>,
        <<IterItem<Iter> as SparseHierarchy>::LevelCount as ConstInteger>::Dec,
    >,
    
    phantom_data: PhantomData<(Iter, F, T)>
}

impl<Iter, F, T> SparseHierarchyState
for 
    MultiUnionState<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
    for<'a> F: Fn(MultiUnionResolveIter<'a, Iter>) -> T
{
    type This = MultiUnion<Iter, F, T>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        let states = ArrayVec::from_iter(
            this.array_iter.clone()
                .map(|array|{
                    let state = SparseHierarchyState::new(array.borrow()); 
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
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
        // unchecked version already deal with non-existent elements
        self.select_level_node_unchecked(this, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
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
        -> Option<<Self::This as SparseHierarchy>::Data<'a>> 
    {
        if <Self::This as SparseHierarchy>::LevelCount::VALUE == 1 {
            todo!("TODO: compile-time special case for 1-level SparseHierarchy");
        }
        
        let lvl_non_empty_states = self.lvls_non_empty_states.as_ref()
                                   .last().unwrap_unchecked();
        if lvl_non_empty_states.is_empty(){
            return None;
        }
        
        Some(
            (this.f)(MultiUnionResolveIter::stateful(
             StateResolveIter {
                lvl_non_empty_states: lvl_non_empty_states.iter(),
                states: &self.states,
                level_index,
            }))
        )
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::Data<'a> 
    {
        self.data(this, level_index).unwrap_unchecked()
    }
}

impl<ArrayIter, Init, F> Borrowable for MultiUnion<ArrayIter, Init, F>{ type Borrowed = Self; }

pub struct StateResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{
    lvl_non_empty_states: slice::Iter<'a, StateIndex>,
    states: &'a [StatesItem<I>],
    level_index: usize,
}

impl<'a, I> Iterator for StateResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{
    /// <I::Item as SparseHierarchy2>::Data<'a>
    type Item = <IterItem<I> as SparseHierarchy>::Data<'a>;

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

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.lvl_non_empty_states.len()))
    }
}

#[inline]
pub fn multi_union<Iter, F, T>(array_iter: Iter, resolve: F) 
    -> MultiUnion<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
    for<'a> F: Fn(MultiUnionResolveIter<'a, Iter>) -> T
{
    MultiUnion { array_iter, f: resolve, phantom_data: PhantomData }
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array::CompactSparseArray;
    use crate::sparse_hierarchy::SparseHierarchy;
    use crate::ops::multi_union3::multi_union;

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
        assert_eq!(unsafe{union.get_unchecked(10)}, 10);
        assert_eq!(unsafe{union.get_unchecked(15)}, 45);
        assert_eq!(unsafe{union.get(15)}, Some(45));
        assert_eq!(unsafe{union.get(25)}, None);
    }

}