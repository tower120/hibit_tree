//! Deprecated - use map instead
//! Experimental.

use std::marker::PhantomData;
use crate::{HibitTree, HibitTreeCursor, HibitTreeCursorTypes, HibitTreeData, HibitTreeTypes, HierarchyIndex, MultiHibitTree, MultiHibitTreeIterItem, MultiHibitTreeTypes};
use crate::const_utils::{ConstBool, ConstFalse, ConstInteger, ConstTrue, IsConstTrue};
use crate::utils::Borrowable;

pub trait MultiMapFn<IterItem> {
    type Output;
    fn exec<Iter: Iterator<Item = IterItem>>(&self, iter: Iter) -> Self::Output;
}

pub struct MultiMap<T, F, D = ConstFalse>{
    tree: T,
    f: F,
    phantom: PhantomData<D>,
}

impl<'this, T, F, D> HibitTreeTypes<'this> for MultiMap<T, F, D>
where
    T: MultiHibitTree,
    F: for<'a> MultiMapFn<MultiHibitTreeIterItem<'a, T>>,
    D: ConstBool
{
    type Data = <F as MultiMapFn<MultiHibitTreeIterItem<'this, T>>>::Output;
    type DataUnchecked = Self::Data;
    type DataOrDefault = Self::Data;
    type Cursor = MultiMapCursor<'this, T, F, D>;
}

impl<T, F, D> HibitTree for MultiMap<T, F, D>
where
    T: MultiHibitTree,
    F: for<'a> MultiMapFn<MultiHibitTreeIterItem<'a, T>>,
    D: ConstBool
{
    const EXACT_HIERARCHY: bool = T::EXACT_HIERARCHY;
    type DefaultData = D;
    
    type LevelCount = T::LevelCount;
    type LevelMask  = T::LevelMask;

    #[inline]
    fn data(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>) 
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
    {
        self.tree.data(index).map(|iter| self.f.exec(iter))
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>) 
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked 
    {
        let iter = self.tree.data_unchecked(index);
        self.f.exec(iter)
    }
    
    #[inline]
    unsafe fn data_or_default(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>) 
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked 
    {
        let iter = self.tree.data_or_default(index);
        self.f.exec(iter)
    }
}

pub struct MultiMapCursor<'tree, T, F, D>
where
    T: MultiHibitTree,
{
    cursor: <T as HibitTreeTypes<'tree>>::Cursor,
    phantom: PhantomData<&'tree MultiMap<T, F, D>>,
}

impl<'this, 'tree, T, F, D> HibitTreeCursorTypes<'this> for MultiMapCursor<'tree, T, F, D>
where
    T: MultiHibitTree,
    F: for<'a> MultiMapFn<MultiHibitTreeIterItem<'a, T>>,
    D: ConstBool
{
    type Data = HibitTreeData<'tree, MultiMap<T, F, D>>;
    type DataUnchecked = Self::Data;
    type DataOrDefault = Self::Data;
}

