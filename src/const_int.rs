use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::ControlFlow;
use std::ops::ControlFlow::{Break, Continue};
use crate::bool_type::{BoolType, TrueType, FalseType};
use crate::primitive_array::Array;

pub trait ConstIntVisitor {
    fn visit<I: ConstInteger>(&mut self, i: I) -> ControlFlow<()>;
}

/// for from..to
pub fn const_for(from: impl ConstInteger, to: impl ConstInteger, mut v: impl ConstIntVisitor)
     -> ControlFlow<()>
{
    to.iterate_as_range(from, &mut v)
}

/// for (from..to).rev()
pub fn const_for_rev(from: impl ConstInteger, to: impl ConstInteger, v: impl ConstIntVisitor)
     -> ControlFlow<()>
{
    to.iterate_as_range_rev(from, v)
}

/// Ala C++ integral_constant.
/// 
/// We need this machinery to fight against Rust's half-baked const evaluation. 
/// With this, we can do const {Self::N+1} in stable rust. 
pub trait ConstInteger: Default + Copy + Eq + Debug {
    const VALUE: usize;
    const DEFAULT: Self;
    
    fn value(self) -> usize {
        Self::VALUE
    }
    
    type Prev: ConstInteger;
    fn prev(self) -> Self::Prev{
        Self::Prev::default()
    }
    
    type Next: ConstInteger;
    fn next(self) -> Self::Next{
        Self::Next::default()
    }
    
    /// const for from..N
    fn iterate_as_range(self, from: impl ConstInteger, visitor: &mut impl ConstIntVisitor)
       -> ControlFlow<()>
    {
        let ctrl = self.prev().iterate_as_range(from, visitor);
        if ctrl.is_continue() {
            if self.value() == from.value() {
                Continue(())
            } else {            
                visitor.visit(self.prev())
            }
        } else {
            Break(())
        }
    }
    
    /// const for (from..N).rev()
    fn iterate_as_range_rev<V: ConstIntVisitor>(self, from: impl ConstInteger, mut visitor: V)
        -> ControlFlow<()>
    {
        let ctrl = visitor.visit(self.prev());
        if ctrl.is_continue(){
            if self.value() == from.value() {
                Continue(())
            } else {
                self.prev().iterate_as_range_rev(from, visitor)
            }
        } else {
            Break(())
        }
    }
    
    /// [T; Self::N]
    type Array<T>: Array<Item = T>;
    
    /*type IsZero: BoolType;
    fn is_zero(self) -> Self::IsZero{
        Self::IsZero::default()
    }*/ 
}

#[derive(Default, Copy, Clone, Eq, PartialEq)]
pub struct ConstInt<const N: usize>;

impl<const N: usize> Debug for ConstInt<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ConstInt<{}>", N)
    }
}

macro_rules! gen_const_int {
    (first $i:literal) => {
        impl ConstInteger for ConstInt<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Prev = ConstIntInvalid;
            type Next = ConstInt<{$i+1}>;
            type Array<T> = [T; $i];
            
            fn iterate_as_range(self, from: impl ConstInteger, visitor: &mut impl ConstIntVisitor) 
                -> ControlFlow<()> 
            {
                Continue(())
            }
            
            fn iterate_as_range_rev<V: ConstIntVisitor>(self, from: impl ConstInteger, visitor: V) 
                -> ControlFlow<()> 
            {
                Continue(())
            }
            
            //type IsZero = TrueType;
        }
    };
    ($i:literal) => {
        impl ConstInteger for ConstInt<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Prev = ConstInt<{$i-1}>;
            type Next = ConstInt<{$i+1}>;
            type Array<T> = [T; $i];
            
            //type IsZero = FalseType;
        }
    };
    (last $i:literal) => {
        impl ConstInteger for ConstInt<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Prev = ConstInt<{$i-1}>;
            type Next = ConstIntInvalid;
            type Array<T> = [T; $i];            
            
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

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ConstIntInvalid;
impl ConstInteger for ConstIntInvalid{
    #[doc(hidden)]
    const VALUE  : usize = panic!();
    #[doc(hidden)]
    const DEFAULT: Self  = panic!();
    
    type Prev = ConstIntInvalid;
    type Next = ConstIntInvalid;
    type Array<T> = [T; 0];
    
    //type IsZero = FalseType;
    
    fn value(self) -> usize {
        panic!()
    }
}
impl Default for ConstIntInvalid{
    fn default() -> Self {
        panic!()
    }
}

#[cfg(test)]
mod test{
    use super::*;
    
    #[test]
    fn test(){
        type One  = ConstInt<1>;
        type Zero = ConstInt<0>;
        type Two  = ConstInt<2>;
        
        assert_eq!(One::DEFAULT.next(), Two::DEFAULT);         
        assert_eq!(One::DEFAULT.prev(), Zero::DEFAULT);
    }
    
    #[test]
    fn loop_test(){
        const_for(ConstInt::<0>, ConstInt::<3>, V);
        struct V;
        impl ConstIntVisitor for V{
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
            fn visit<I: ConstInteger>(&mut self, i: I) -> ControlFlow<()> {
                println!("{:?}", i);
                Continue(())
            }
        }
    }    
}