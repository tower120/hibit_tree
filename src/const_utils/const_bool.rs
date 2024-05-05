use std::ops::{Deref, DerefMut};
use crate::BitBlock;
use crate::const_utils::cond_type::CondType;
use crate::const_utils::const_int::ConstInteger;

pub trait ConstBool: Default + Copy{
    const VALUE: bool;
    /// T if true, F otherwise.
    type CondType<T, F>;
    /// Same as [CondType] but with [ConstInteger] bounds.
    type CondInt<T: ConstInteger, F: ConstInteger>: ConstInteger;
}

#[derive(Default, Clone, Copy)]
pub struct ConstTrue;
impl ConstBool for ConstTrue {
    const VALUE: bool = true;
    type CondType<T, F> = T;
    type CondInt<T: ConstInteger, F: ConstInteger> = T;
}

#[derive(Default, Clone, Copy)]
pub struct ConstFalse;
impl ConstBool for ConstFalse {
    const VALUE: bool = false;
    type CondType<T, F> = F;
    type CondInt<T: ConstInteger, F: ConstInteger> = F;
}

#[cfg(test)]
mod test{
    use super::*;
    
    struct S<B: ConstBool>{
        v: B::CondType<usize, f32>
    }
    
    fn test<B: ConstBool>(mut s: S<B>)
    where
        B::CondType<usize, f32>: Clone
    {
        let c = s.v.clone();
    }
}