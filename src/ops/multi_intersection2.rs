use std::marker::PhantomData;
use std::borrow::Borrow;
use std::ptr::NonNull;
use std::slice;
use arrayvec::ArrayVec;
use crate::{BitBlock, LazySparseHierarchy};
use crate::const_utils::{ConstArray, ConstCopyArrayType, ConstInteger};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::utils::{Array, Borrowable, Take};

// HOPEFULLY this always acts as one of concrete iterator variants,
// without additional branching. At least small tests in godbolt show so.
// (because we construct and immoderately consume iterator in resolve closure)
//
// Ideally, we would need to pass concrete iterators to closure, but that would
// require generic closures.
/// Iterator for [multi_intersection] resolve function.
/// 
// Following part of doc valid for data() variant 1 implementation. 
/*/// Can be one of a few iterators, depending on what operation you call.
/// _(Compiler optimizes away all dispatch switches)_
/// 
/// Iterator returned for [data()]/[get()] is special: it will detect fact of 
/// intersection on the fly - by iterating over items at requested index, 
/// even if there is no actual intersection there (not all SparseHierarchies 
/// return Some at requested index).
/// If iterator meets None item - there is no intersection, and the result
/// of resolve function **will be thrown away**. 
/// 
/// This is the fastest way of doing this - in one go. Other methods either require additional
/// memory for storing all items, while we check if all of them exists at requested index.
/// Or require traversing to the same point twice, first - to check if it exists, 
/// second - to actually get item.
///
/// [data_unchecked()]/[get_unchecked()] just assumes that intersection occurs,
/// so we just don't check.
/// 
/// Stateful operations always have computed bitmask of terminal node, so we always
/// know do we intersect at requested item or not. So this is not the problem for
/// [iter()]. 
/// 
/// All-in-all we believe that this is the most performant solution for the most 
/// cases, which does not require a separate resolve function for each kind of operation.
/// 
/// If an object is heavy to build / heavy to drop, **and** you use [get()] heavily - consider splitting
/// intersection operation into intersection + map:
/// 
/// ```
/// multi_intersection(arrays.iter(), |ds| -> usize {
///     ds.map(|d: Data|d.0).sum()  // gather data for construction of a heavy object.
/// })
/// // this map() will be called only in case of actual intersection 
/// .map(|i| Data(i))   // Data is heavy object. 
/// ```
/// 
/// In the worst case - you can gather items in container - and then process them: 
/// ```
/// #use arrayvec::ArrayVec;
/// multi_intersection(arrays.iter(), |ds| -> ArrayVec<Data, 32> {
///     ds.collect()
/// })
/// // this map() will be called only in case of actual intersection
/// .map(|data_array: ArrayVec<Data, 32>| -> Data {
///     // construct Data somehow very funny from data_array 
/// }) 
/// ```*/
pub enum MultiIntersectionResolveIter<'a, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{
    stateless(ResolveIter<'a, Iter>),
    stateless_unchecked(ResolveIterUnchecked<'a, Iter>),
    statefull(StateResolveIter<'a, Iter>)
}
impl<'a, Iter> Iterator for MultiIntersectionResolveIter<'a, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>,
{
    type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self{
            MultiIntersectionResolveIter::stateless(iter) => iter.next(),
            MultiIntersectionResolveIter::stateless_unchecked(iter) => iter.next(),
            MultiIntersectionResolveIter::statefull(iter) => iter.next(),
        }
    }

    #[inline]
    fn fold<B, F>(self, init: B, f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        match self{
            MultiIntersectionResolveIter::stateless(iter) => iter.fold(init, f),
            MultiIntersectionResolveIter::stateless_unchecked(iter) => iter.fold(init, f),
            MultiIntersectionResolveIter::statefull(iter) => iter.fold(init, f),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self{
            MultiIntersectionResolveIter::stateless(iter) => iter.size_hint(),
            MultiIntersectionResolveIter::stateless_unchecked(iter) => iter.size_hint(),
            MultiIntersectionResolveIter::statefull(iter) => iter.size_hint(),
        }
    }
}

pub struct MultiIntersection<Iter, F, T> {
    iter: Iter,
    f: F,
    phantom_data: PhantomData<T>
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Borrowable>::Borrowed;

impl<Iter, F, T> SparseHierarchy for MultiIntersection<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
    for<'a> F: Fn(MultiIntersectionResolveIter<'a, Iter>) -> T
{
    const EXACT_HIERARCHY: bool = false;
    
    type LevelCount = <IterItem<Iter> as SparseHierarchy>::LevelCount;

    type LevelMaskType = <IterItem<Iter> as SparseHierarchy>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self: 'a;
    
    type DataType = T;
    type Data<'a> = T where Self: 'a;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<Self::Data<'_>> 
    {
        // There are few ways to do it:
        // 1. Iterate, get data() and build resolve value on the fly. As
        //    soon as None meet - throw away half-built resolve value and 
        //    return None. 
        //    This should be the fastest one, when intersection is
        //    successful most of the time. Thou it may be not the best one
        //    from user perspective, since resolve function will act "special"
        //    for get() operations.
        // 2. Iterate, get data() and STORE it. If we do not meet None - pass 
        //    stored data to resolve function.
        // 3. Contains + get_unchecked. We traverse hierarchy TWICE.
        
        // Variant 1 implementation.
/*        {
            if self.iter.clone().next().is_none(){
                return None;
            }
            
            let mut not_intersects = false;
            let resolve = (self.f)(
                MultiIntersectionResolveIter::stateless(
                    ResolveIter {
                        index, 
                        level_indices, 
                        iter: self.iter.clone(), 
                        not_intersects: &mut not_intersects
                    }
                )
            );
            if not_intersects{
                None
            } else {
                Some(resolve)
            }
        }*/
        
        // Variant 2 implementation.
        //
        // Slower 20% than variant 1 on plain data. Could be more on something
        // more complex. (But we expect that generative SparseHierarchies will not
        // be used for heavy objects)
        //
        // Performs poorly with Vec storage instead of ArrayVec (thou if it 
        // does not fit stack - overhead will probably be neglectable).
        //
        // But no "special cases" from user perspective.
        {
            let mut datas: ArrayVec<_, N> = Default::default();
            for array in self.iter.clone(){
                // TODO: This is only OK, if:
                //
                //     SparseHierarchy<Data = DataType>        // LazySparseHierarchy?
                //     ||
                //     Iterator<Item = &impl SparseHierarchy>
                //
                //  Or just accept only Iterator<Item = &impl SparseHierarchy> instead of Borrowable
                
                // TODO: AS-IS this is wrong, if self.iter returns arrays as values,
                //       while array.data() contains pointer/reference to array.
                //
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                let data = unsafe{ array.as_ref().data(index, level_indices) };
                if let Some(data) = data{
                    datas.push(data);
                } else {
                    return None;
                }
            }
            let resolve = (self.f)(
                MultiIntersectionResolveIter::stateless(
                    ResolveIter {
                        items: datas.into_iter()
                    }
                )
            );
            Some(resolve)
        }

        // Variant 3 implementation.
        // Performance degrades linearly with depth increase.
        /*{
            for array in self.iter.clone(){
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                let data = unsafe{ array.as_ref().data(index, level_indices) };
                if data.is_none(){
                    return None;
                }
            }
            
            let resolve = (self.f)(
                MultiIntersectionResolveIter::stateless(
                    ResolveIter {
                        index,
                        level_indices,
                        iter: self.iter.clone(),
                    }
                )
            );
            Some(resolve)            
        }*/
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> Self::Data<'_> 
    {
        (self.f)(
            MultiIntersectionResolveIter::stateless_unchecked(
                ResolveIterUnchecked {
                    index, 
                    level_indices, 
                    iter: self.iter.clone(), 
                }
            )
        )
    }

    type State = MultiIntersectionState<Iter, F, T>;
}

use data_resolve_v2::ResolveIter;

mod data_resolve_v1 {
    use super::*;
    
    pub struct ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        pub index: usize, 
        pub level_indices: &'a [usize],
        pub iter: Iter,
        pub not_intersects: &'a mut bool
    }
    impl<'a, Iter> Iterator for ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;
    
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if let Some(array) = self.iter.next(){
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                if let Some(data) = unsafe{ array.as_ref().data(self.index, self.level_indices) } {
                    return Some(data);
                }
                *self.not_intersects = true;
            }
            None
        }
    }
    impl<'a, Iter> Drop for ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        #[inline]
        fn drop(&mut self) {
            if *self.not_intersects{
                return;
            }
            // search if there are any non-intersected elements left.
            self.fold((), |_, _|());
        }
    }
}

