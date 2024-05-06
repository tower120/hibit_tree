use std::mem::MaybeUninit;
use std::ptr::NonNull;
use crate::const_utils::{ConstBool, ConstFalse, ConstTrue};

pub enum Either<T, F>{
    True(T),
    False(F)
}

/// Ala C++ std::conditional_t with ability to safely access
/// underlying value.
#[repr(transparent)]
pub struct CondType<B: ConstBool, T, F>(
    pub B::Conditional<T, F>
);

impl<T, F> From<T> for CondType<ConstTrue, T, F>{
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T, F> From<F> for CondType<ConstFalse, T, F>{
    fn from(value: F) -> Self {
        Self(value)
    }
}
impl<B: ConstBool, T, F> Default for CondType<B, T, F>
where 
    T: Default,
    F: Default,
{
    fn default() -> Self {
        Self::new(||Default::default(), ||Default::default())
    }
}

impl<B: ConstBool, T, F> Clone for CondType<B, T, F>
where 
    T: Clone,
    F: Clone,
{
    #[inline(always)]
    fn clone(&self) -> Self {
        use Either::*;
        Self::new(
            || match self.get() {
                True(v)  => v.clone(),
                _ => unsafe{ std::hint::unreachable_unchecked() },
            },
            || match self.get() {
                False(v) => v.clone(),
                _ => unsafe{ std::hint::unreachable_unchecked() },
            }
        )        
    }
}

impl<B: ConstBool, T, F> CondType<B, T, F>{
    #[inline(always)]
    pub fn new(
        mut true_fn : impl FnMut() -> T,
        mut false_fn: impl FnMut() -> F,
    ) -> Self {
        let mut this = MaybeUninit::uninit();
        
        if B::VALUE {
            let this_ptr = this.as_mut_ptr() as *mut u8 as *mut T;
            unsafe{ this_ptr.write(true_fn()); }
        } else {
            let this_ptr = this.as_mut_ptr() as *mut u8 as *mut F;
            unsafe{ this_ptr.write(false_fn()); }
        }
        
        unsafe{ this.assume_init() }
    }
    
    #[inline(always)]
    pub fn get(&self) -> Either<&T, &F>{
        if B::VALUE {
            let p: NonNull<T> = NonNull::from(&self.0).cast();
            Either::True(unsafe{p.as_ref()})
        } else {
            let p: NonNull<F> = NonNull::from(&self.0).cast();
            Either::False(unsafe{p.as_ref()})
        }
    }
    
    #[inline(always)]
    pub fn get_mut(&mut self) -> Either<&mut T, &mut F>{
        if B::VALUE {
            let mut p: NonNull<T> = NonNull::from(&mut self.0).cast();
            Either::True(unsafe{p.as_mut()})
        } else {
            let mut p: NonNull<F> = NonNull::from(&mut self.0).cast();
            Either::False(unsafe{p.as_mut()})
        }
    }    
    
    /*#[inline(always)]
    fn visit<R>(
        &self,
        mut true_fn : impl FnMut(&T) -> R,
        mut false_fn: impl FnMut(&F) -> R,
    ) -> R {
        if B::VALUE {
            let p: NonNull<T> = NonNull::from(&self.0).cast();
            true_fn(unsafe{p.as_ref()})
        } else {
            let p: NonNull<F> = NonNull::from(&self.0).cast();
            false_fn(unsafe{p.as_ref()})
        }
    }*/
}

#[cfg(test)]
mod test{
    use std::fmt::Debug;
    use crate::const_utils::cond_type::CondType;
    use crate::const_utils::cond_type::Either;
    use crate::const_utils::const_bool::*;

    #[derive(Default)]
    struct S<B: ConstBool>{
        v: CondType<B, String, f32>
    }
    
    trait DebugVisitor{
        fn visit(&self, v: &impl Debug);
    }
    
    fn visit_debug<B: ConstBool, T: Debug, F: Debug>(
        s: &CondType<B, T, F>, mut f: impl DebugVisitor
    ) {
        match s.get(){
            Either::True(v) => f.visit(v),
            Either::False(v) => f.visit(v),
        }
    }
    
    #[test]
    fn test_impl_visit(){
        let s: S<ConstTrue> = Default::default();
        struct V;
        impl DebugVisitor for V{
            fn visit(&self, v: &impl Debug) {
                println!("{:?}", v);
            }
        }
        visit_debug(&s.v, V);
    }

    #[test]
    fn test(){
        let s: S<ConstTrue> = S{v: CondType(String::from("bbb"))}/* Default::default()*/;
        
        use Either::*;
        match s.v.clone().get(){
            True(s) => {
                let mut s = s.clone();
                s+="aaa";
                println!("{s}");
            }
            False(f) => {
                println!("{f}");
            }
        }
    }
}
