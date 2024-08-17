use std::marker::PhantomData;
use std::ptr::NonNull;
use std::slice;
use arrayvec::ArrayVec;
use rand::distributions::uniform::SampleBorrow;
use crate::{BitBlock, SparseHierarchy, SparseHierarchyState, SparseHierarchyStateTypes, SparseHierarchyTypes};
use crate::const_utils::{ConstArrayType, ConstInteger};
use crate::utils::{Array, Borrowable, Ref};

pub struct MultiUnion<Iter> {
    iter: Iter
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Ref>::Type;
type IterItemState<'item, Iter> = <IterItem<Iter> as SparseHierarchyTypes<'item>>::State;

impl<'item, 'this, Iter, T> SparseHierarchyTypes<'this> for MultiUnion<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item
{
    type Data  = /*ResolveIter<'item, Iter>*/();
    type DataUnchecked = /*ResolveIterUnchecked<Iter>*/();
    type State = MultiUnionState<'this, 'item, Iter>;
}

impl<'i, Iter, T> SparseHierarchy for MultiUnion<Iter>
where
    Iter: Iterator<Item = &'i T> + Clone,
    T: SparseHierarchy + 'i
{
    const EXACT_HIERARCHY: bool = T::EXACT_HIERARCHY;
    
    type LevelCount = T::LevelCount;
    type LevelMask  = T::LevelMask;

    unsafe fn data(&self, index: usize, level_indices: &[usize]) -> Option<<Self as SparseHierarchyTypes<'_>>::Data> {
        todo!()
    }

    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) -> <Self as SparseHierarchyTypes<'_>>::DataUnchecked {
        todo!()
    }
}

const N: usize = 32;
type StateIndex = u8;
type StatesItem<'item, Iter> = (<Iter as Iterator>::Item, IterItemState<'item, Iter>);

pub struct MultiUnionState<'src, 'item, Iter>
where
    Iter: Iterator<Item: Ref<Type: SparseHierarchy>> + Clone,
{
    states: ArrayVec<StatesItem<'item, Iter>, N>,
    
    /// [ArrayVec<usize, N>; Array::LevelCount - 1]
    /// 
    /// Root level skipped.
    lvls_non_empty_states: ConstArrayType<
        ArrayVec<StateIndex, N>,
        <<IterItem<Iter> as SparseHierarchy>::LevelCount as ConstInteger>::Dec,
    >,
    
    phantom_data: PhantomData<&'src MultiUnion<Iter>>
}

impl<'this, 'src, 'item, Iter> SparseHierarchyStateTypes<'this> for MultiUnionState<'src, 'item, Iter>
where
    Iter: Iterator<Item: Ref<Type: SparseHierarchy>> + Clone
{
    type Data = StateDataIter<'this, 'item, Iter>;
}

impl<'src, 'item, Iter, T> SparseHierarchyState<'src> for MultiUnionState<'src, 'item, Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item
{
    type Src = MultiUnion<Iter>;