mod data_resolve_v2 {
    use super::*;
    
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

        // Do nothing for ArrayVec/Vec
        /*#[inline]
        fn fold<B, F>(self, init: B, f: F) -> B
        where
            Self: Sized,
            F: FnMut(B, Self::Item) -> B,
        {
            self.items.fold(init, f)
        }*/

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.items.size_hint()
        }
    }
}

mod data_resolve_v3 {
    use super::*;
    
    pub struct ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        pub index: usize, 
        pub level_indices: &'a [usize],
        pub iter: Iter,
    }
    impl<'a, Iter> Iterator for ResolveIter<'a, Iter>
    where
        Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    {
        type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;
    
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            if let Some(array) = self.iter.next(){
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                Some(unsafe{ array.as_ref().data_unchecked(self.index, self.level_indices) })
            } else {
                None
            }
        }
    }
}

struct ResolveIterUnchecked<'a, Iter> {
    index: usize, 
    level_indices: &'a [usize],
    iter: Iter,
}
impl<'a, Iter> Iterator for ResolveIterUnchecked<'a, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
{
    type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|array| unsafe {
                // TODO: reuse as fn?
                let array = NonNull::from(array.borrow()); // drop borrow lifetime
                array.as_ref().data_unchecked(self.index, self.level_indices)
            })
    }

    #[inline]
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.iter.fold(init, |init, array| unsafe {
            let array = NonNull::from(array.borrow()); // drop borrow lifetime
            let data = array.as_ref().data_unchecked(self.index, self.level_indices);
            f(init, data)
        })
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

