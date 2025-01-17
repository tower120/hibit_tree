use std::marker::PhantomData;
use std::ptr::NonNull;
use std::slice;
use arrayvec::ArrayVec;
use crate::{BitBlock, LazyHibitTree, RegularHibitTree, MultiHibitTree, MultiHibitTreeTypes, HibitTree, HibitTreeData, HibitTreeCursor, HibitTreeCursorTypes, HibitTreeTypes, HierarchyIndex, union};
use crate::const_utils::{ConstArrayType, ConstBool, ConstFalse, ConstInteger, ConstTrue, IsConstTrue};
use crate::utils::{Array, Borrowable, Ref};

pub struct MultiUnion<Iter, D=ConstFalse> {
    iter: Iter,
    phantom: PhantomData<D>,
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Ref>::Type;
type IterItemCursor<'item, Iter> = <IterItem<Iter> as HibitTreeTypes<'item>>::Cursor;

impl<'item, 'this, Iter, T, D> HibitTreeTypes<'this> for MultiUnion<Iter, D>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item,
    D: ConstBool,
{
    type Data  = Data<'item, Iter>;
    type DataUnchecked = DataUnchecked<Iter>;
    type DataOrDefault = DataOrDefault<Iter>;
    type Cursor = Cursor<'this, 'item, Iter, D>;
}

impl<'i, Iter, T, D> HibitTree for MultiUnion<Iter, D>
where
    Iter: Iterator<Item = &'i T> + Clone,
    T: HibitTree + 'i,
    D: ConstBool
{
    const EXACT_HIERARCHY: bool = T::EXACT_HIERARCHY;
    type DefaultData = T::DefaultData;
    
    type LevelCount = T::LevelCount;
    type LevelMask  = T::LevelMask;

    #[inline]
    fn data(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
    {
        // TODO: we can have custom iterator here, without gathering everything into array.
        // Gather items - then return as iter.
        let mut datas: ArrayVec<_, N> = Default::default();
        for array in self.iter.clone(){
            let data = unsafe{ array.borrow().data(index) };
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
    unsafe fn data_unchecked(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked 
    {
        DataUnchecked {
            iter: self.iter.clone(),
            hi_index: index.clone(),
        }
    }
    
    #[inline]
    unsafe fn data_or_default(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> <Self as HibitTreeTypes<'_>>::DataOrDefault 
    {
        DataOrDefault {
            iter: self.iter.clone(),
            hi_index: index.clone(),
        }
    }
}

pub type Data<'item, Iter> = arrayvec::IntoIter<<IterItem<Iter> as HibitTreeTypes<'item>>::Data, N>;

pub struct DataUnchecked<Iter>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>>,
{
    iter: Iter,
    hi_index: HierarchyIndex<
        <IterItem<Iter> as HibitTree>::LevelMask,
        <IterItem<Iter> as HibitTree>::LevelCount,
    >,
}
impl<'item, Iter, T> Iterator for DataUnchecked<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item,
{
    type Item = <T as HibitTreeTypes<'item>>::Data;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|array| unsafe{
            array.data(&self.hi_index)
        })
    }

    #[inline]
    fn fold<B, F>(self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        for array in self.iter {
            if let Some(item) = unsafe{array.data(&self.hi_index)} {
                init = f(init, item)    
            }
        }
        init
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.iter.size_hint().1)
    }
}

pub struct DataOrDefault<Iter>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>>,
{
    iter: Iter,
    hi_index: HierarchyIndex<
        <IterItem<Iter> as HibitTree>::LevelMask,
        <IterItem<Iter> as HibitTree>::LevelCount,
    >,
}
impl<'item, Iter, T> Iterator for DataOrDefault<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item,
{
    type Item = <T as HibitTreeTypes<'item>>::DataOrDefault;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|array| unsafe{
            array.data_or_default(&self.hi_index)
        })
    }

    #[inline]
    fn fold<B, F>(self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        for array in self.iter {
            let item = unsafe{array.data_or_default(&self.hi_index)};
            init = f(init, item);    
        }
        init
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}
impl<'item, Iter, T> ExactSizeIterator for DataOrDefault<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item
{}

// --- CURSOR ---

const N: usize = 32;
type CursorIndex = u8;
type CursorsItem<'item, Iter> = (<Iter as Iterator>::Item, IterItemCursor<'item, Iter>);

impl<'this, 'src, 'item, Iter, D> HibitTreeCursorTypes<'this> for Cursor<'src, 'item, Iter, D>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>> + Clone,
    D: ConstBool,
{
    type Data = CursorData<'this, 'item, Iter, ConstFalse>;
    type DataUnchecked = CursorData<'this, 'item, Iter, D>;
    type DataOrDefault = CursorData<'this, 'item, Iter, ConstTrue>;
}

