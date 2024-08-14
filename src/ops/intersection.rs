use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::BitAnd;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::{LazySparseHierarchy, MonoSparseHierarchy, SparseHierarchyStateTypes, SparseHierarchyTypes};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::utils::{Borrowable, Take};

pub struct Intersection<S0, S1>{
    s0: S0,
    s1: S1
}

impl<'this, S0, S1> SparseHierarchyTypes<'this> for Intersection<S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask,
    >>,
{
    type Data = (
        <S0::Borrowed as SparseHierarchyTypes<'this>>::Data,
        <S1::Borrowed as SparseHierarchyTypes<'this>>::Data
    );
    
    type DataUnchecked = (
        <S0::Borrowed as SparseHierarchyTypes<'this>>::DataUnchecked,
        <S1::Borrowed as SparseHierarchyTypes<'this>>::DataUnchecked
    );
    
    type State = State<'this, S0, S1>;
}

impl<S0, S1> SparseHierarchy for Intersection<S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask,
    >>,
{
    const EXACT_HIERARCHY: bool = false;
    
    type LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount;
    type LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as SparseHierarchyTypes<'_>>::Data> 
    {
        let d0 = self.s0.borrow().data(index, level_indices);
        let d1 = self.s1.borrow().data(index, level_indices);
        if d0.is_none() | d1.is_none(){
            return None;
        }
        Some((d0.unwrap_unchecked(), d1.unwrap_unchecked()))
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> <Self as SparseHierarchyTypes<'_>>::DataUnchecked
    {
        let d0 = self.s0.borrow().data_unchecked(index, level_indices);
        let d1 = self.s1.borrow().data_unchecked(index, level_indices);
        (d0, d1)
    }
}

pub struct State<'src, S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy>,
{
    s0: <S0::Borrowed as SparseHierarchyTypes<'src>>::State, 
    s1: <S1::Borrowed as SparseHierarchyTypes<'src>>::State,
    phantom: PhantomData<&'src Intersection<S0, S1>>
}

impl<'this, 'src, S0, S1> SparseHierarchyStateTypes<'this> for State<'src, S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy>,
{
    type Data = (
        <<S0::Borrowed as SparseHierarchyTypes<'src>>::State as SparseHierarchyStateTypes<'this>>::Data,
        <<S1::Borrowed as SparseHierarchyTypes<'src>>::State as SparseHierarchyStateTypes<'this>>::Data
    );
}

impl<'src, S0, S1> SparseHierarchyState<'src> for State<'src, S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask,
    >>,
{
    type Src = Intersection<S0, S1>;

    #[inline]
    fn new(this: &'src Self::Src) -> Self {
        Self{
            s0: SparseHierarchyState::new(this.s0.borrow()), 
            s1: SparseHierarchyState::new(this.s1.borrow()),
            phantom: PhantomData
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, this: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        // Putting "if" here is not justified for general case. 
        
        let mask0 = self.s0.select_level_node(
            this.s0.borrow(), level_n, level_index
        );
        let mask1 = self.s1.select_level_node(
            this.s1.borrow(), level_n, level_index
        );
       
        // mask0.take_or_clone() &= mask1.borrow()
        {
            let mut mask = mask0.take_or_clone();
            mask &= mask1.borrow();
            mask
        }
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger> (
        &mut self, this: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        let mask0 = self.s0.select_level_node_unchecked(
            this.s0.borrow(), level_n, level_index
        );
        let mask1 = self.s1.select_level_node_unchecked(
            this.s1.borrow(), level_n, level_index
        );
        
        // mask0.take_or_clone() &= mask1.borrow()
        {
            let mut mask = mask0.take_or_clone();
            mask &= mask1.borrow();
            mask
        }
    }

    #[inline]
    unsafe fn data<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> Option<<Self as SparseHierarchyStateTypes<'a>>::Data> 
    {
        let d0 = self.s0.data(this.s0.borrow(), level_index);
        let d1 = self.s1.data(this.s1.borrow(), level_index);
        // TODO: Probably there is a case, where we can prove that 
        //       d0_exists == d1_exists, and we can check only one of them
        //       for existence.
        if d0.is_none() | d1.is_none(){
            return None;
        }
        Some(( d0.unwrap_unchecked(), d1.unwrap_unchecked() ))
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> <Self as SparseHierarchyStateTypes<'a>>::Data 
    {
        let d0 = self.s0.data_unchecked(this.s0.borrow(), level_index);
        let d1 = self.s1.data_unchecked(this.s1.borrow(), level_index);
        (d0, d1)
    }
}

impl<S0, S1> LazySparseHierarchy for Intersection<S0, S1>
where
    Intersection<S0, S1>: MonoSparseHierarchy
{}

impl<S0, S1> Borrowable for Intersection<S0, S1>{ type Borrowed = Self; }

#[inline]
pub fn intersection<S0, S1>(s0: S0, s1: S1) -> Intersection<S0, S1>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMask  = <S0::Borrowed as SparseHierarchy>::LevelMask,
    >>,
{
    Intersection { s0, s1 }
} 

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use crate::compact_sparse_array::CompactSparseArray;
    use crate::ops::intersection::intersection;
    use crate::sparse_hierarchy::SparseHierarchy;

    #[test]
    fn smoke_test(){
        type Array = CompactSparseArray<usize, 3>;
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

        for (key, value) in intersect.iter(){
            println!("{:?}", value);
        }
        
        // TODO: Move to integral tests. Requires map() now.
/*        let i = intersection(&a1, &a2, |i0, i1| i0+i1);
        
        assert_eq!(unsafe{ i.get_unchecked(200) }, 400);
        assert_eq!(i.get(15), Some(30));
        assert_eq!(i.get(10), None);
        assert_eq!(i.get(20), None);
        
        assert_equal(i.iter(), [(15,30), (200, 400)]);*/
    }
}