const N: usize = 32;
type StatesItem<Iter> = (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy>::State);

pub struct MultiIntersectionState<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
{
    states: ArrayVec<
        (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy>::State),
        N
    >,    
    empty_below_n: usize,
    terminal_node_mask: <IterItem<Iter> as SparseHierarchy>::LevelMaskType,
    phantom_data: PhantomData<(Iter, F, T)>
}

impl<Iter, F, T> SparseHierarchyState for MultiIntersectionState<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
    for<'a> F: Fn(MultiIntersectionResolveIter<'a, Iter>) -> T
{
    type This = MultiIntersection<Iter, F, T>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        let states = ArrayVec::from_iter(
            this.iter.clone()
                .map(|array|{
                    let state = SparseHierarchyState::new(array.borrow()); 
                    (array, state)
                })
        );
        
        Self {
            states,
            empty_below_n: usize::MAX,
            terminal_node_mask: BitBlock::zero(),
            phantom_data: PhantomData,
        }        
    }

    #[inline]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
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
        
        /*const*/ if N::VALUE == <Self::This as SparseHierarchy>::LevelCount::VALUE - 1 {
            self.terminal_node_mask = acc_mask.clone(); 
        }
        
        acc_mask
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger> (
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
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
        -> Option<<Self::This as SparseHierarchy>::Data<'a>> 
    {
        if !self.terminal_node_mask.get_bit(level_index){
            return None;
        }
        
        Some(self.data_unchecked(this, level_index))
    }

    #[inline]
    unsafe fn data_unchecked<'a>(
        &self, this: &'a Self::This, level_index: usize
    ) -> <Self::This as SparseHierarchy>::Data<'a> {
        /*
        // Store data in ArrayVec. That would let to use the same iter with &[Data] 
        // for all cases.
        // More than 100% (x2 times) performance drop.
        {
            let mut datas: ArrayVec<_, N> = Default::default();
            
            let level_index = level_index;
            for (array, array_state) in self.states.iter() {
                let data = unsafe{ array_state.data_unchecked(array.borrow(), level_index) };
                datas.push_unchecked(data);
            }
            
            (this.f)(
                MultiIntersectionResolveIter::stateless(
                    ResolveIter { items: datas.into_iter() }
                )
            )
        }
        */
        
        (this.f)(
            MultiIntersectionResolveIter::statefull(
                StateResolveIter { level_index, states_iter: self.states.iter() }
            )
        )
    }
}

// States slice to Data iterator adapter.
struct StateResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{
    level_index: usize,
    states_iter: slice::Iter<'a, StatesItem<I>>
}

/// Iterator for [MultiIntersection] resolve function.
/// 
/// Prefer using [fold]-based[^1] operations over [next]-ing.
///
/// [^1]: Such as [for_each], [sum], etc... 
impl<'a, I> Iterator for StateResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{
    /// <I::Item as SparseHierarchy>::Data<'a>
    type Item = <IterItem<I> as SparseHierarchy>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Compiler optimizes away additional branching here.
        self.states_iter
            .next()
            .map(|(array, array_state)| unsafe { 
                array_state.data_unchecked(array.borrow(), self.level_index)
            })
    }

    #[inline]
    fn fold<B, F>(self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let level_index = self.level_index;
        for (array, array_state) in self.states_iter {
            let data = unsafe{ array_state.data_unchecked(array.borrow(), level_index) };
            init = f(init, data);
        }
        init
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.states_iter.size_hint()
    }
}

impl<'a, I> ExactSizeIterator for StateResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{}

impl<Iter, Init, F> LazySparseHierarchy for MultiIntersection<Iter, Init, F>
where
    MultiIntersection<Iter, Init, F>: SparseHierarchy
{}

impl<Iter, Init, F> Borrowable for MultiIntersection<Iter, Init, F>{ type Borrowed = Self; }

#[inline]
pub fn multi_intersection<Iter, F, T>(iter: Iter, resolve: F) 
    -> MultiIntersection<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
    for<'a> F: Fn(MultiIntersectionResolveIter<'a, Iter>) -> T
{
    MultiIntersection{ iter, f: resolve, phantom_data: Default::default() }
}

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use crate::compact_sparse_array::CompactSparseArray;
    use crate::ops::multi_intersection2::multi_intersection;
    use crate::sparse_hierarchy::SparseHierarchy;

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
        
        let intersection = multi_intersection(arrays.iter(), |vs| vs.sum() ); 
        
        assert_equal(intersection.iter(), [(15, 45)]);
        assert_eq!(unsafe{ intersection.get_unchecked(15) }, 45);
        assert_eq!(unsafe{ intersection.get(15) }, Some(45));
        assert_eq!(unsafe{ intersection.get(200) }, None);
    }

}
