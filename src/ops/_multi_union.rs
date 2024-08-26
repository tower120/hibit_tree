use std::marker::PhantomData;
use std::ptr::NonNull;
use std::slice;
use arrayvec::ArrayVec;
use crate::{BitBlock, LazyHibitTree, RegularHibitTree, MultiHibitTree, MultiHibitTreeTypes, HibitTree, HibitTreeData, HibitTreeCursor, HibitTreeCursorTypes, HibitTreeTypes};
use crate::const_utils::{ConstArrayType, ConstInteger};
use crate::utils::{Array, Borrowable, Ref};

pub struct MultiUnion<Iter> {
    iter: Iter
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Ref>::Type;
type IterItemCursor<'item, Iter> = <IterItem<Iter> as HibitTreeTypes<'item>>::Cursor;

impl<'item, 'this, Iter, T> HibitTreeTypes<'this> for MultiUnion<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item
{
    type Data  = Data<'item, Iter>;
    type DataUnchecked = DataUnchecked<Iter>;
    type Cursor = Cursor<'this, 'item, Iter>;
}

impl<'i, Iter, T> HibitTree for MultiUnion<Iter>
where
    Iter: Iterator<Item = &'i T> + Clone,
    T: HibitTree + 'i
{
    const EXACT_HIERARCHY: bool = T::EXACT_HIERARCHY;
    
    type LevelCount = T::LevelCount;
    type LevelMask  = T::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
    {
        // Gather items - then return as iter.
        let mut datas: ArrayVec<_, N> = Default::default();
        for array in self.iter.clone(){
            let array = NonNull::from(array.borrow()); // drop borrow lifetime
            let data = unsafe{ array.as_ref().data(index, level_indices) };
            if let Some(data) = data {
                datas.push(data);
            }
        }
        if datas.is_empty(){
            return None;
        }
        
        Some(datas.into_iter())
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize])
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked 
    {
        DataUnchecked {
            iter: self.iter.clone(),
            index,
            level_indices: Array::from_fn(|i| unsafe{ *level_indices.get_unchecked(i) }),
        }
    }
}

pub type Data<'item, Iter> = arrayvec::IntoIter<<IterItem<Iter> as HibitTreeTypes<'item>>::Data, N>;

pub struct DataUnchecked<Iter>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>>,
{
    iter: Iter,
    index: usize, 
    // This is copy from level_indices &[usize]. 
    // Compiler optimize away the very act of cloning and directly use &[usize].
    // At least, if value used immediately, and not stored for latter use. 
    level_indices: ConstArrayType<usize, <IterItem<Iter> as HibitTree>::LevelCount>,
}
impl<'item, Iter, T> Iterator for DataUnchecked<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item,
{
    type Item = </*IterItem<Iter>*/T as HibitTreeTypes<'item>>::Data;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|array|{
            unsafe{
                array.data(self.index, self.level_indices.as_ref())
            }
        })
    }

    #[inline]
    fn fold<B, F>(self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        for array in self.iter {
            unsafe{
                if let Some(item) = array.data(self.index, self.level_indices.as_ref()){
                    init = f(init, item)    
                }
            }
        }
        init
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.iter.size_hint().1)
    }
}

const N: usize = 32;
type CursorIndex = u8;
type CursorsItem<'item, Iter> = (<Iter as Iterator>::Item, IterItemCursor<'item, Iter>);

pub struct Cursor<'src, 'item, Iter>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>> + Clone,
{
    cursors: ArrayVec<CursorsItem<'item, Iter>, N>,
    
    /// [ArrayVec<usize, N>; Array::LevelCount - 1]
    /// 
    /// Root level skipped.
    lvls_non_empty_states: ConstArrayType<
        ArrayVec<CursorIndex, N>,
        <<IterItem<Iter> as HibitTree>::LevelCount as ConstInteger>::Dec,
    >,
    
    phantom_data: PhantomData<&'src MultiUnion<Iter>>
}

impl<'this, 'src, 'item, Iter> HibitTreeCursorTypes<'this> for Cursor<'src, 'item, Iter>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>> + Clone
{
    type Data = CursorData<'this, 'item, Iter>;
}

impl<'src, 'item, Iter, T> HibitTreeCursor<'src> for Cursor<'src, 'item, Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item
{
    type Src = MultiUnion<Iter>;

