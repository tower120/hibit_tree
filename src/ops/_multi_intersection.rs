use std::marker::PhantomData;
use std::borrow::Borrow;
use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::slice;
use arrayvec::ArrayVec;
use crate::{BitBlock, LazyHibitTree, RegularHibitTree, MultiHibitTree, MultiHibitTreeTypes, HibitTreeData, HibitTreeCursorTypes, HibitTreeTypes};
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::hibit_tree::{HibitTree, HibitTreeCursor};
use crate::utils::{Array, Borrowable, Ref};

/// Intersection between all iterator items.
///
/// All data iterators are [ExactSizeIterator]. 
pub struct MultiIntersection<Iter> {
    iter: Iter,
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Ref>::Type;
type IterItemCursor<'item, Iter> = <IterItem<Iter> as HibitTreeTypes<'item>>::Cursor;

impl<'item, 'this, Iter, T> HibitTreeTypes<'this> for MultiIntersection<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item
{
    type Data  = Data<'item, Iter>;
    type DataUnchecked = DataUnchecked<Iter>;
    type Cursor = Cursor<'this, 'item, Iter>;
}

impl<'i, Iter, T> HibitTree for MultiIntersection<Iter>
where
    Iter: Iterator<Item = &'i T> + Clone,
    T: HibitTree + 'i
{
    const EXACT_HIERARCHY: bool = false;
    
    type LevelCount = T::LevelCount;
    type LevelMask  = T::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
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
                //     SparseHierarchy<Data:'static>
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
            
            Some(datas.into_iter())
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
    unsafe fn data_unchecked<'a>(&'a self, index: usize, level_indices: &'a [usize]) 
        -> <Self as HibitTreeTypes<'a>>::DataUnchecked
    {
        DataUnchecked {
            index, 
            level_indices: Array::from_fn(|i| unsafe{ *level_indices.get_unchecked(i) }), 
            iter: self.iter.clone(),
        }
    }
}

pub type Data<'item, Iter> = arrayvec::IntoIter<<IterItem<Iter> as HibitTreeTypes<'item>>::Data, N>; 

/*use data_resolve_v2::ResolveIter;

/*mod data_resolve_v1 {
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
*/
mod data_resolve_v2 {
    use super::*;
    
    // Theoretically we could "somehow" extract 'item lifetime from Iter.
    // But for the sake of sanity - we just pass it.
    pub struct ResolveIter<'item, Iter>
    where
        Iter: Iterator<Item: Ref<Type: SparseHierarchy>>,
    {
        pub items: arrayvec::IntoIter<<IterItem<Iter> as SparseHierarchyTypes<'item>>::Data, N>
    }
    impl<'item, Iter> Iterator for ResolveIter<'item, Iter>
    where
        Iter: Iterator<Item: Ref<Type: SparseHierarchy>>,
    {
        type Item = <IterItem<Iter> as SparseHierarchyTypes<'item>>::Data;
    
        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            self.items.next()
        }

        #[inline]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.items.size_hint()
        }
    }
    
    impl<'item, Iter> ExactSizeIterator for ResolveIter<'item, Iter>
    where
        Iter: Iterator<Item: Ref<Type: SparseHierarchy>>
    {}
}*/

/*mod data_resolve_v3 {
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
*/

pub struct DataUnchecked<Iter> 
where
    Iter: Iterator<Item: Ref<Type: HibitTree>>,
{
    index: usize, 
    // This is copy from level_indices &[usize]. 
    // Compiler optimize away the very act of cloning and directly use &[usize].
    // At least, if value used immediately, and not stored for latter use. 
    level_indices: ConstArrayType<usize, <IterItem<Iter> as HibitTree>::LevelCount>,
    iter: Iter,
}
impl<'item, Iter, T> Iterator for DataUnchecked<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item,
{
    type Item = </*IterItem<Iter>*/T as HibitTreeTypes<'item>>::DataUnchecked;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|array| unsafe {
                array.data_unchecked(self.index, self.level_indices.as_ref())
            })
    }

    #[inline]
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        self.iter.fold(init, |init, array| unsafe {
            let data = array.data_unchecked(self.index, self.level_indices.as_ref());
            f(init, data)
        })
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'item, Iter, T> ExactSizeIterator for DataUnchecked<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item,
{}

const N: usize = 32;
type CursorsItem<'item, Iter> = IterItemCursor<'item, Iter>; 

pub struct Cursor<'src, 'item, I>
where
    I: Iterator<Item: Ref<Type: HibitTree>>
{
    cursors: ArrayVec<CursorsItem<'item, I>, N>,    
    empty_below_n: usize,
    terminal_node_mask: <IterItem<I> as HibitTree>::LevelMask,
    phantom_data: PhantomData<&'src MultiIntersection<I>>
}

impl<'this, 'src, 'item, Iter> HibitTreeCursorTypes<'this> for Cursor<'src, 'item, Iter>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>>
{
    type Data = CursorData<'this, 'item, Iter>;
}

impl<'src, 'item, Iter, T> HibitTreeCursor<'src> for Cursor<'src, 'item, Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item
{
    type Src = MultiIntersection<Iter>;

