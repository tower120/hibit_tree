use std::mem::MaybeUninit;
use std::ptr::NonNull;
use crate::const_utils::{ConstBool, ConstFalse, ConstTrue};

pub enum CondType<T, F>{
    True(T),
    False(F)
}

#[repr(transparent)]
pub struct ConditionalType<B: ConstBool, T, F>(
    B::CondType<T, F>
);

impl<T, F> From<T> for ConditionalType<ConstTrue, T, F>{
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T, F> From<F> for ConditionalType<ConstFalse, T, F>{
    fn from(value: F) -> Self {
        Self(value)
    }
}

impl<B: ConstBool, T, F> Default for ConditionalType<B, T, F>
where 
    T: Default,
    F: Default,
{
    fn default() -> Self {
        Self::new(||Default::default(), ||Default::default())
    }
}

impl<B: ConstBool, T, F> Clone for ConditionalType<B, T, F>
where 
    T: Clone,
    F: Clone,
{
    #[inline(always)]
    fn clone(&self) -> Self {
        use CondType::*;
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

impl<B: ConstBool, T, F> ConditionalType<B, T, F>{
    #[inline(always)]
    fn new(
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
    fn get(&self) -> CondType<&T, &F>{
        if B::VALUE {
            let p: NonNull<T> = NonNull::from(&self.0).cast();
            CondType::True(unsafe{p.as_ref()})
        } else {
            let p: NonNull<F> = NonNull::from(&self.0).cast();
            CondType::False(unsafe{p.as_ref()})
        }
    }
    
    #[inline(always)]
    fn get_mut(&mut self) -> CondType<&mut T, &mut F>{
        if B::VALUE {
            let mut p: NonNull<T> = NonNull::from(&mut self.0).cast();
            CondType::True(unsafe{p.as_mut()})
        } else {
            let mut p: NonNull<F> = NonNull::from(&mut self.0).cast();
            CondType::False(unsafe{p.as_mut()})
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
    use crate::const_utils::cond_type::ConditionalType;
    use crate::const_utils::cond_type::CondType;
    use crate::const_utils::const_bool::*;

    #[derive(Default)]
    struct S<B: ConstBool>{
        v: ConditionalType<B, String, f32>
    }
    
    #[test]
    fn test(){
        let s: S::<ConstTrue> = Default::default();
        
        use CondType::*;
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