    #[inline]
    fn new(src: &'src Self::Src) -> Self {
        let states = ArrayVec::from_iter(
            src.iter.clone()
                .map(|array|{
                    let state = HibitTreeCursor::new(array.borrow()); 
                    (array, state)
                })
        );
        
        Self {
            cursors: states,
            lvls_non_empty_states: Array::from_fn(|_|ArrayVec::new()),
            phantom_data: PhantomData,
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(&mut self, src: &'src Self::Src, level_n: N, level_index: usize) 
        -> <Self::Src as HibitTree>::LevelMask 
    {
        // unchecked version already deal with non-existent elements
        self.select_level_node_unchecked(src, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(&mut self, src: &'src Self::Src, level_n: N, level_index: usize) 
        -> <Self::Src as HibitTree>::LevelMask 
    {
        let mut acc_mask = BitBlock::zero();
        
        if N::VALUE == 0 {
            for (array, array_cursor) in self.cursors.iter_mut() {
                let mask = array_cursor.select_level_node(array, level_n, level_index);
                acc_mask |= mask;
            }            
            return acc_mask;
        }
        
        // drop lifetime checks for `get_many`-like access. 
        let mut lvls_non_empty_states = NonNull::from(self.lvls_non_empty_states.as_mut());
        
        let lvl_non_empty_states = 
            lvls_non_empty_states.as_mut().get_unchecked_mut(level_n.value()-1);
        lvl_non_empty_states.clear();
        
        let len = self.cursors.len() as u8;
        
        let mut foreach = |i: CursorIndex| {
            let (array, array_cursor) = self.cursors.get_unchecked_mut(i as usize);
            let mask = array_cursor.select_level_node(array, level_n, level_index);
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
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        if <Self::Src as HibitTree>::LevelCount::VALUE == 1 {
            todo!("TODO: compile-time special case for 1-level SparseHierarchy");
        }
        
        let lvl_non_empty_states = self.lvls_non_empty_states.as_ref()
                                   .last().unwrap_unchecked();
        if lvl_non_empty_states.is_empty(){
            return None;
        }
        
        Some(CursorData {
            lvl_non_empty_states: lvl_non_empty_states.iter(),
            cursors: &self.cursors,
            level_index,
        })
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, src: &'src Self::Src, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::Data 
    {
        self.data(src, level_index).unwrap_unchecked()
    }
}

pub struct CursorData<'cursor, 'item, I>
where
    I: Iterator<Item: Ref<Type: HibitTree>>
{
    lvl_non_empty_states: slice::Iter<'cursor, CursorIndex>,
    cursors: &'cursor [CursorsItem<'item, I>],
    level_index: usize,
}

impl<'cursor, 'item, I, T> Iterator for CursorData<'cursor, 'item, I>
where
    I: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item
{
    /// <I::Item as SparseHierarchy2>::Data<'a>
    type Item = <IterItemCursor<'item, I> as HibitTreeCursorTypes<'cursor>>::Data;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.lvl_non_empty_states
            .find_map(|&i| unsafe {
                let (array, array_cursor) = self.cursors.get_unchecked(i as usize);
                if let Some(data) = array_cursor.data(array, self.level_index) {
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
            let (array, array_cursor) = unsafe{ self.cursors.get_unchecked(i as usize) };
            if let Some(data) = unsafe{ array_cursor.data(array, level_index) } {
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

impl<Iter> LazyHibitTree for MultiUnion<Iter>
where
    MultiUnion<Iter>: HibitTree
{}

impl<'item, 'this, Iter, T> MultiHibitTreeTypes<'this> for MultiUnion<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item
{ 
    type IterItem = HibitTreeData<'item, T>; 
}

impl<'item, Iter, T> MultiHibitTree for MultiUnion<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item
{}

impl<Iter> Borrowable for MultiUnion<Iter>{ type Borrowed = Self; }

/// Union between multiple &[HibitTree]s.
/// 
/// `iter` will be cloned and iterated multiple times.
/// Pass something like [slice::Iter].
#[inline]
pub fn multi_union<Iter>(iter: Iter) 
    -> MultiUnion<Iter>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>> + Clone,
{
    MultiUnion{ iter }
}

#[cfg(test)]
mod tests{
    use super::*;
    use itertools::assert_equal;
    use crate::dense_tree::DenseTree;
    use crate::hibit_tree::HibitTree;
    use crate::utils::LendingIterator;

    #[test]
    fn smoke_test(){
        type Array = DenseTree<usize, 3>;
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

        assert_equal( 
            union.get(10).unwrap(),
            vec![arrays[0].get(10).unwrap()]
        );
        
        assert_equal( 
            union.get(15).unwrap(),
            vec![arrays[0].get(15).unwrap(), arrays[1].get(15).unwrap(), arrays[2].get(15).unwrap()]
        );
        
        assert!(union.get(25).is_none());
        
        assert_equal(unsafe{ union.get_unchecked(10) }, union.get(10).unwrap());
        assert_equal(unsafe{ union.get_unchecked(15) }, union.get(15).unwrap());
    }

}