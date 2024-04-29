use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::ControlFlow;
use std::ops::ControlFlow::{Break, Continue};
use crate::bool_type::{BoolType, TrueType, FalseType};
use crate::primitive_array::{Array, ConstArray};
use crate::{Primitive, PrimitiveArray};

pub trait ConstIntVisitor {
    type Out;
    fn visit<I: ConstInteger>(&mut self, i: I) -> ControlFlow<Self::Out>;
}

/// for from..to
pub fn const_for<V: ConstIntVisitor>(from: impl ConstInteger, to: impl ConstInteger, mut v: V)
     -> ControlFlow<V::Out>
{
    to.iterate_as_range(from, &mut v)
}

/// for (from..to).rev()
pub fn const_for_rev<V: ConstIntVisitor>(from: impl ConstInteger, to: impl ConstInteger, v: V)
     -> ControlFlow<V::Out>
{
    to.iterate_as_range_rev(from, v)
}

trait ConstIntegerPrivate{
    /// const for from..N
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
pub trait ConstInteger: ConstIntegerPrivate + Default + Copy + Eq + Debug {
    const VALUE: usize;
    const DEFAULT: Self;
    
    fn value(self) -> usize {
        Self::VALUE
    }
    
    type Dec: ConstInteger;
    fn dec(self) -> Self::Dec {
        Self::Dec::default()
    }
    
    type Inc: ConstInteger;
    fn inc(self) -> Self::Inc {
        Self::Inc::default()
    }
    
    /// [T; Self::N]
    type SelfSizeArray<T>: ConstArray<Item=T, Cap=Self>;
    
    /*type IsZero: BoolType;
    fn is_zero(self) -> Self::IsZero{
        Self::IsZero::default()
    }*/ 
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ConstInt<const N: usize>;

impl<const N: usize> Default for ConstInt<N>{
    fn default() -> Self {
        if N == MAX{
            panic!()
        }
        Self
    }
}

impl<const N: usize> Debug for ConstInt<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ConstInt<{}>", N)
    }
}

macro_rules! gen_const_int {
    (first $i:literal) => {
        impl ConstIntegerPrivate for ConstInt<$i>{
            fn iterate_as_range<V: ConstIntVisitor>(self, from: impl ConstInteger, visitor: &mut V) 
                -> ControlFlow<V::Out> 
            {
                Continue(())
            }
            
            fn iterate_as_range_rev<V: ConstIntVisitor>(self, from: impl ConstInteger, visitor: V) 
                -> ControlFlow<V::Out> 
            {
                Continue(())
            }
        }
        impl ConstInteger for ConstInt<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Dec = ConstIntInvalid;
            type Inc = ConstInt<{$i+1}>;
            type SelfSizeArray<T> = [T; $i];

            //type IsZero = TrueType;
        }
    };
    ($i:literal) => {
        impl ConstIntegerPrivate for ConstInt<$i>{}
        impl ConstInteger for ConstInt<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Dec = ConstInt<{$i-1}>;
            type Inc = ConstInt<{$i+1}>;
            type SelfSizeArray<T> = [T; $i];
            
            //type IsZero = FalseType;
        }
    };
    (last $i:literal) => {
        impl ConstIntegerPrivate for ConstInt<$i>{}
        impl ConstInteger for ConstInt<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Dec = ConstInt<{$i-1}>;
            type Inc = ConstIntInvalid;
            type SelfSizeArray<T> = [T; $i];
            
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
impl ConstIntegerPrivate for ConstInt<MAX> {}
impl ConstInteger for ConstInt<MAX>{ 
    const VALUE  : usize = MAX;
    const DEFAULT: Self  = ConstInt::<MAX>;
    
    type Dec = ConstInt<MAX>;
    type Inc = ConstInt<MAX>;
    type SelfSizeArray<T> = [T; MAX];
    
    //type IsZero = FalseType;
}
type ConstIntInvalid = ConstInt<MAX>;

#[cfg(test)]
mod test{
    use super::*;
    
    #[test]
    fn test(){
        type One  = ConstInt<1>;
        type Zero = ConstInt<0>;
        type Two  = ConstInt<2>;
        
        assert_eq!(One::DEFAULT.inc(), Two::DEFAULT);         
        assert_eq!(One::DEFAULT.dec(), Zero::DEFAULT);
    }
    
    #[test]
    fn loop_test(){
        const_for(ConstInt::<0>, ConstInt::<3>, V);
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
        const_for_rev(ConstInt::<0>, ConstInt::<3>, V);
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