use std::marker::PhantomData;
use std::borrow::Borrow;
use std::ops::{BitAnd, BitOr};
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyCursor};
use crate::{BitBlock, LazySparseHierarchy, SparseHierarchyCursorTypes, SparseHierarchyTypes};
use crate::bit_queue::BitQueue;
use crate::utils::{Array, Borrowable, Take};

pub struct Union<S0, S1>{
    s0: S0,
    s1: S1,
}

impl<'this, S0, S1> SparseHierarchyTypes<'this> for Union<S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask,
    >>,
{
    type Data = (
        Option<<S0::Borrowed as SparseHierarchyTypes<'this>>::Data>,
        Option<<S1::Borrowed as SparseHierarchyTypes<'this>>::Data>
    );
    
    type DataUnchecked = Self::Data;
    
    type Cursor = Cursor<'this, S0, S1>;
}

impl<S0, S1> SparseHierarchy for Union<S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask,
    >>,
{
    /// true if S0 & S1 are EXACT_HIERARCHY.
    const EXACT_HIERARCHY: bool = <S0::Borrowed as SparseHierarchy>::EXACT_HIERARCHY 
                                & <S1::Borrowed as SparseHierarchy>::EXACT_HIERARCHY;
    
    type LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount;
    type LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as SparseHierarchyTypes<'_>>::Data> 
    {
        let d0 = self.s0.borrow().data(index, level_indices);
        let d1 = self.s1.borrow().data(index, level_indices);
        if d0.is_none() & d1.is_none(){
            None
        } else {
            Some((d0, d1))
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> <Self as SparseHierarchyTypes<'_>>::Data
    {
        self.data(index, level_indices).unwrap_unchecked()
    }
}

/// [S::Mask; S::DEPTH]
type Masks<S> = ConstArrayType<
    <<S as Borrowable>::Borrowed as SparseHierarchy>::LevelMask,
    <<S as Borrowable>::Borrowed as SparseHierarchy>::LevelCount,
>;

pub struct Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy>,
{
    s0: <S0::Borrowed as SparseHierarchyTypes<'src>>::Cursor, 
    s1: <S1::Borrowed as SparseHierarchyTypes<'src>>::Cursor,
    phantom: PhantomData<&'src Union<S0, S1>>
}

impl<'this, 'src, S0, S1> SparseHierarchyCursorTypes<'this> for Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy>,
{
    type Data = (
        Option<<<S0::Borrowed as SparseHierarchyTypes<'src>>::Cursor as SparseHierarchyCursorTypes<'this>>::Data>,
        Option<<<S1::Borrowed as SparseHierarchyTypes<'src>>::Cursor as SparseHierarchyCursorTypes<'this>>::Data>
    );
}

impl<'src, S0, S1> SparseHierarchyCursor<'src> for Cursor<'src, S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask,
    >>
{
    type Src = Union<S0, S1>;

    #[inline]
    fn new(src: &'src Self::Src) -> Self {
        Self{
            s0: SparseHierarchyCursor::new(src.s0.borrow()), 
            s1: SparseHierarchyCursor::new(src.s1.borrow()),
            phantom: PhantomData
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, this: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        // unchecked version already deal with non-existent elements
        self.select_level_node_unchecked(this, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger> (
        &mut self, this: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        let mask0 = self.s0.select_level_node(
            this.s0.borrow(), level_n, level_index,
        );

        let mask1 = self.s1.select_level_node(
            this.s1.borrow(), level_n, level_index,
        );

        // mask0.take_or_clone() |= mask1.borrow() 
        {
            let mut mask = mask0.take_or_clone();
            mask |= mask1.borrow();
            mask
        }
    }

    #[inline]
    unsafe fn data<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> Option<<Self as SparseHierarchyCursorTypes<'a>>::Data> 
    {
        let d0 = self.s0.data(this.s0.borrow(), level_index);
        let d1 = self.s1.data(this.s1.borrow(), level_index);
        if d0.is_none() & d1.is_none(){
            None
        } else {
            Some((d0, d1))
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> <Self as SparseHierarchyCursorTypes<'a>>::Data 
    {
        self.data(this, level_index).unwrap_unchecked()
    }
}

impl<S0, S1> LazySparseHierarchy for Union<S0, S1>
where
    Union<S0, S1>: SparseHierarchy
{}

impl<S0, S1> Borrowable for Union<S0, S1>{ type Borrowed = Self; }

#[inline]
pub fn union<S0, S1>(s0: S0, s1: S1) -> Union<S0, S1>
where
    // bounds needed here for F's arguments auto-deduction
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask,
    >>
{
    Union { s0, s1 }
}

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use crate::compact_sparse_array::CompactSparseArray;
    use crate::map;
    use crate::ops::union::union;
    use crate::sparse_hierarchy::SparseHierarchy;

    #[test]
    fn smoke_test(){
        type Array = CompactSparseArray<usize, 3>;
        let mut a1 = Array::default();
        let mut a2 = Array::default();
        
        *a1.get_or_insert(10) = 10;
        *a1.get_or_insert(15) = 15;
        *a1.get_or_insert(200) = 200;
        
        *a2.get_or_insert(100) = 100;
        *a2.get_or_insert(15)  = 15;
        *a2.get_or_insert(200) = 200;        

        // test with map
        let union = map(union(&a1, &a2), |(i0, i1): (Option<&usize>, Option<&usize>)| {
            i0.unwrap_or(&0) + i1.unwrap_or(&0)
        });
        
        assert_eq!(unsafe{ union.get_unchecked(200) }, 400);
        assert_eq!(union.get(15), Some(30));
        assert_eq!(union.get(10), Some(10));
        assert_eq!(union.get(20), None);
        
        assert_equal(union.iter(), [(10, 10), (15, 30), (100, 100), (200, 400)]);
    }
}