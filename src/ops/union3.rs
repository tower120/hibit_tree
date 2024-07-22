use std::marker::PhantomData;
use std::borrow::Borrow;
use std::hint::unreachable_unchecked;
use std::mem::MaybeUninit;
use std::ops::{BitAnd, BitOr};
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
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

// TODO: consider removing
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

#[inline]
fn get_data<T0, T1, F, R>(d0: Option<impl Borrow<T0>>, d1: Option<impl Borrow<T1>>, f: &F) 
    -> Option<R>
where
    F: Fn(Option<&T0>, Option<&T1>) -> R,
{
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
        } else { unsafe {unreachable_unchecked()} };
    } else if d1_is_none {
        // we know that d0 exists.
        o0 = if let Some(d) = &d0 {
            Some(d.borrow())
        } else { unsafe {unreachable_unchecked()} };
        
        o1 = None;
    } else {
        // both exists
        o0 = if let Some(d) = &d0 {
            Some(d.borrow())
        } else { unsafe {unreachable_unchecked()} };
        
        o1 = if let Some(d) = &d1 {
            Some(d.borrow())
        } else { unsafe {unreachable_unchecked()} };
    }
    
    Some(f(o0, o1))    
}

pub struct Union<S0, S1, F>{
    s0: S0,
    s1: S1,
    f: F
}

impl<S0, S1, F> SparseHierarchy for Union<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy<DataType: Clone>>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount    = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy>::LevelMaskType,
    >>,
    
    F: UnionResolve<
        // v1
        <S0::Borrowed as SparseHierarchy>::DataType, 
        <S1::Borrowed as SparseHierarchy>::DataType,
    >,
{
    /// true if S0 & S1 are EXACT_HIERARCHY.
    const EXACT_HIERARCHY: bool = <S0::Borrowed as SparseHierarchy>::EXACT_HIERARCHY 
                                & <S1::Borrowed as SparseHierarchy>::EXACT_HIERARCHY;
    
    type LevelCount = <S0::Borrowed as SparseHierarchy>::LevelCount;
    
    type LevelMaskType = <S0::Borrowed as SparseHierarchy>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self:'a;
    
    type DataType = F::Out;
    type Data<'a> = F::Out where Self: 'a;

    #[inline]
    unsafe fn data<I>(&self, index: usize, level_indices: I) -> Option<Self::Data<'_>>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        let d0 = self.s0.borrow().data(index, level_indices);
        let d1 = self.s1.borrow().data(index, level_indices);
        get_data(d0, d1, &self.f)
    }

    #[inline]
    unsafe fn data_unchecked<I>(&self, index: usize, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        self.data(index, level_indices).unwrap_unchecked()
    }

    type State = State<S0, S1, F>;
}

/// [S::Mask; S::DEPTH]
type Masks<S> = ConstArrayType<
    <<S as Borrowable>::Borrowed as SparseHierarchy>::LevelMaskType,
    <<S as Borrowable>::Borrowed as SparseHierarchy>::LevelCount,
>;

pub struct State<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy>,
{
    s0: <S0::Borrowed as SparseHierarchy>::State, 
    s1: <S1::Borrowed as SparseHierarchy>::State,
    
    phantom_data: PhantomData<(S0, S1, F)>
}

impl<S0, S1, F> SparseHierarchyState for State<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy<DataType: Clone>>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount    = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy>::LevelMaskType,
    >>,
    
    F: UnionResolve<
        // v1
        <S0::Borrowed as SparseHierarchy>::DataType, 
        <S1::Borrowed as SparseHierarchy>::DataType,
    >,
{
    type This = Union<S0, S1, F>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        Self{
            s0: SparseHierarchyState::new(this.s0.borrow()), 
            s1: SparseHierarchyState::new(this.s1.borrow()),
            
            phantom_data: PhantomData
        }
    }

    #[inline]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
        // unchecked version already deal with non-existent elements
        self.select_level_node_unchecked(this, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger> (
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
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
        -> Option<<Self::This as SparseHierarchy>::Data<'a>> 
    {
        let d0 = self.s0.data(this.s0.borrow(), level_index);
        let d1 = self.s1.data(this.s1.borrow(), level_index);
        get_data(d0, d1, &this.f)
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::Data<'a> 
    {
        self.data(this, level_index).unwrap_unchecked()
    }
}

impl<S0, S1, F> Borrowable for Union<S0, S1, F>{ type Borrowed = Self; }

#[inline]
pub fn union<S0, S1, F>(s0: S0, s1: S1, f: F) -> Union<S0, S1, F>
where
    // bounds needed here for F's arguments auto-deduction
    S0: Borrowable<Borrowed: SparseHierarchy>,
    S1: Borrowable<Borrowed: SparseHierarchy<
        LevelCount    = <S0::Borrowed as SparseHierarchy>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy>::LevelMaskType,
    >>,
    
    F: UnionResolve<
        <S0::Borrowed as SparseHierarchy>::DataType, 
        <S1::Borrowed as SparseHierarchy>::DataType,
    >,
{
    Union { s0, s1, f }
} 

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array::CompactSparseArray;
    use crate::ops::union3::union;
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
        
        let union = union(&a1, &a2, |i0, i1| {
            i0.unwrap_or(&0) + i1.unwrap_or(&0)
        });
        
        assert_eq!(unsafe{ union.get_unchecked(200) }, 400);
        assert_eq!(union.get(15), Some(30));
        assert_eq!(union.get(10), Some(10));
        assert_eq!(union.get(20), None);
        
        assert_equal(union.iter(), [(10, 10), (15, 30), (100, 100), (200, 400)]);
    }
}