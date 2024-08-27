use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::ControlFlow;
use std::ops::ControlFlow::{Break, Continue};
use crate::const_utils::const_bool::{ConstBool, ConstTrue, ConstFalse};
use crate::const_utils::const_array::{ConstArray};
use crate::utils::Array;

pub trait ConstIntVisitor {
    type Out;
    fn visit<I: ConstInteger>(&mut self, i: I) -> ControlFlow<Self::Out>;
}

/// for from..to
#[inline]
pub fn const_for<V: ConstIntVisitor>(from: impl ConstInteger, to: impl ConstInteger, mut v: V)
     -> ControlFlow<V::Out>
{
    to.iterate_as_range(from, &mut v)
}

/// for (from..to).rev()
#[inline]
pub fn const_for_rev<V: ConstIntVisitor>(from: impl ConstInteger, to: impl ConstInteger, v: V)
     -> ControlFlow<V::Out>
{
    to.iterate_as_range_rev(from, v)
}

trait ConstIntegerPrivate{
    /// const for from..N
    #[inline(always)]
    fn iterate_as_range<V: ConstIntVisitor>(self, from: impl ConstInteger, visitor: &mut V)
       -> ControlFlow<V::Out>
    where
        Self: ConstInteger
    {
        let ctrl = self.dec().iterate_as_range(from, visitor);
        if ctrl.is_continue() {
            if self.value() == from.value() {
                Continue(())
            } else {            
                visitor.visit(self.dec())
            }
        } else {
            ctrl
        }
    }
    
    /// const for (from..N).rev()
    #[inline(always)]
    fn iterate_as_range_rev<V: ConstIntVisitor>(self, from: impl ConstInteger, mut visitor: V)
        -> ControlFlow<V::Out>
    where
        Self: ConstInteger
    {
        let ctrl = visitor.visit(self.dec());
        if ctrl.is_continue(){
            if self.value() == from.value() {
                Continue(())
            } else {
                self.dec().iterate_as_range_rev(from, visitor)
            }
        } else {
            ctrl
        }
    }
}

/// Ala C++ integral_constant.
/// 
/// We need this machinery to fight against Rust's half-baked const evaluation. 
/// With this, we can do const {Self::N+1} in stable rust. 
pub trait ConstInteger: ConstIntegerPrivate + Default + Copy + Eq + Debug + 'static {
    const VALUE: usize;
    const DEFAULT: Self;
    
    #[inline]
    fn value(self) -> usize {
        Self::VALUE
    }
    
    /// Saturating decrement
    type SatDec: ConstInteger;
    /// Saturating decrement
    #[inline]
    fn sat_dec(self) -> Self::Dec {
        Self::Dec::default()
    }
    
    type Dec: ConstInteger;
    #[inline]
    fn dec(self) -> Self::Dec {
        Self::Dec::default()
    }
    
    type Inc: ConstInteger;
    #[inline]
    fn inc(self) -> Self::Inc {
        Self::Inc::default()
    }
    
    /// [T; Self::N]
    type SelfSizeArray<T>: ConstArray<Item=T, Cap=Self>;
    
    /// Same as [Self::SelfSizeArray], but with additional type bounds.
    /// 
    /// N.B. We can't **just** forward Copy for SelfSizeArray if T: Copy in Rust.
    type SelfSizeCopyArray<T: Copy>: ConstArray<Item=T, Cap=Self, DecArray:Copy> + Copy;
    
    /*type IsZero: BoolType;
    fn is_zero(self) -> Self::IsZero{
        Self::IsZero::default()
    }*/ 
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ConstUsize<const N: usize>;

impl<const N: usize> Default for ConstUsize<N>{
    #[inline]
    fn default() -> Self {
        if N == MAX{
            panic!()
        }
        Self
    }
}

impl<const N: usize> Debug for ConstUsize<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ConstInt<{}>", N)
    }
}

