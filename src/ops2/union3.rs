use std::marker::PhantomData;
use std::borrow::Borrow;
use std::hint::unreachable_unchecked;
use std::mem::MaybeUninit;
use std::ops::{BitAnd, BitOr};
use crate::const_utils::{ConstArray, ConstArrayType, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::{BitBlock, SparseHierarchy, SparseHierarchyState};
use crate::bit_queue::BitQueue;
use crate::utils::{Array, Borrowable, FnRR, Take};


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
}


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


pub struct Union3<S0, S1, F>{
    s0: S0,
    s1: S1,
    f: F
}

impl<S0, S1, F> SparseHierarchy2 for Union3<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy2<DataType: Clone>>,
    S1: Borrowable<Borrowed: SparseHierarchy2<
        LevelCount    = <S0::Borrowed as SparseHierarchy2>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType,
        
        //v2
        DataType = <S0::Borrowed as SparseHierarchy2>::DataType,
    >>,
    
    F: UnionResolve<
        // v1
        /*<S0::Borrowed as SparseHierarchy2>::DataType, 
        <S1::Borrowed as SparseHierarchy2>::DataType,*/
        
        // v2
        <S0::Borrowed as SparseHierarchy2>::DataType,
        <S0::Borrowed as SparseHierarchy2>::DataType,
        Out = <S0::Borrowed as SparseHierarchy2>::DataType,

    >,
    
    // &Mask & &Mask
    //for<'a> &'a <S0::Borrowed as SparseHierarchy2>::LevelMaskType: BitOr<&'a <S0::Borrowed as SparseHierarchy2>::LevelMaskType, Output = <S0::Borrowed as SparseHierarchy2>::LevelMaskType>,
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
    
    /*masks0: Masks<S0>,
    masks1: Masks<S1>,*/
    
    phantom_data: PhantomData<(S0, S1, F)>
}

