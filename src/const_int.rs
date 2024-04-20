use std::fmt;
use std::fmt::{Debug, Display};
use std::ops::RangeTo;

pub trait IntVisitor{
    fn visit<I: ConstInteger>(&mut self, i: I);
}

pub fn const_for<I: ConstInteger, V: IntVisitor>(range: RangeTo<I>, mut v: V){
    range.end.iterate_as_range(&mut v);
}

/// Ala C++ integral_constant.
/// 
/// We need this machinery to fight against Rust's half-baked const evaluation. 
/// With this, we can do const {Self::N+1} in stable rust. 
pub trait ConstInteger: Default + Copy + Eq + Debug {
    const N: usize;
    const DEFAULT: Self;
    
    type Prev: ConstInteger;
    fn prev(self) -> Self::Prev{
        Self::Prev::DEFAULT
    }
    
    type Next: ConstInteger;
    fn next(self) -> Self::Next{
        Self::Next::DEFAULT
    }
    
    /// const for 0..N
    fn iterate_as_range<V: IntVisitor>(self, visitor: &mut V){
        self.prev().iterate_as_range(visitor);
        visitor.visit(self.prev());    
    }    
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
            const N: usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Prev = ConstIntInvalid;
            type Next = ConstInt<{$i+1}>;
            
            fn iterate_as_range<V: IntVisitor>(self, visitor: &mut V){}
        }
    };
    ($i:literal) => {
        impl ConstInteger for ConstInt<$i>{ 
            const N: usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Prev = ConstInt<{$i-1}>;
            type Next = ConstInt<{$i+1}>;
        }
    };
    (last $i:literal) => {
        impl ConstInteger for ConstInt<$i>{ 
            const N: usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Prev = ConstInt<{$i-1}>;
            type Next = ConstIntInvalid;
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
    const N      : usize = panic!();
    const DEFAULT: Self  = panic!();
    
    type Prev = ConstIntInvalid;
    type Next = ConstIntInvalid;
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
        ConstInt::<3>.iterate_as_range(&mut V);
        const_for(..ConstInt::<0>, V);
        struct V;
        impl IntVisitor for V{
            fn visit<I: ConstInteger>(&mut self, i: I) {
                println!("{:?}", i);
            }
        }
    }
}