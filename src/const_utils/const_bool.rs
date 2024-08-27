use std::ops::{Deref, DerefMut};
use crate::BitBlock;
use crate::const_utils::cond_type::Either;
use crate::const_utils::const_int::ConstInteger;

pub trait ConstBool: Default + Copy {
    const VALUE: bool;
    /// T if true, F otherwise.
    type Conditional<T, F>;
    /// Same as [Self::Conditional] but with [ConstInteger] bounds.
    type ConditionalInt<T: ConstInteger, F: ConstInteger>: ConstInteger;
    
    fn value(self) -> bool {
        Self::VALUE
    }
}

#[derive(Default, Clone, Copy)]
pub struct ConstTrue;
impl ConstBool for ConstTrue {
    const VALUE: bool = true;
    type Conditional<T, F> = T;
    type ConditionalInt<T: ConstInteger, F: ConstInteger> = T;
}

#[derive(Default, Clone, Copy)]
pub struct ConstFalse;
impl ConstBool for ConstFalse {
    const VALUE: bool = false;
    type Conditional<T, F> = F;
    type ConditionalInt<T: ConstInteger, F: ConstInteger> = F;
}

#[cfg(test)]
mod test{
    use super::*;
    
    struct S<B: ConstBool>{
        v: B::Conditional<usize, f32>
    }
    
    fn test<B: ConstBool>(mut s: S<B>)
    where
        B::Conditional<usize, f32>: Clone
    {
        let c = s.v.clone();
    }
}