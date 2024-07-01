use std::marker::PhantomData;
use std::borrow::Borrow;
use std::ops::BitAnd;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::{SparseHierarchy, SparseHierarchyState};
use crate::utils::{Borrowable, FnRR, Take};

pub struct Intersection2<S0, S1, F>{
    s0: S0,
    s1: S1,
    f: F
}

impl<S0, S1, F> SparseHierarchy2 for Intersection2<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy2>,
    S1: Borrowable<Borrowed: SparseHierarchy2<
        LevelCount    = <S0::Borrowed as SparseHierarchy2>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType,
    >>,
    
    F: FnRR<
        <S0::Borrowed as SparseHierarchy2>::DataType, 
        <S1::Borrowed as SparseHierarchy2>::DataType
    >,
    
    // &Mask & &Mask
    for<'a> &'a <S0::Borrowed as SparseHierarchy2>::LevelMaskType: BitAnd<&'a <S0::Borrowed as SparseHierarchy2>::LevelMaskType, Output = <S0::Borrowed as SparseHierarchy2>::LevelMaskType>,
{
    type LevelCount = <S0::Borrowed as SparseHierarchy2>::LevelCount;
    
    type LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self:'a;
    
    type DataType = F::Out;
    type Data<'a> = F::Out where Self: 'a;

    unsafe fn data<I>(&self, level_indices: I) -> Option<Self::Data<'_>>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        todo!()
    }

    unsafe fn data_unchecked<I>(&self, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        todo!()
    }

    type State = State<S0, S1, F>;
}

pub struct State<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy2>,
    S1: Borrowable<Borrowed: SparseHierarchy2>,
{
    s0: <S0::Borrowed as SparseHierarchy2>::State, 
    s1: <S1::Borrowed as SparseHierarchy2>::State,
    
    phantom_data: PhantomData<(S0, S1, F)>
}

impl<S0, S1, F> SparseHierarchyState2 for State<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy2>,
    S1: Borrowable<Borrowed: SparseHierarchy2<
        LevelCount    = <S0::Borrowed as SparseHierarchy2>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType,
    >>,
    
    F: FnRR<
        <S0::Borrowed as SparseHierarchy2>::DataType, 
        <S1::Borrowed as SparseHierarchy2>::DataType,
    >,
    
    // Actually, we can just use Take here, since as for now, masks always SIMD values.
    // &Mask & &Mask
    for<'a> &'a <S0::Borrowed as SparseHierarchy2>::LevelMaskType: BitAnd<&'a <S0::Borrowed as SparseHierarchy2>::LevelMaskType, Output = <S0::Borrowed as SparseHierarchy2>::LevelMaskType>,
{
    type This = Intersection2<S0, S1, F>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        Self{
            s0: SparseHierarchyState2::new(this.s0.borrow()), 
            s1: SparseHierarchyState2::new(this.s1.borrow()),
            phantom_data: PhantomData
        }
    }

    unsafe fn select_level_node<'a, N: ConstInteger>(&mut self, this: &'a Self::This, level_n: N, level_index: usize) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        todo!()
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger> (
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        let mask1 = self.s0.select_level_node_unchecked(
            this.s0.borrow(), level_n, level_index
        );
        let mask2 = self.s1.select_level_node_unchecked(
            this.s1.borrow(), level_n, level_index
        );
        
        mask1.borrow() & mask2.borrow()
    }

    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) -> Option<<Self::This as SparseHierarchy2>::Data<'a>> {
        todo!()
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy2>::Data<'a> 
    {
        let d0 = self.s0.data_unchecked(
            this.s0.borrow(), level_index
        );
        let d1 = self.s1.data_unchecked(
            this.s1.borrow(), level_index
        );

        (this.f)(d0.borrow(), d1.borrow())
    }
}

impl<S0, S1, F> Borrowable for Intersection2<S0, S1, F>{ type Borrowed = Self; }

#[inline]
pub fn intersection2<S0, S1, F>(s0: S0, s1: S1, f: F) -> Intersection2<S0, S1, F>
where
    // bounds needed here for F's arguments auto-deduction
    S0: Borrowable<Borrowed: SparseHierarchy2>,
    S1: Borrowable<Borrowed: SparseHierarchy2<
        LevelCount    = <S0::Borrowed as SparseHierarchy2>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType,
    >>,
    
    F: FnRR<
        <S0::Borrowed as SparseHierarchy2>::DataType, 
        <S1::Borrowed as SparseHierarchy2>::DataType,
    >,
{
    Intersection2{ s0, s1, f }
} 

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array2::CompactSparseArray2;
    use crate::ops2::intersection2::intersection2;
    use crate::sparse_hierarchy2::SparseHierarchy2;

    #[test]
    fn smoke_test(){
        type Array = CompactSparseArray2<usize, 3>;
        let mut a1= Array::default();
        let mut a2= Array::default();
        
        *a1.get_or_insert(10) = 10;
        *a1.get_or_insert(15) = 15;
        *a1.get_or_insert(200) = 200;
        
        *a2.get_or_insert(100) = 100;
        *a2.get_or_insert(15)  = 15;
        *a2.get_or_insert(200) = 200;        
        
        let i = intersection2(&a1, &a2, |i0, i1| i0+i1);
        
        assert_equal(i.iter(), [(15,30), (200, 400)]);
    }
}