impl<'tree, T, F, D> HibitTreeCursor<'tree> for MultiMapCursor<'tree, T, F, D>
where
    T: MultiHibitTree,
    F: for<'a> MultiMapFn<MultiHibitTreeIterItem<'a, T>>,
    D: ConstBool
{
    type Tree = MultiMap<T, F, D>;

    #[inline]
    fn new(tree: &'tree Self::Tree) -> Self {
        Self{
            cursor : HibitTreeCursor::new(&tree.tree),
            phantom: Default::default(),
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(&mut self, tree: &'tree Self::Tree, level_n: N, level_index: usize) 
        -> <Self::Tree as HibitTree>::LevelMask 
    {
        self.cursor.select_level_node(&tree.tree, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(&mut self, tree: &'tree Self::Tree, level_n: N, level_index: usize) 
        -> <Self::Tree as HibitTree>::LevelMask 
    {
        self.cursor.select_level_node_unchecked(&tree.tree, level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        self.cursor.data(&tree.tree, level_index).map(|iter| tree.f.exec(iter))
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataUnchecked 
    {
        let iter = self.cursor.data_unchecked(&tree.tree, level_index);
        tree.f.exec(iter)
    }
    
    #[inline]
    unsafe fn data_or_default<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataOrDefault 
    {
        let iter = self.cursor.data_or_default(&tree.tree, level_index);
        tree.f.exec(iter)
    }    
}

impl<T, F, D> Borrowable for MultiMap<T, F, D> {
    type Borrowed = Self;
}

pub fn multi_map<T, F>(tree: T, f: F) -> MultiMap<T, F>
where
    T: MultiHibitTree,
    F: for<'a> MultiMapFn<
        <T as MultiHibitTreeTypes<'a>>::IterItem
    >,
{
    MultiMap{
        tree,
        f,
        phantom: Default::default(),
    }
}

pub fn multi_map_w_default<T, F>(tree: T, f: F) -> MultiMap<T, F, ConstTrue>
where
    T: MultiHibitTree<DefaultData: IsConstTrue>,
    F: for<'a> MultiMapFn<
        <T as MultiHibitTreeTypes<'a>>::IterItem
    >,
{
    MultiMap{
        tree,
        f,
        phantom: Default::default(),
    }
}

/*macro_rules! multi_map {
    ($tree:tt, |$arg:ident : $iter_item:ty| -> $output:ty { $body:expr } ) => {
        struct F;
        impl MultiMapFn<$iter_item> for F{
            type Output = $output;

            fn exec<Iter: Iterator<Item=$iter_item>>(&self, $arg: Iter) -> Self::Output {
                $body
            }
        }        
        
        multi_map($tree, F)
    }
}*/

macro_rules! multi_map_fn {
    ([$($generics:tt)*] |$arg:ident: impl Iterator<Item=$iter_item:ty>| where [$($bounds:tt)*] -> $output:ty $body:block ) => {
        {
            struct F;
            impl<$($generics)*> MultiMapFn<$iter_item> for F
            where
                $($bounds)*
            {
                type Output = $output;
    
                #[inline]
                fn exec<Iter: Iterator<Item=$iter_item>>(&self, mut $arg: Iter) -> Self::Output {
                    $body
                }
            }
            F
        }
    };
    
    ([$($generics:tt)*] |$arg:ident: impl Iterator<Item=$iter_item:ty>| -> $output:ty $body:block ) => {
        multi_map_fn!(
            [$($generics)*] |$arg: impl Iterator<Item=$iter_item>| where [] -> $output { $body } 
        )
    };
    
    (|$arg:ident: impl Iterator<Item=&$iter_item:ty>| -> &$output:ty { $body:expr } ) => {
        multi_map_fn!(
            ['a] |$arg: impl Iterator<Item=&'a $iter_item>| where [] -> &'a $output { $body } 
        )
    };
    
    (|$arg:ident: impl Iterator<Item=&$iter_item:ty>| -> $output:ty { $body:expr } ) => {
        multi_map_fn!(
            ['a] |$arg: impl Iterator<Item=&'a $iter_item>| where [] -> $output { $body } 
        )
    };        

    (|$arg:ident: impl Iterator<Item=$iter_item:ty>| -> $output:ty { $body:expr } ) => {
        multi_map_fn!(
            [] |$arg: impl Iterator<Item=$iter_item>| where [] -> $output { $body } 
        )
    };
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::{multi_intersection, ReqDefault};
    use crate::hibit_tree::RegularHibitTree;
    use crate::ops::iterate_with_default::iterate_with_default;
    use crate::tree2::Config64bit;
    use crate::utils::Primitive;
    use super::*;
    
    #[test]
    fn multi_map_fn_macro_test(){
        let is = [0,1,2,3];
        let f = multi_map_fn!(['a, T: Clone] |xs: impl Iterator<Item=&'a T>| -> T {
            xs.next().unwrap().clone()
        });
        let out = f.exec(is.iter());
        
        let f = multi_map_fn!(|xs: impl Iterator<Item=&usize>| -> &usize {
            xs.next().unwrap()
        });
        let out = f.exec(is.iter());
        
        let f = multi_map_fn!(|xs: impl Iterator<Item=&usize>| -> usize {
            *xs.next().unwrap()
        });
        let out = f.exec(is.iter());
        
        let f = multi_map_fn!(|xs: impl Iterator<Item=usize>| -> usize {
            xs.next().unwrap()
        });
        let out = f.exec(is.iter().copied());
    }
    
    #[test]
    fn smoke_test() {
        type Tree<T> = crate::tree2::Tree<T, Config64bit<3>, ReqDefault>;
        type Array = Tree<usize>; 
        let mut a1: Array = Default::default();
        let mut a2: Array = Default::default();
        let mut a3: Array = Default::default();
        
        a1.insert(10, 10);
        a1.insert(15, 15);
        a1.insert(30, 30);
        
        a2.insert(15, 15);
        a2.insert(20, 20);
        
        a3.insert(15, 15);
        a3.insert(30, 30);
        
        let arrays = vec![a1, a2, a3];
        let intersect = multi_map_w_default(
            multi_intersection(arrays.iter()), 
            //multi_map_fn!(|xs: impl Iterator<Item=&usize>| -> usize { xs.sum() })
            multi_map_fn!(['a, T: Primitive] |xs: impl Iterator<Item=&'a T>| -> T {
                let mut sum = 0;
                for i in xs{
                    sum += i.as_usize();
                }
                Primitive::from_usize(sum)
            })
        );
        assert_eq!(intersect.get(10), None);
        assert_eq!(intersect.get(15), Some(45));
        assert_eq!(intersect.get(30), None);
        assert_eq!(unsafe{ intersect.get_unchecked(15) }, 45);
        //assert_eq!(intersect.get_or_default(4), 0);
        //assert_equal(intersect.iter(), [(15, 45)]);


        {
            let intersect = iterate_with_default(intersect);
            let mut iter = intersect.iter();
            for i in iter{
                println!("{:?}", i);
            }
            //iter.next();
        }
        /*assert_equal(
            iterate_with_default(intersect).iter(), 
            [
                (15, 45),
            ]
        );*/
    }    
}
