use std::fmt;
use std::fmt::{Debug, Display};
use crate::const_utils::const_array::ConstArray;
use crate::utils::Array;

/// Ala C++ integral_constant.
/// 
/// We need this machinery to fight against Rust's half-baked const evaluation. 
/// With this, we can do const {Self::N+1} in stable rust. 
pub trait ConstInteger: Default + Copy + Eq + Debug + 'static {
    const VALUE: usize;
    /// const Default::default()
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
    type ArrayOf<T>: ConstArray<Item=T, Cap=Self>;
    
    /// Same as [Self::ArrayOf], but with additional type bounds.
    /// 
    /// N.B. We can't **just** forward Copy for ArrayOf if T: Copy in Rust.
    type CopyArrayOf<T: Copy>: ConstArray<Item=T, Cap=Self, DecArray:Copy> + Copy;
    
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
        impl ConstInteger for ConstUsize<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstUsize::<$i>;
            
            type Dec    = ConstIntInvalid;
            type SatDec = ConstUsize<{$i}>;
            type Inc = ConstUsize<{$i+1}>;
            type ArrayOf<T> = [T; $i];
            type CopyArrayOf<T: Copy> = [T; $i];

            //type IsZero = TrueType;
        }
    };
    ($i:literal) => {
        impl ConstInteger for ConstUsize<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstUsize::<$i>;
            
            type Dec    = ConstUsize<{$i-1}>;
            type SatDec = ConstUsize<{$i-1}>;
            type Inc = ConstUsize<{$i+1}>;
            type ArrayOf<T> = [T; $i];
            type CopyArrayOf<T: Copy> = [T; $i];
            
            //type IsZero = FalseType;
        }
    };
    (last $i:literal) => {
        impl ConstInteger for ConstUsize<$i>{ 
            const VALUE  : usize = $i;
            const DEFAULT: Self = ConstUsize::<$i>;
            
            type Dec    = ConstUsize<{$i-1}>;
            type SatDec = ConstUsize<{$i-1}>;
            type Inc = ConstIntInvalid;
            type ArrayOf<T> = [T; $i];
            type CopyArrayOf<T: Copy> = [T; $i];
            
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

// TODO: gen_single
gen_const_seq!(63,64;65);
gen_const_seq!(127,128;129);
gen_const_seq!(255,256;257);

const MAX: usize = usize::MAX;
impl ConstInteger for ConstUsize<MAX>{ 
    const VALUE  : usize = MAX;
    const DEFAULT: Self  = ConstUsize::<MAX>;
    
    type Dec    = ConstUsize<MAX>;
    type SatDec = ConstUsize<MAX>;
    type Inc = ConstUsize<MAX>;
    type ArrayOf<T> = [T; MAX];
    type CopyArrayOf<T: Copy> = [T; MAX];
    
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
}