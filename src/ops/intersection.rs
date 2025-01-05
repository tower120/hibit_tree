use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::BitAnd;
use crate::const_utils::{ConstAnd, ConstBool, ConstInteger};
use crate::{LazyHibitTree, RegularHibitTree, HibitTreeCursorTypes, HibitTreeTypes, HierarchyIndex};
use crate::hibit_tree::{HibitTree, HibitTreeCursor};
use crate::utils::{Borrowable};

pub struct Intersection<S0, S1>{
    s0: S0,
    s1: S1,
}

impl<'this, S0, S1> HibitTreeTypes<'this> for Intersection<S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
{
    type Data = (
        <S0::Borrowed as HibitTreeTypes<'this>>::Data,
        <S1::Borrowed as HibitTreeTypes<'this>>::Data
    );
    
    type DataUnchecked = (
        <S0::Borrowed as HibitTreeTypes<'this>>::DataUnchecked,
        <S1::Borrowed as HibitTreeTypes<'this>>::DataUnchecked
    );
    
    type DataOrDefault = (
        <S0::Borrowed as HibitTreeTypes<'this>>::DataOrDefault,
        <S1::Borrowed as HibitTreeTypes<'this>>::DataOrDefault
    );
    
    type Cursor = Cursor<'this, S0, S1>;
}

impl<S0, S1> HibitTree for Intersection<S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
{
    const EXACT_HIERARCHY: bool = false;
    
    /// true if S0 & S1 are having [DefaultData].
    /// 
    /// [DefaultData]: HibitTree::DefaultData
    type DefaultData = ConstAnd<
        <S0::Borrowed as HibitTree>::DefaultData,
        <S1::Borrowed as HibitTree>::DefaultData
    >;

    type LevelCount = <S0::Borrowed as HibitTree>::LevelCount;
    type LevelMask  = <S0::Borrowed as HibitTree>::LevelMask;

    #[inline]
    fn data(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
    {
        let d0 = self.s0.borrow().data(index);
        let d1 = self.s1.borrow().data(index);
        if d0.is_none() | d1.is_none(){
            return None;
        }
        unsafe{
            Some((d0.unwrap_unchecked(), d1.unwrap_unchecked()))
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked
    {
        let d0 = self.s0.borrow().data_unchecked(index);
        let d1 = self.s1.borrow().data_unchecked(index);
        (d0, d1)
    }

    #[inline]
    unsafe fn data_or_default(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> <Self as HibitTreeTypes<'_>>::DataOrDefault
    {
        let d0 = self.s0.borrow().data_or_default(index);
        let d1 = self.s1.borrow().data_or_default(index);
        (d0, d1)
    }
}

pub struct Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree>,
{
    s0: <S0::Borrowed as HibitTreeTypes<'src>>::Cursor, 
    s1: <S1::Borrowed as HibitTreeTypes<'src>>::Cursor,
    phantom: PhantomData<&'src Intersection<S0, S1>>
}

impl<'this, 'src, S0, S1> HibitTreeCursorTypes<'this> for Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree>,
{
    type Data = (
        <<S0::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::Data,
        <<S1::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::Data
    );
    type DataUnchecked = (
        <<S0::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::DataUnchecked,
        <<S1::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::DataUnchecked
    );
    type DataOrDefault = (
        <<S0::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::DataOrDefault,
        <<S1::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::DataOrDefault
    );
}

impl<'src, S0, S1> HibitTreeCursor<'src> for Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
{
    type Tree = Intersection<S0, S1>;

    #[inline]
    fn new(this: &'src Self::Tree) -> Self {
        Self{
            s0: HibitTreeCursor::new(this.s0.borrow()), 
            s1: HibitTreeCursor::new(this.s1.borrow()),
            phantom: PhantomData
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, this: &'src Self::Tree, level_n: N, level_index: usize
    ) -> <Self::Tree as HibitTree>::LevelMask {
        let mask0 = self.s0.select_level_node(
            this.s0.borrow(), level_n, level_index
        );
        let mask1 = self.s1.select_level_node(
            this.s1.borrow(), level_n, level_index
        );
       
        // mask0.take_or_clone() &= mask1.borrow()
        {
            let mut mask = mask0;
            mask &= mask1.borrow();
            mask
        }
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger> (
        &mut self, this: &'src Self::Tree, level_n: N, level_index: usize
    ) -> <Self::Tree as HibitTree>::LevelMask {
        let mask0 = self.s0.select_level_node_unchecked(
            this.s0.borrow(), level_n, level_index
        );
        let mask1 = self.s1.select_level_node_unchecked(
            this.s1.borrow(), level_n, level_index
        );
        
        // mask0.take_or_clone() &= mask1.borrow()
        {
            let mut mask = mask0;
            mask &= mask1.borrow();
            mask
        }
    }

    #[inline]
    unsafe fn data<'a>(&'a self, this: &'src Self::Tree, level_index: usize) 
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        let d0 = self.s0.data(this.s0.borrow(), level_index);
        let d1 = self.s1.data(this.s1.borrow(), level_index);
        if d0.is_none() | d1.is_none(){
            return None;
        }
        Some(( d0.unwrap_unchecked(), d1.unwrap_unchecked() ))
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, this: &'src Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataUnchecked
    {
        let d0 = self.s0.data_unchecked(this.s0.borrow(), level_index);
        let d1 = self.s1.data_unchecked(this.s1.borrow(), level_index);
        (d0, d1)
    }
    
    #[inline]
    unsafe fn data_or_default<'a>(&'a self, this: &'src Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataOrDefault
    {
        let d0 = self.s0.data_or_default(this.s0.borrow(), level_index);
        let d1 = self.s1.data_or_default(this.s1.borrow(), level_index);
        (d0, d1)
    }
}

impl<S0, S1> LazyHibitTree for Intersection<S0, S1>
where
    Intersection<S0, S1>: RegularHibitTree
{}

impl<S0, S1> Borrowable for Intersection<S0, S1>{ type Borrowed = Self; }

#[inline]
pub fn intersection<S0, S1>(s0: S0, s1: S1) -> Intersection<S0, S1>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
{
    Intersection { s0, s1 }
}

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use crate::dense_tree::DenseTree;
    use crate::ops::intersection::intersection;
    use crate::hibit_tree::HibitTree;
    use crate::map;

    #[test]
    fn smoke_test(){
        type Array = DenseTree<usize, 3>;
        let mut a1= Array::default();
        let mut a2= Array::default();
        
        *a1.get_or_insert(10) = 10;
        *a1.get_or_insert(15) = 15;
        *a1.get_or_insert(200) = 200;
        
        *a2.get_or_insert(100) = 100;
        *a2.get_or_insert(15)  = 15;
        *a2.get_or_insert(200) = 200;
        
        let intersect = intersection(&a1, &a2);

        assert_equal(intersect.iter(), [
            (15, (a1.get(15).unwrap(), a2.get(15).unwrap() ) ),
            (200, (a1.get(200).unwrap(), a2.get(200).unwrap() ) ) 
        ]);
        
        let i = map(intersection(&a1, &a2), |(i0, i1): (&usize, &usize)| i0+i1);
        assert_eq!(unsafe{ i.get_unchecked(200) }, 400);
        assert_eq!(i.get(15), Some(30));
        assert_eq!(i.get(10), None);
        assert_eq!(i.get(20), None);
        assert_equal(i.iter(), [(15,30), (200, 400)]);
    }
}