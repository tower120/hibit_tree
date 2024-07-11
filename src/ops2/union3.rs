use std::marker::PhantomData;
use std::borrow::Borrow;
use std::hint::unreachable_unchecked;
use std::mem::MaybeUninit;
use std::ops::{BitAnd, BitOr};
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::BitBlock;
use crate::bit_queue::BitQueue;
use crate::utils::{Array, Borrowable, FnRR, Take};


/*// Not used now
trait OptionBorrow<T>{
    fn option_borrow(&self) -> Option<&T>; 
}
impl<T> OptionBorrow<T> for Option<&T>{
    fn option_borrow(&self) -> Option<&T> {
        *self
    }
}
impl<T> OptionBorrow<T> for Option<T>{
    fn option_borrow(&self) -> Option<&T> {
        self.as_ref()
    }
}*/

pub trait UnionResolve<T0, T1>
    : Fn(Option<&T0>, Option<&T1>) -> Self::Out
{
    type Out;
}

impl<F, T0, T1, Out> UnionResolve<T0, T1> for F 
where
    F: Fn(Option<&T0>, Option<&T1>) -> Out,
{
    type Out = Out; 
}


pub struct Union<S0, S1, F>{
    s0: S0,
    s1: S1,
    f: F
}

impl<S0, S1, F> SparseHierarchy2 for Union<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy2<DataType: Clone>>,
    S1: Borrowable<Borrowed: SparseHierarchy2<
        LevelCount    = <S0::Borrowed as SparseHierarchy2>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType,
    >>,
    
    F: UnionResolve<
        // v1
        <S0::Borrowed as SparseHierarchy2>::DataType, 
        <S1::Borrowed as SparseHierarchy2>::DataType,
    >,
{
    /// true if S0 & S1 are EXACT_HIERARCHY.
    const EXACT_HIERARCHY: bool = <S0::Borrowed as SparseHierarchy2>::EXACT_HIERARCHY 
                                & <S1::Borrowed as SparseHierarchy2>::EXACT_HIERARCHY;
    
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

/// [S::Mask; S::DEPTH]
type Masks<S> = ConstArrayType<
    <<S as Borrowable>::Borrowed as SparseHierarchy2>::LevelMaskType,
    <<S as Borrowable>::Borrowed as SparseHierarchy2>::LevelCount,
>;

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
    S0: Borrowable<Borrowed: SparseHierarchy2<DataType: Clone>>,
    S1: Borrowable<Borrowed: SparseHierarchy2<
        LevelCount    = <S0::Borrowed as SparseHierarchy2>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType,
    >>,
    
    F: UnionResolve<
        // v1
        <S0::Borrowed as SparseHierarchy2>::DataType, 
        <S1::Borrowed as SparseHierarchy2>::DataType,
    >,
{
    type This = Union<S0, S1, F>;

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
        // unchecked version already deal with non-existent elements
        self.select_level_node_unchecked(this, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger> (
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
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
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> Option<<Self::This as SparseHierarchy2>::Data<'a>> 
    {
        let d0 = self.s0.data(this.s0.borrow(), level_index);
        let d1 = self.s1.data(this.s1.borrow(), level_index);
        
        let d0_is_none = d0.is_none(); 
        let d1_is_none = d1.is_none();
        if d0_is_none & d1_is_none{
            return None;
        }
        
        // Looks like compiler optimize away these re-borrow transformations.
        let o0;
        let o1;
        if d0_is_none {
            o0 = None;
            
            // we know that d1 exists.
            o1 = if let Some(d) = &d1 {
                Some(d.borrow())
            } else { unreachable_unchecked() };
        } else if d1_is_none {
            // we know that d0 exists.
            o0 = if let Some(d) = &d0 {
                Some(d.borrow())
            } else { unreachable_unchecked() };
            
            o1 = None;
        } else {
            // both exists
            o0 = if let Some(d) = &d0 {
                Some(d.borrow())
            } else { unreachable_unchecked() };
            
            o1 = if let Some(d) = &d1 {
                Some(d.borrow())
            } else { unreachable_unchecked() };
        }
        
        return Some((this.f)(o0, o1));
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy2>::Data<'a> 
    {
        self.data(this, level_index).unwrap_unchecked()
    }
}

impl<S0, S1, F> Borrowable for Union<S0, S1, F>{ type Borrowed = Self; }

#[inline]
pub fn union<S0, S1, F>(s0: S0, s1: S1, f: F) -> Union<S0, S1, F>
where
    // bounds needed here for F's arguments auto-deduction
    S0: Borrowable<Borrowed: SparseHierarchy2>,
    S1: Borrowable<Borrowed: SparseHierarchy2<
        LevelCount    = <S0::Borrowed as SparseHierarchy2>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType,
    >>,
    
    F: UnionResolve<
        <S0::Borrowed as SparseHierarchy2>::DataType, 
        <S1::Borrowed as SparseHierarchy2>::DataType,
    >,
{
    Union { s0, s1, f }
} 

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array3::CompactSparseArray;
    use crate::ops2::union3::union;
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
        
        let union = union(&a1, &a2, |i0, i1| {
            i0.unwrap_or(&0) + i1.unwrap_or(&0)
        });
        
        assert_equal(union.iter(), [(10, 10), (15, 30), (100, 100), (200, 400)]);
    }
}