macro_rules! gen_const_int {
    (first $i:literal) => {
        impl ConstIntegerPrivate for ConstUsize<$i>{
            #[inline]
            fn iterate_as_range<V: ConstIntVisitor>(self, from: impl ConstInteger, visitor: &mut V) 
                -> ControlFlow<V::Out> 
            {
                Continue(())
            }
            
            #[inline]
            fn iterate_as_range_rev<V: ConstIntVisitor>(self, from: impl ConstInteger, visitor: V) 
                -> ControlFlow<V::Out> 
            {
                Continue(())
            }
        }
        impl ConstInteger for ConstUsize<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstUsize::<$i>;
            
            type Dec    = ConstIntInvalid;
            type SatDec = ConstUsize<{$i}>;
            type Inc = ConstUsize<{$i+1}>;
            type SelfSizeArray<T> = [T; $i];
            type SelfSizeCopyArray<T: Copy> = [T; $i];

            //type IsZero = TrueType;
        }
    };
    ($i:literal) => {
        impl ConstIntegerPrivate for ConstUsize<$i>{}
        impl ConstInteger for ConstUsize<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstUsize::<$i>;
            
            type Dec    = ConstUsize<{$i-1}>;
            type SatDec = ConstUsize<{$i-1}>;
            type Inc = ConstUsize<{$i+1}>;
            type SelfSizeArray<T> = [T; $i];
            type SelfSizeCopyArray<T: Copy> = [T; $i];
            
            //type IsZero = FalseType;
        }
    };
    (last $i:literal) => {
        impl ConstIntegerPrivate for ConstUsize<$i>{}
        impl ConstInteger for ConstUsize<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstUsize::<$i>;
            
            type Dec    = ConstUsize<{$i-1}>;
            type SatDec = ConstUsize<{$i-1}>;
            type Inc = ConstIntInvalid;
            type SelfSizeArray<T> = [T; $i];
            type SelfSizeCopyArray<T: Copy> = [T; $i];
            
            //type IsZero = FalseType;
        }
    };
}

macro_rules! gen_const_seq {
    ($first_i:literal, $($i:literal),+; $last_i:literal) => {
        gen_const_int!(first $first_i);
        $(
            gen_const_int!($i);
        )+
        gen_const_int!(last $last_i);
    }
}

gen_const_seq!(0,1,2,3,4,5,6,7,8;9);

const MAX: usize = usize::MAX;
impl ConstIntegerPrivate for ConstUsize<MAX> {}
impl ConstInteger for ConstUsize<MAX>{ 
    const VALUE  : usize = MAX;
    const DEFAULT: Self  = ConstUsize::<MAX>;
    
    type Dec    = ConstUsize<MAX>;
    type SatDec = ConstUsize<MAX>;
    type Inc = ConstUsize<MAX>;
    type SelfSizeArray<T> = [T; MAX];
    type SelfSizeCopyArray<T: Copy> = [T; MAX];
    
    //type IsZero = FalseType;
}
type ConstIntInvalid = ConstUsize<MAX>;

#[cfg(test)]
mod test{
    use super::*;
    
    #[test]
    fn test(){
        type Zero = ConstUsize<0>;
        type One  = ConstUsize<1>;
        type Two  = ConstUsize<2>;
        
        assert_eq!(One::DEFAULT.inc(), Two::DEFAULT);         
        assert_eq!(One::DEFAULT.dec(), Zero::DEFAULT);
    }
    
    #[test]
    fn loop_test(){
        const_for(ConstUsize::<0>, ConstUsize::<3>, V);
        struct V;
        impl ConstIntVisitor for V{
            type Out = ();
            fn visit<I: ConstInteger>(&mut self, i: I) -> ControlFlow<()> {
                println!("{:?}", i);
                Continue(())
            }
        }
    }
    
    #[test]
    fn loop_rev_test(){
        const_for_rev(ConstUsize::<0>, ConstUsize::<3>, V);
        struct V;
        impl ConstIntVisitor for V{
            type Out = ();
            fn visit<I: ConstInteger>(&mut self, i: I) -> ControlFlow<()> {
                println!("{:?}", i);
                Continue(())
            }
        }
    }
}