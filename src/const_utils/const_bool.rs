use std::ops::{Deref, DerefMut};
use crate::const_utils::const_int::ConstInteger;
use crate::utils::function::NullaryFunction;

pub trait ConstBool: Default + Copy {
    const VALUE: bool;
    
    /// T if true, F otherwise.
    type Conditional<T, F>;
    
    /// Same as [Self::Conditional] but with [ConstBool] bounds.
    type ConditionalBool<T: ConstBool, F: ConstBool>: ConstBool;
    
    // Not used.
    /// Same as [Self::Conditional] but with [ConstInteger] bounds.
    type ConditionalInt<T: ConstInteger, F: ConstInteger>: ConstInteger;
    
    fn value(self) -> bool {
        Self::VALUE
    }
    
    fn conditional_exec<TrueFn, FalseFn>(t: TrueFn, f: FalseFn) 
        -> Self::Conditional<TrueFn::Output, FalseFn::Output>
    where 
        TrueFn: NullaryFunction, FalseFn: NullaryFunction;
    
    type And<Other: ConstBool>: ConstBool;
}

#[derive(Default, Clone, Copy)]
pub struct ConstTrue;
impl ConstBool for ConstTrue {
    const VALUE: bool = true;
    type Conditional<T, F> = T;
    type ConditionalBool<T: ConstBool, F: ConstBool> = T;
    type ConditionalInt<T: ConstInteger, F: ConstInteger> = T;
    
    type And<Other: ConstBool> = Other::ConditionalBool<ConstTrue, ConstFalse>;
    
    #[inline]
    fn conditional_exec<TrueFn, FalseFn>(t: TrueFn, _: FalseFn) 
        -> Self::Conditional<TrueFn::Output, FalseFn::Output>
    where 
        TrueFn: NullaryFunction, FalseFn: NullaryFunction
    {
        t.exec()
    }
}
pub trait IsConstTrue: ConstBool {}
impl IsConstTrue for ConstTrue{}

#[derive(Default, Clone, Copy)]
pub struct ConstFalse;
impl ConstBool for ConstFalse {
    const VALUE: bool = false;
    type Conditional<T, F> = F;
    type ConditionalBool<T: ConstBool, F: ConstBool> = F;
    type ConditionalInt<T: ConstInteger, F: ConstInteger> = F;
    
    type And<Other: ConstBool> = Other::ConditionalBool<ConstFalse, ConstFalse>;
    
    #[inline]
    fn conditional_exec<TrueFn, FalseFn>(_: TrueFn, f: FalseFn) 
        -> Self::Conditional<TrueFn::Output, FalseFn::Output>
    where 
        TrueFn: NullaryFunction, FalseFn: NullaryFunction
    {
        f.exec()
    }    
}
pub trait IsConstFalse: ConstBool {}
impl IsConstFalse for ConstFalse{}

pub type ConstAnd<L, R> = <L as ConstBool>::And<R>;

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
    
    #[test]
    fn and_test(){
        type T0 = ConstTrue;
        type T1 = ConstTrue;
        type A = ConstAnd<T0, T1>;
        assert_eq!(A::VALUE, true);
        fn test<B: IsConstTrue>(){};
        test::<A>();
    }
}