pub struct Cursor<'src, 'item, Iter, D>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>> + Clone,
    D: ConstBool
{
    cursors: ArrayVec<CursorsItem<'item, Iter>, N>,
    
    /// [ArrayVec<usize, N>; Array::LevelCount - 1]
    /// 
    /// Root level skipped.
    lvls_non_empty_states: ConstArrayType<
        ArrayVec<CursorIndex, N>,
        <<IterItem<Iter> as HibitTree>::LevelCount as ConstInteger>::Dec,
    >,
    
    phantom_data: PhantomData<&'src MultiUnion<Iter, D>>
}

impl<'src, 'item, Iter, T, D> Cursor<'src, 'item, Iter, D>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item,
    D: ConstBool
{
    #[inline]
    unsafe fn make_cursor_data<Def: ConstBool>(&self, level_index: usize) 
        -> CursorData<'_, 'item, Iter, Def> 
    {
        if <<<Self as HibitTreeCursor>::Tree as HibitTree>::LevelCount as ConstInteger>::VALUE == 1 {
            todo!("TODO: compile-time special case for 1-level SparseHierarchy");
        }
        
        let lvl_non_empty_states = self.lvls_non_empty_states.as_ref()
                                   .last().unwrap_unchecked();
        
        CursorData {
            lvl_non_empty_states: lvl_non_empty_states.iter(),
            cursors: &self.cursors,
            level_index,
            phantom_data: PhantomData,
        }        
    }
}