    #[inline]
    fn new(src: &'src Self::Src) -> Self {
        let states = ArrayVec::from_iter(
            src.iter.clone()
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
    unsafe fn select_level_node<N: ConstInteger>(&mut self, src: &'src Self::Src, level_n: N, level_index: usize) 
        -> <Self::Src as SparseHierarchy>::LevelMask 
    {
        // unchecked version already deal with non-existent elements
        self.select_level_node_unchecked(src, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(&mut self, src: &'src Self::Src, level_n: N, level_index: usize) 
        -> <Self::Src as SparseHierarchy>::LevelMask 
    {
        let mut acc_mask = BitBlock::zero();
        
        if N::VALUE == 0 {
            for (array, array_state) in self.states.iter_mut() {
                let mask = array_state.select_level_node(array, level_n, level_index);
                acc_mask |= mask;
            }            
            return acc_mask;
        }
        
        // drop lifetime checks for `get_many`-like access. 
        let mut lvls_non_empty_states = NonNull::from(self.lvls_non_empty_states.as_mut());
        
        let lvl_non_empty_states = 
            lvls_non_empty_states.as_mut().get_unchecked_mut(level_n.value()-1);
        lvl_non_empty_states.clear();
        
        let len = self.states.len() as u8;
        
        let mut foreach = |i: StateIndex| {
            let (array, array_state) = self.states.get_unchecked_mut(i as usize);
            let mask = array_state.select_level_node(array, level_n, level_index);
            if !mask.is_zero() {
                lvl_non_empty_states.push_unchecked(i);
            }
            acc_mask |= mask;            
        };
        
        if N::VALUE == 1 {
            // Prev level is root. Since we don't store root - 
            // just iterate all states.
            for i in 0..len { foreach(i) }    
        } else {
            let prev_lvl_non_empty_states =
                lvls_non_empty_states.as_ref().get_unchecked(level_n.value()-2);
            for i in prev_lvl_non_empty_states { foreach(*i) }
        }
        
        acc_mask
    }

    #[inline]
    unsafe fn data<'a>(&'a self, src: &'src Self::Src, level_index: usize) 
        -> Option<<Self as SparseHierarchyStateTypes<'a>>::Data> 
    {
        if <Self::Src as SparseHierarchy>::LevelCount::VALUE == 1 {
            todo!("TODO: compile-time special case for 1-level SparseHierarchy");
        }
        
        let lvl_non_empty_states = self.lvls_non_empty_states.as_ref()
                                   .last().unwrap_unchecked();
        if lvl_non_empty_states.is_empty(){
            return None;
        }
        
        Some(StateDataIter {
            lvl_non_empty_states: lvl_non_empty_states.iter(),
            states: &self.states,
            level_index,
        })
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, src: &'src Self::Src, level_index: usize) 
        -> <Self as SparseHierarchyStateTypes<'a>>::Data 
    {
        self.data(src, level_index).unwrap_unchecked()
    }
}

pub struct StateDataIter<'state, 'item, I>
where
    I: Iterator<Item: Ref<Type: SparseHierarchy>>
{
    lvl_non_empty_states: slice::Iter<'state, StateIndex>,
    states: &'state [StatesItem<'item, I>],
    level_index: usize,
}

impl<'state, 'item, I, T> Iterator for StateDataIter<'state, 'item, I>
where
    I: Iterator<Item = &'item T> + Clone,
    T: SparseHierarchy + 'item
{
    /// <I::Item as SparseHierarchy2>::Data<'a>
    type Item = <IterItemState<'item, I> as SparseHierarchyStateTypes<'state>>::Data;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.lvl_non_empty_states
            .find_map(|&i| unsafe {
                let (array, array_state) = self.states.get_unchecked(i as usize);
                if let Some(data) = array_state.data(array, self.level_index) {
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
            if let Some(data) = unsafe{ array_state.data(array, level_index) } {
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

impl<Iter> Borrowable for MultiUnion<Iter>{ type Borrowed = Self; }

#[inline]
pub fn multi_union<Iter>(iter: Iter) 
    -> MultiUnion<Iter>
where
    Iter: Iterator<Item: Ref<Type:SparseHierarchy>> + Clone,
{
    MultiUnion{ iter }
}

#[cfg(test)]
mod tests{
    use super::*;
    use itertools::assert_equal;
    use crate::compact_sparse_array::CompactSparseArray;
    use crate::sparse_hierarchy::SparseHierarchy;
    use crate::utils::LendingIterator;

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
        
        let union = multi_union( arrays.iter() ); 
        
        let mut v = Vec::new();
        let mut iter = union.iter();
        while let Some((index, values)) = iter.next(){
            let values: Vec<&usize> = values.collect();
            println!("{:?}", values);
            v.push(values);
        }
        
        assert_equal(v, vec![
            vec![arrays[0].get(10).unwrap()],
            vec![
                arrays[0].get(15).unwrap(),
                arrays[1].get(15).unwrap(),
                arrays[2].get(15).unwrap(),
            ],
            vec![arrays[1].get(100).unwrap()],
            vec![
                arrays[0].get(200).unwrap(),
                arrays[1].get(200).unwrap(),
            ],
            vec![arrays[2].get(300).unwrap()],
        ]);
        
        /*assert_eq!(unsafe{union.get_unchecked(10)}, 10);
        assert_eq!(unsafe{union.get_unchecked(15)}, 45);
        assert_eq!(unsafe{union.get(15)}, Some(45));
        assert_eq!(unsafe{union.get(25)}, None);*/
    }

}