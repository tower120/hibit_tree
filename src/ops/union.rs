use std::marker::PhantomData;
use std::borrow::Borrow;
use std::ops::{BitAnd, BitOr};
use crate::const_utils::{ConstAnd, ConstBool, ConstFalse, ConstInteger, ConstTrue, IsConstTrue};
use crate::hibit_tree::{HibitTree, HibitTreeCursor};
use crate::{BitBlock, LazyHibitTree, HibitTreeCursorTypes, HibitTreeTypes, HierarchyIndex};
use crate::bit_queue::BitQueue;
use crate::utils::{Array, Borrowable};

pub struct Union<S0, S1, D=ConstFalse>{
    s0: S0,
    s1: S1,
    phantom: PhantomData<D>
}

impl<'this, S0, S1, D> HibitTreeTypes<'this> for Union<S0, S1, D>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
    D: ConstBool
{
    type Data = (
        Option<<S0::Borrowed as HibitTreeTypes<'this>>::Data>,
        Option<<S1::Borrowed as HibitTreeTypes<'this>>::Data>
    );
    
    type DataUnchecked = D::Conditional<
        Self::DataOrDefault,
        Self::Data
    >;
    
    type DataOrDefault = (
        Option<<S0::Borrowed as HibitTreeTypes<'this>>::DataOrDefault>,
        Option<<S1::Borrowed as HibitTreeTypes<'this>>::DataOrDefault>
    );

    type Cursor = Cursor<'this, S0, S1, D>;
}