impl<S0, S1, F> SparseHierarchyState2 for State<S0, S1, F>
where
    S0: Borrowable<Borrowed: SparseHierarchy2<DataType: Clone>>,
    S1: Borrowable<Borrowed: SparseHierarchy2<
        LevelCount    = <S0::Borrowed as SparseHierarchy2>::LevelCount,
        LevelMaskType = <S0::Borrowed as SparseHierarchy2>::LevelMaskType,
        
        //v2
        DataType = <S0::Borrowed as SparseHierarchy2>::DataType,
    >>,
    
    F: UnionResolve<
        // v1
        /*<S0::Borrowed as SparseHierarchy2>::DataType, 
        <S1::Borrowed as SparseHierarchy2>::DataType,*/
        
        // v2
        <S0::Borrowed as SparseHierarchy2>::DataType,
        <S0::Borrowed as SparseHierarchy2>::DataType,
        Out = <S0::Borrowed as SparseHierarchy2>::DataType,
    >,
    
    // Actually, we can just use Take here, since as for now, masks always SIMD values.
    // &Mask & &Mask
    //for<'a> &'a <S0::Borrowed as SparseHierarchy2>::LevelMaskType: BitOr<&'a <S0::Borrowed as SparseHierarchy2>::LevelMaskType, Output = <S0::Borrowed as SparseHierarchy2>::LevelMaskType>,
{
    type This = Union3<S0, S1, F>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        Self{
            s0: SparseHierarchyState2::new(this.s0.borrow()), 
            s1: SparseHierarchyState2::new(this.s1.borrow()),
            
            /*// Just MaybeEmpty, instead?
            masks0: Array::from_fn(|_| BitBlock::zero()), 
            masks1: Array::from_fn(|_| BitBlock::zero()),*/
            
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
        /*// v1
        // s0
        let contains0 = if N::VALUE == 0 { true } else {
            let parent_level_n = level_n.dec().value();
            let parent_mask0 = self.masks0.as_ref().get_unchecked(parent_level_n).borrow();
            parent_mask0.get_bit(level_index)
        };
        let mask0 = if contains0 {
            self.s0.select_level_node_unchecked(
                this.s0.borrow(), level_n, level_index
            ).take_or_clone()
        } else {
            BitBlock::zero()
        };
        *self.masks0.as_mut().get_unchecked_mut(level_n.value()) = mask0.clone();
        
        // s1
        let contains1 = if N::VALUE == 0 { true } else {
            let parent_level_n = level_n.dec().value();
            let parent_mask1 = self.masks1.as_ref().get_unchecked(parent_level_n).borrow();
            parent_mask1.get_bit(level_index)
        };
        let mask1 = if contains1 {
            self.s1.select_level_node_unchecked(
                this.s1.borrow(), level_n, level_index
            ).take_or_clone()
        } else {
            BitBlock::zero()
        };
        *self.masks1.as_mut().get_unchecked_mut(level_n.value()) = mask1.clone();*/
        
/*        // v1.1
        // s0
        let contains0 = if N::VALUE == 0 { true } else {
            let parent_level_n = level_n.dec().value();
            let parent_mask0 = self.masks0.as_ref().get_unchecked(parent_level_n).borrow();
            parent_mask0.get_bit(level_index)
        };
        let mut mask0 = self.s0.select_level_node_unchecked(
            this.s0.borrow(), level_n, level_index
        ).take_or_clone();
        //let mask0 = if contains0 { mask0 } else {BitBlock::zero()};
        //mask0.as_array_mut().as_mut()[0] *= contains0 as u64; 
        *self.masks0.as_mut().get_unchecked_mut(level_n.value()) = mask0.clone();
        
        // s1
        let contains1 = if N::VALUE == 0 { true } else {
            let parent_level_n = level_n.dec().value();
            let parent_mask1 = self.masks1.as_ref().get_unchecked(parent_level_n).borrow();
            parent_mask1.get_bit(level_index)
        };
        let mut mask1 = self.s1.select_level_node_unchecked(
            this.s1.borrow(), level_n, level_index
        ).take_or_clone();
        //let mask1 = if contains1 { mask1 } else { BitBlock::zero() };
        //mask1.as_array_mut().as_mut()[0] *= contains1 as u64;
        *self.masks1.as_mut().get_unchecked_mut(level_n.value()) = mask1.clone();   */ 
        
        // v2
        let mask0 =
            self.s0.select_level_node(
                this.s0.borrow(), level_n, level_index
            ).take_or_clone();
            
        let mask1 =
            self.s1.select_level_node(
                this.s1.borrow(), level_n, level_index
            ).take_or_clone();

        mask0 | mask1
    }

    #[inline]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> Option<<Self::This as SparseHierarchy2>::Data<'a>> 
    {
        let d0 = self.s0.data(this.s0.borrow(), level_index);
        let d1 = self.s1.data(this.s1.borrow(), level_index);
        
        if d0.is_none() & d1.is_none(){
            return None;
        }
        
        // Looks like compiler optimize away these re-borrow transformations.
        let o0;
        let o1;
        if d0.is_none(){
            o0 = None;
            
            // we know that d1 exists.
            o1 = if let Some(d) = &d1 {
                Some(d.borrow())
            } else { unreachable_unchecked() };
        } else if d1.is_none(){
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
        
/*        {
            let d0 = self.s0.data(this.s0.borrow(), level_index);
            let d1 = self.s1.data(this.s1.borrow(), level_index) ;
            
            // Looks like compiler optimize away these transformations.
            let o0 = if let Some(d) = &d0 {
                Some(d.borrow())
            } else {
                None
            };
            
            let o1 = if let Some(d) = &d1 {
                Some(d.borrow())
            } else {
                None
            };            
            
            //return (this.f)(Some(d0.borrow()), Some(d1.borrow()));         
            return (this.f)(o0, o1);
        }
*/
        
        /*let parent_mask0 = self.masks0.as_ref().last().unwrap_unchecked().borrow();
        let contains0 = parent_mask0.get_bit(level_index);
        
        let parent_mask1 = self.masks1.as_ref().last().unwrap_unchecked().borrow();
        let contains1 = parent_mask1.get_bit(level_index);
        
        /*// v1
        {
            let mut v0 = MaybeUninit::uninit();
            let d0 = if contains0 {
                v0.write( self.s0.data_unchecked(this.s0.borrow(), level_index) ); 
                Some(v0.assume_init_ref().borrow())
            } else {
                None
            };
            
            let mut v1 = MaybeUninit::uninit();
            let d1 = if contains1 {
                v1.write( self.s1.data_unchecked(this.s1.borrow(), level_index) ); 
                Some(v1.assume_init_ref().borrow())
            } else {
                None
            };
            
            let mask = (this.f)(d0, d1);
            
            MaybeUninit::assume_init(v0);
            MaybeUninit::assume_init(v1);
            
            mask
        }*/

        // v2
        {
            if contains0 & contains1 {
                let d0 = self.s0.data_unchecked(
                    this.s0.borrow(), level_index
                );
                let d1 = self.s1.data_unchecked(
                    this.s1.borrow(), level_index
                );
                (this.f)(Some(d0.borrow()), Some(d1.borrow()))
            } else if contains0 {
                self.s0.data_unchecked(
                    this.s0.borrow(), level_index
                ).take_or_clone()      
            } else {
                self.s1.data_unchecked(
                    this.s1.borrow(), level_index
                ).take_or_clone()
            }
        }*/
    }
}

impl<S0, S1, F> Borrowable for Union3<S0, S1, F>{ type Borrowed = Self; }

#[inline]
pub fn union3<S0, S1, F>(s0: S0, s1: S1, f: F) -> Union3<S0, S1, F>
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
    Union3 { s0, s1, f }
} 

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array2::CompactSparseArray2;
    use crate::ops2::union2::union2;
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
        
        let union = union2(&a1, &a2, |i0, i1| {
            i0.unwrap_or(&0) + i1.unwrap_or(&0)
        });
        
        assert_equal(union.iter(), [(10, 10), (15, 30), (100, 100), (200, 400)]);
    }
    
    // TODO: remove
    #[test]
    fn regression_test(){
        type Array = CompactSparseArray2<usize, 2>;
        let mut compact_array1 = Array::default();
        let mut compact_array2 = Array::default();
        for i in 0..100{
            *compact_array1.get_or_insert(i*20) = i;
            *compact_array2.get_or_insert(i*20) = i;
        }

        {
            let union = union2(compact_array1, compact_array2, |v0, v1|{
                let v0 = v0.map_or(0, |v|*v);
                let v1 = v1.map_or(0, |v|*v);
                v0 + v1
            });
            
            let mut s = 0;
            for (_, i) in union.iter(){
                s += i;
            }
            //s
        }
    }
}