use std::fmt::Debug;
use std::marker::PhantomData;

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
}

#[derive(Default, Copy, Clone, Eq, PartialEq, Debug)]
pub struct ConstInt<const N: usize>;

macro_rules! gen_const_int {
    (first $i:literal) => {
        impl ConstInteger for ConstInt<$i>{ 
            const N: usize = $i;
            const DEFAULT: Self = ConstInt::<$i>;
            
            type Prev = ConstIntInvalid;
            type Next = ConstInt<{$i+1}>;
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

gen_const_seq!(0,1,2,3,4,5,6,7,8,9,10,11,12;13);

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ConstIntInvalid(PhantomData<()>);
impl ConstInteger for ConstIntInvalid{
    #[doc(hidden)]
    const N: usize = panic!();
    #[doc(hidden)]
    const DEFAULT: Self = panic!();
    
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
        type One  = ConstInt::<1>;
        type Zero = ConstInt::<0>;
        type Two  = ConstInt::<2>;
        
        assert_eq!(One::DEFAULT.next(), Two::DEFAULT);         
        assert_eq!(One::DEFAULT.prev(), Zero::DEFAULT);
    }
}