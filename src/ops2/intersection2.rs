use std::marker::PhantomData;
use std::borrow::Borrow;
use std::hint::unreachable_unchecked;
use std::ops::BitAnd;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::utils::{Borrowable, FnRR, Take};

pub struct Intersection<S0, S1, F>{
    s0: S0,
    s1: S1,
    f: F
}

impl<S0, S1, F> SparseHierarchy2 for Intersection<S0, S1, F>
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
{
    const EXACT_HIERARCHY: bool = false;
    
    type LevelCount = <S0::Borrowed as SparseHierarchy2>::LevelCount;
    
    type LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self:'a;
    
    type DataType = F::Out;
    type Data<'a> = F::Out where Self: 'a;

    unsafe fn data<I>(&self, index: usize, level_indices: I) -> Option<Self::Data<'_>>
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
{
    type This = Intersection<S0, S1, F>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        Self{
            s0: SparseHierarchyState2::new(this.s0.borrow()), 
            s1: SparseHierarchyState2::new(this.s1.borrow()),
            phantom_data: PhantomData
        }
    }

    #[inline]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        // Putting if here is not justified for general case. 
        
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
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger> (
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
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
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> Option<<Self::This as SparseHierarchy2>::Data<'a>> 
    {
        let d0 = self.s0.data(this.s0.borrow(), level_index);
        if d0.is_none(){
            return None;
        }
        let d1 = self.s1.data(this.s1.borrow(), level_index);
        
        let o0 = if let Some(d) = &d0 {
            d.borrow()
        } else { unreachable_unchecked() };
        
        let o1 = if let Some(d) = &d1 {
            d.borrow()
        } else { unreachable_unchecked() };
        
        return Some((this.f)(o0, o1));
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

impl<S0, S1, F> Borrowable for Intersection<S0, S1, F>{ type Borrowed = Self; }

#[inline]
pub fn intersection<S0, S1, F>(s0: S0, s1: S1, f: F) -> Intersection<S0, S1, F>
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
    Intersection { s0, s1, f }
} 

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array3::CompactSparseArray;
    use crate::ops2::intersection2::intersection;
    use crate::sparse_hierarchy2::SparseHierarchy2;

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
        
        let i = intersection(&a1, &a2, |i0, i1| i0+i1);
        
        assert_equal(i.iter(), [(15,30), (200, 400)]);
    }
}