impl<'src, 'item, Iter, T, D> HibitTreeCursor<'src> for Cursor<'src, 'item, Iter, D>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item,
    D: ConstBool
{
    type Tree = MultiUnion<Iter, D>;

    #[inline]
    fn new(src: &'src Self::Tree) -> Self {
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
    unsafe fn select_level_node<N: ConstInteger>(&mut self, src: &'src Self::Tree, level_n: N, level_index: usize) 
        -> <Self::Tree as HibitTree>::LevelMask 
    {
        let mut acc_mask = BitBlock::zero();
        
        if N::VALUE == 0 {
            for (array, array_cursor) in self.cursors.iter_mut() {
                let mask = array_cursor.select_level_node(array, level_n, level_index);
                acc_mask |= mask;
            }            
            return acc_mask;
        }
        
        // Work with pointers for `get_many`-like access. 
        let lvls_non_empty_states = self.lvls_non_empty_states.as_mut().as_mut_ptr();
        let lvl_non_empty_states  = &mut*lvls_non_empty_states.add(level_n.value()-1);
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
            let prev_lvl_non_empty_states = &*lvls_non_empty_states.add(level_n.value()-2);
            for i in prev_lvl_non_empty_states { foreach(*i) }
        }
        
        acc_mask
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(&mut self, src: &'src Self::Tree, level_n: N, level_index: usize) 
        -> <Self::Tree as HibitTree>::LevelMask 
    {
        // There is actually no unchecked version for union.
        self.select_level_node(src, level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, src: &'src Self::Tree, level_index: usize) 
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        if <<Self::Tree as HibitTree>::LevelCount as ConstInteger>::VALUE == 1 {
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
            phantom_data: PhantomData,
        })
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, src: &'src Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataUnchecked
    {
        self.make_cursor_data(level_index)
    }
    
    #[inline]
    unsafe fn data_or_default<'a>(&'a self, src: &'src Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataOrDefault 
    {
        self.make_cursor_data(level_index)
    }    
}

/// `D=true` will use [data_or_default] to return values. 
/// Otherwise, [data] will be used.
pub struct CursorData<'cursor, 'item, I, D>
where
    I: Iterator<Item: Ref<Type: HibitTree>>
{
    lvl_non_empty_states: slice::Iter<'cursor, CursorIndex>,
    cursors: &'cursor [CursorsItem<'item, I>],
    level_index: usize,
    phantom_data: PhantomData<D>
}

impl<'cursor, 'item, I, T, D> Iterator for CursorData<'cursor, 'item, I, D>
where
    I: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item,
    D: ConstBool
{
    type Item = <IterItemCursor<'item, I> as HibitTreeCursorTypes<'cursor>>::Data;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if D::VALUE {
            self.lvl_non_empty_states.next().map(|&i| unsafe{
                let (array, array_cursor) = self.cursors.get_unchecked(i as usize);
                array_cursor.data_or_default(array, self.level_index)
            })
        } else {
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
            if D::VALUE {
                let data = unsafe{ array_cursor.data_or_default(array, self.level_index) };
                init = f(init, data);
            } else {
                if let Some(data) = unsafe{ array_cursor.data(array, level_index) } {
                    init = f(init, data);
                }
            }
        }
        init
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.lvl_non_empty_states.len();
        if D::VALUE{
            (len, Some(len))
        } else {
            (0, Some(len))
        }
    }
}

impl<'cursor, 'item, I, T, D> ExactSizeIterator for CursorData<'cursor, 'item, I, D>
where
    I: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item,
    D: IsConstTrue
{}

impl<'item, Iter, T, D> LazyHibitTree for MultiUnion<Iter, D>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item,
    D: ConstBool
{}

impl<'item, 'this, Iter, T, D> MultiHibitTreeTypes<'this> for MultiUnion<Iter, D>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item,
    D: ConstBool
{ 
    type IterItem = HibitTreeData<'item, T>; 
}

impl<'item, Iter, T, D> MultiHibitTree for MultiUnion<Iter, D>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item,
    D: ConstBool
{}

impl<Iter, D> Borrowable for MultiUnion<Iter, D>{ type Borrowed = Self; }

/// Union between multiple &[RegularHibitTree]s.
/// 
/// `iter` will be cloned and iterated multiple times.
/// Pass something like [slice::Iter].
#[inline]
pub fn multi_union<Iter>(iter: Iter) 
    -> MultiUnion<Iter>
where
    Iter: Iterator<Item: Ref<Type: RegularHibitTree>> + Clone,
{
    MultiUnion{ iter, phantom: Default::default() }
}

/// Same as [multi_union] but iterator will use [data_or_default].
/// 
/// This can lead to a faster code, since iterator does not have to 
/// skip values during iteration.
/// 
/// Default values MAY appear in iterator output.
#[inline]
pub fn multi_union_w_default<Iter>(iter: Iter) 
    -> MultiUnion<Iter, ConstTrue>
where
    Iter: Iterator<Item: Ref<Type: RegularHibitTree<DefaultData: IsConstTrue>>> + Clone,
{
    MultiUnion{ iter, phantom: Default::default() }
}

#[cfg(test)]
mod tests{
    use super::*;
    use itertools::assert_equal;
    use crate::hibit_tree::HibitTree;
    use crate::ReqDefault;
    use crate::tree::Config64bit;
    use crate::utils::LendingIterator;
    
    type Array = crate::tree::Tree<usize, Config64bit<3>, ReqDefault>;

    #[test]
    fn multi_union_test(){
        let mut a1 = Array::default();
        let mut a2 = Array::default();
        let mut a3 = Array::default();
        
        a1.insert(10, 10);
        a1.insert(15, 15);
        a1.insert(200, 200);
        
        a2.insert(100, 100);
        a2.insert(15, 15);
        a2.insert(200, 200);
        
        a3.insert(300, 300);
        a3.insert(15, 15);
        
        let arrays = [a1, a2, a3];

        let union = multi_union( arrays.iter() ); 
        
        // iter test
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

        // get test
        assert_equal( 
            union.get(10).unwrap(),
            vec![arrays[0].get(10).unwrap()]
        );
        assert_equal( 
            union.get(15).unwrap(),
            vec![arrays[0].get(15).unwrap(), arrays[1].get(15).unwrap(), arrays[2].get(15).unwrap()]
        );
        assert!(union.get(25).is_none());
        
        // get_unchecked test
        assert_equal(unsafe{ union.get_unchecked(10) }, union.get(10).unwrap());
        assert_equal(unsafe{ union.get_unchecked(15) }, union.get(15).unwrap());
        
        // get_or_default test
        assert_equal( 
            union.get_or_default(10),
            vec![arrays[0].get(10).unwrap(), &0, &0]
        );
    }
    
    #[test]
    fn multi_union_w_default_test(){
        let mut a1 = Array::default();
        let mut a2 = Array::default();
        let mut a3 = Array::default();
        
        // All indices must be in the same terminal block for this test.
        // Otherwise - they will not appear as default values in iterator output.
        a1.insert(1, 1);
        a1.insert(15, 15);
        a1.insert(20, 20);
        
        a2.insert(10, 10);
        a2.insert(15, 15);
        a2.insert(20, 20);
        
        a3.insert(30, 30);
        a3.insert(15, 15);
        
        let arrays = [a1, a2, a3];
        
        let union = multi_union_w_default( arrays.iter() );
        
        // iter test
        let mut v = Vec::new();
        let mut iter = union.iter();
        while let Some((index, values)) = iter.next(){
            let values: Vec<&usize> = values.collect();
            println!("{:?}", values);
            v.push(values);
        }
        assert_equal(v, vec![
            vec![arrays[0].get(1).unwrap(), &0, &0],
            vec![&0, arrays[1].get(10).unwrap(), &0],
            vec![
                arrays[0].get(15).unwrap(),
                arrays[1].get(15).unwrap(),
                arrays[2].get(15).unwrap(),
            ],
            vec![
                arrays[0].get(20).unwrap(),
                arrays[1].get(20).unwrap(),
                &0
            ],
            vec![&0, &0, arrays[2].get(30).unwrap()],
        ]);
    }    

}