    #[inline]
    fn new(src: &'src Self::Src) -> Self {
        let cursors = ArrayVec::from_iter(
            src.iter.clone()
                .map(|array|{
                    HibitTreeCursor::new(array)
                })
        );
        
        Self {
            cursors,
            empty_below_n: usize::MAX,
            terminal_node_mask: BitBlock::zero(),
            phantom_data: PhantomData,
        }        
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as HibitTree>::LevelMask {
        // if we know that upper levels returned empty - return early.
        if N > self.empty_below_n {
            return BitBlock::zero(); 
        }
        
        let mut cursors_iter = self.cursors.iter_mut();
        let mut array_iter  = src.iter.clone();
        
        let mut acc_mask = 
            if let Some(array_cursor) = cursors_iter.next(){
                let array = array_iter.next().unwrap_unchecked();
                array_cursor.select_level_node(array, level_n, level_index)
            } else {
                return BitBlock::zero();
            };
        
        for array_cursor in cursors_iter {
            let array = array_iter.next().unwrap_unchecked();
            let mask = array_cursor.select_level_node(
                array, level_n, level_index
            );
            acc_mask &= mask;
        }
        
        self.empty_below_n = if acc_mask.is_zero(){
             N
        } else {
            usize::MAX
        };
        
        /*const*/ if N::VALUE == <Self::Src as HibitTree>::LevelCount::VALUE - 1 {
            self.terminal_node_mask = acc_mask.clone(); 
        }
        
        acc_mask
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger> (
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as HibitTree>::LevelMask {
        // TODO: Almost the same as in checked version. Reuse somehow. 
        let mut cursors_iter = self.cursors.iter_mut();
        let mut array_iter  = src.iter.clone();
        
        let mut acc_mask = 
            if let Some(array_cursor) = cursors_iter.next() {
                let array = array_iter.next().unwrap_unchecked();
                array_cursor.select_level_node_unchecked(array, level_n, level_index)
            } else {
                return BitBlock::zero();
            };
        
        for array_cursor in cursors_iter {
            let array = array_iter.next().unwrap_unchecked();
            let mask = array_cursor.select_level_node_unchecked(
                array, level_n, level_index
            );
            acc_mask &= mask;
        }            
        
        acc_mask
    }

    #[inline]
    unsafe fn data<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        if !self.terminal_node_mask.get_bit(level_index){
            return None;
        }
        
        Some(self.data_unchecked(this, level_index))
    }

    #[inline]
    unsafe fn data_unchecked<'a>(
        &'a self, src: &'src Self::Src, level_index: usize
    ) -> <Self as HibitTreeCursorTypes<'a>>::Data {
        CursorData { 
            level_index,
            array_iter: src.iter.clone(),
            cursors_iter: self.cursors.iter(),
        }
    }
}

pub struct CursorData<'cursor, 'item, I>
where
    I: Iterator<Item: Ref<Type: HibitTree>>
{
    level_index: usize,
    array_iter: I,
    cursors_iter: slice::Iter<'cursor, CursorsItem<'item, I>>,
}

/// Iterator for [MultiIntersection] [Cursor::Data].
/// 
/// Prefer using [fold]-based[^1] operations over [next]-ing.
///
/// [^1]: Such as [for_each], [sum], etc...
///
/// [fold]: Iterator::fold
/// [next]: Iterator::next 
/// [for_each]: Iterator::for_each
/// [sum]: Iterator::sum
impl<'cursor, 'item, I, T> Iterator for CursorData<'cursor, 'item, I>
where
    I: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item
{
    type Item = <IterItemCursor<'item, I> as HibitTreeCursorTypes<'cursor>>::Data;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Compiler optimizes away additional branching here.
        self.cursors_iter
            .next()
            .map(|array_cursor| unsafe { 
                let array = self.array_iter.next().unwrap_unchecked();
                array_cursor.data_unchecked(array, self.level_index)
            })
    }

    #[inline]
    fn fold<B, F>(mut self, mut init: B, mut f: F) -> B
    where
        Self: Sized,
        F: FnMut(B, Self::Item) -> B,
    {
        let level_index = self.level_index;
        for array_cursor in self.cursors_iter {
            let data = unsafe{
                let array = self.array_iter.next().unwrap_unchecked();
                array_cursor.data_unchecked(array, level_index) 
            };
            init = f(init, data);
        }
        init
    }
    
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.cursors_iter.size_hint()
    }
}

impl<'cursor, 'item, I, T> ExactSizeIterator for CursorData<'cursor, 'item, I>
where
    I: Iterator<Item = &'item T> + Clone,
    T: HibitTree + 'item
{}

impl<'item, 'this, Iter, T> MultiHibitTreeTypes<'this> for MultiIntersection<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item
{ 
    type IterItem = HibitTreeData<'item, T>; 
}

impl<'item, Iter, T> MultiHibitTree for MultiIntersection<Iter>
where
    Iter: Iterator<Item = &'item T> + Clone,
    T: RegularHibitTree + 'item
{} 

impl<Iter> LazyHibitTree for MultiIntersection<Iter>
where
    MultiIntersection<Iter>: HibitTree
{}

impl<Iter> Borrowable for MultiIntersection<Iter>{ type Borrowed = Self; }

/// Intersection between multiple &[HibitTree]s.
/// 
/// `iter` will be cloned and iterated multiple times.
/// Pass something like [slice::Iter].
#[inline]
pub fn multi_intersection<Iter>(iter: Iter) 
    -> MultiIntersection<Iter>
where
    Iter: Iterator<Item: Ref<Type: HibitTree>> + Clone,
{
    MultiIntersection{ iter }
}

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use crate::dense_tree::DenseTree;
    use crate::hibit_tree::HibitTree;
    use crate::utils::LendingIterator;
    use super::multi_intersection;

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
        
        let intersection = multi_intersection(arrays.iter());
        
        let mut iter = intersection.iter();
        while let Some((index, values)) = iter.next(){
            let values: Vec<_> = values.collect();
            println!("{:?}", values);
        }
        
        assert_equal( 
            intersection.get(15).unwrap(),
            vec![arrays[0].get(15).unwrap(), arrays[1].get(15).unwrap(), arrays[2].get(15).unwrap()]
        );
        assert!( intersection.get(200).is_none() );
        assert_equal(unsafe{ intersection.get_unchecked(15) }, intersection.get(15).unwrap());
    }

}