impl<S0, S1, D> HibitTree for Union<S0, S1, D>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
    D: ConstBool
{
    /// true if S0 & S1 are EXACT_HIERARCHY.
    const EXACT_HIERARCHY: bool = <S0::Borrowed as HibitTree>::EXACT_HIERARCHY 
                                & <S1::Borrowed as HibitTree>::EXACT_HIERARCHY;

    /// true if S0 & S1 are [DefaultData]-friendly.
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
        if d0.is_none() & d1.is_none(){
            None
        } else {
            Some((d0, d1))
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked
    {
        D::conditional_exec(
            || self.data_or_default(index),
            || self.data(index).unwrap_unchecked(),
        )
    }
    
    #[inline]
    unsafe fn data_or_default(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>) 
        -> <Self as HibitTreeTypes<'_>>::DataOrDefault
    {
        let d0 = self.s0.borrow().data_or_default(index);
        let d1 = self.s1.borrow().data_or_default(index);
        (Some(d0), Some(d1))
    }
}

pub struct Cursor<'src, S0, S1, D>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree>,
{
    s0: <S0::Borrowed as HibitTreeTypes<'src>>::Cursor, 
    s1: <S1::Borrowed as HibitTreeTypes<'src>>::Cursor,
    phantom: PhantomData<&'src Union<S0, S1, D>>
}

impl<'this, 'src, S0, S1, D> HibitTreeCursorTypes<'this> for Cursor<'src, S0, S1, D>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree>,
    D: ConstBool
{
    type Data = (
        Option<<<S0::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::Data>,
        Option<<<S1::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::Data>
    );
    type DataUnchecked = D::Conditional<
        Self::DataOrDefault,
        Self::Data
    >;
    type DataOrDefault = (
        Option<<<S0::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::DataOrDefault>,
        Option<<<S1::Borrowed as HibitTreeTypes<'src>>::Cursor as HibitTreeCursorTypes<'this>>::DataOrDefault>
    );
}

impl<'tree, S0, S1, D> HibitTreeCursor<'tree> for Cursor<'tree, S0, S1, D>
where
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>,
    D: ConstBool
{
    type Tree = Union<S0, S1, D>;

    #[inline]
    fn new(src: &'tree Self::Tree) -> Self {
        Self{
            s0: HibitTreeCursor::new(src.s0.borrow()), 
            s1: HibitTreeCursor::new(src.s1.borrow()),
            phantom: PhantomData
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, this: &'tree Self::Tree, level_n: N, level_index: usize
    ) -> <Self::Tree as HibitTree>::LevelMask {
        let mask0 = self.s0.select_level_node(
            this.s0.borrow(), level_n, level_index,
        );

        let mask1 = self.s1.select_level_node(
            this.s1.borrow(), level_n, level_index,
        );

        // mask0.take_or_clone() |= mask1.borrow() 
        {
            let mut mask = mask0;
            mask |= mask1.borrow();
            mask
        }
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger> (
        &mut self, this: &'tree Self::Tree, level_n: N, level_index: usize
    ) -> <Self::Tree as HibitTree>::LevelMask {
        // There is actually no unchecked version for union.
        self.select_level_node(this, level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        let d0 = self.s0.data(tree.s0.borrow(), level_index);
        let d1 = self.s1.data(tree.s1.borrow(), level_index);
        if d0.is_none() & d1.is_none(){
            None
        } else {
            Some((d0, d1))
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataUnchecked 
    {
        D::conditional_exec(
            || self.data_or_default(tree, level_index), 
            || self.data(tree, level_index).unwrap_unchecked(),
        )
    }
    
    #[inline]
    unsafe fn data_or_default<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataOrDefault
    {
        let d0 = self.s0.data_or_default(tree.s0.borrow(), level_index);
        let d1 = self.s1.data_or_default(tree.s1.borrow(), level_index);
        (Some(d0), Some(d1))
    }
}

impl<S0, S1, D> LazyHibitTree for Union<S0, S1, D>
where
    Union<S0, S1, D>: HibitTree
{}

impl<S0, S1, D> Borrowable for Union<S0, S1, D>{ type Borrowed = Self; }

#[inline]
pub fn union<S0, S1>(s0: S0, s1: S1) -> Union<S0, S1>
where
    // bounds needed here for F's arguments auto-deduction
    S0: Borrowable<Borrowed: HibitTree>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
    >>
{
    Union { s0, s1, phantom: Default::default() }
}

/// Same as [union] but iterator will use [data_or_default].
/// 
/// This can lead to a faster code, since you don't need to unwrap values.
#[inline]
pub fn union_w_default<S0, S1>(s0: S0, s1: S1) -> Union<S0, S1, ConstTrue>
where
    // bounds needed here for F's arguments auto-deduction
    S0: Borrowable<Borrowed: HibitTree<DefaultData: IsConstTrue>>,
    S1: Borrowable<Borrowed: HibitTree<
        LevelCount = <S0::Borrowed as HibitTree>::LevelCount,
        LevelMask  = <S0::Borrowed as HibitTree>::LevelMask,
        DefaultData: IsConstTrue
    >>
{
    // TODO: map to unwrap Options as well 
    Union { s0, s1, phantom: Default::default() }
}

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use crate::{map, map_w_default, ReqDefault};
    use crate::ops::union::{union, union_w_default};
    use crate::hibit_tree::HibitTree;
    use crate::tree::Config64bit;

    #[test]
    fn smoke_test(){
        type Array = crate::tree::Tree<usize, Config64bit<3>, ReqDefault>;
        let mut a1 = Array::default();
        let mut a2 = Array::default();
        
        a1.insert(10, 10);
        a1.insert(15, 15);
        a1.insert(200, 200);
        
        a2.insert(100, 100);
        a2.insert(15, 15);
        a2.insert(200, 200);
        
        let union = union(&a1, &a2);
        
        assert_eq!(unsafe{ union.get_unchecked(200) }, (Some(&200), Some(&200)));
        assert_eq!(union.get_or_default(200), (Some(&200), Some(&200)));
        assert_eq!(union.get_or_default(201), (Some(&0), Some(&0)));
        
        // test with map
        let union = map_w_default(union, |(i0, i1): (Option<&usize>, Option<&usize>)| {
            i0.unwrap_or(&0) + i1.unwrap_or(&0)
        });
        
        assert_eq!(unsafe{ union.get_unchecked(200) }, 400);
        assert_eq!(union.get_or_default(200), 400);
        assert_eq!(union.get(15), Some(30));
        assert_eq!(union.get(10), Some(10));
        assert_eq!(union.get(20), None);
        
        assert_equal(union.iter(), [(10, 10), (15, 30), (100, 100), (200, 400)]);
    }
}