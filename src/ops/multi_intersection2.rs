use std::marker::PhantomData;
use std::borrow::Borrow;
use std::ptr::NonNull;
use std::slice;
use arrayvec::ArrayVec;
use crate::BitBlock;
use crate::const_utils::{ConstArray, ConstCopyArrayType, ConstInteger};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::utils::{Array, Borrowable, Take};

type LevelIndices<S> = ConstCopyArrayType<usize, <S as SparseHierarchy>::LevelCount>;

// HOPEFULLY this always acts as one of concrete iterator variants,
// without additional branching. At least small tests in godbolt show so.
// (because we construct and immoderately consume iterator in resolve closure)
//
// Ideally, we would need to pass concrete iterators to closure, but that would
// require generic closures.
pub enum AnyResolveIter<'a, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{
    stateless(ResolveIter<'a, LevelIndices<IterItem<Iter>>, Iter>),
    statefull(MultiIntersectionResolveIter<'a, Iter>)
}
impl<'a, Iter> Iterator for AnyResolveIter<'a, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
{
    type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self{
            AnyResolveIter::stateless(iter) => iter.next(),
            AnyResolveIter::statefull(iter) => iter.next(),
        }
    }
}
// TODO: size_hint, fold, ExactSize

pub struct MultiIntersection<Iter, F, T> {
    iter: Iter,
    f: F,
    phantom_data: PhantomData<T>
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Borrowable>::Borrowed;

impl<Iter, F, T> SparseHierarchy for MultiIntersection<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
    for<'a> F: Fn(AnyResolveIter<'a, Iter>) -> T
{
    const EXACT_HIERARCHY: bool = false;
    
    type LevelCount = <IterItem<Iter> as SparseHierarchy>::LevelCount;

    type LevelMaskType = <IterItem<Iter> as SparseHierarchy>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self: 'a;
    
    type DataType = T;
    type Data<'a> = T where Self: 'a;

    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<Self::Data<'_>> 
    {
        todo!()
    }

    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> Self::Data<'_> 
    {
        todo!()
        /*(self.f)(
            AnyResolveIter::stateless(
                ResolveIter{
                    index, 
                    level_indices, 
                    iter: self.iter.clone(), 
                    phantom_data: PhantomData
                }
            )
        )*/
    }

    type State = MultiIntersectionState<Iter, F, T>;
}

pub struct ResolveIter<'a, Indices, Iter> {
    index: usize, 
    level_indices: Indices,
    iter: Iter,
    phantom_data: PhantomData<&'a ()>
}
impl<'a, Indices, Iter> Iterator for ResolveIter<'a, Indices, Iter>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + 'a,
    Indices: ConstArray<Item=usize, Cap=<IterItem<Iter> as SparseHierarchy>::LevelCount> + Copy
{
    type Item = <IterItem<Iter> as SparseHierarchy>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|array| unsafe {
                // drop borrow lifetime
                let array = NonNull::from(array.borrow());
                array.as_ref().data_unchecked(self.index, self.level_indices)
            })
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
    for<'a> F: Fn(AnyResolveIter<'a, Iter>) -> T
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
        (this.f)(
            AnyResolveIter::statefull(
                MultiIntersectionResolveIter { level_index, states_iter: self.states.iter() }
            )
        )
    }
}

// States slice to Data iterator adapter.
pub struct MultiIntersectionResolveIter<'a, I>
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
impl<'a, I> Iterator for MultiIntersectionResolveIter<'a, I>
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

impl<'a, I> ExactSizeIterator for MultiIntersectionResolveIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>
{}

impl<Iter, Init, F> Borrowable for MultiIntersection<Iter, Init, F>{ type Borrowed = Self; }

#[inline]
pub fn multi_intersection<Iter, F, T>(iter: Iter, resolve: F) 
    -> MultiIntersection<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>> + Clone,
    for<'a> F: Fn(AnyResolveIter<'a, Iter>) -> T
{
    MultiIntersection{ iter, f: resolve, phantom_data: Default::default() }
}

#[cfg(test)]
mod test{
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
    }

}