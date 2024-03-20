//! Ala "universal reference"/"forwarding reference" in C++
//! 
//! Ideally, we should just have blanket implementation for everything.
//! And somehow opt-out for val/ref.

pub trait RefOrVal{
    /// without ref
    type Type;
    fn as_ref(&self) -> &Self::Type;
}

pub trait MutOrVal: RefOrVal {
    fn as_mut(&mut self) -> &mut Self::Type;
}

/// Implements [RefOrVal] and [MutOrVal] for your type.
/// 
/// `ref` - implementation for & and &mut only (no by-value).
#[macro_export]
macro_rules! ref_or_val {
   
    (impl for $t:ty) => {
        ref_or_val!(impl {} for $t where );
    };
    
    (impl <$($generics:tt),*> for $t:ty) => {
        ref_or_val!(impl {$($generics),*} for $t where );
    };
    
    (impl <$($generics:tt),*> for $t:ty where $($where_bounds:tt)*) => {
        ref_or_val!(impl {$($generics),*} for $t where $($where_bounds)*);
    };
    
    (impl {$($generics:tt),*} for $t:ty where $($where_bounds:tt)*) => {
        impl<$($generics),*> $crate::RefOrVal for $t 
        where
            $($where_bounds)*
        {
            type Type = $t;
            
            #[inline]
            fn as_ref(&self) -> &Self::Type{
                self
            }
        }
        
        impl<$($generics),*> $crate::MutOrVal for $t 
        where
            $($where_bounds)*
        {
            #[inline]
            fn as_mut(&mut self) -> &mut Self::Type{
                self
            }
        }
        
        ref_or_val!(impl {$($generics),*} for ref $t where $($where_bounds)*);
    };
    
    (impl for ref $t:ty) => {
        ref_or_val!(impl {} for ref $t where );
    };

    (impl <$($generics:tt),*> for ref $t:ty) => {
        ref_or_val!(impl {$($generics),*} for ref $t where );
    };
    
    (impl <$($generics:tt),*> for ref $t:ty where $($where_bounds:tt)*) => {
        ref_or_val!(impl {$($generics),*} for ref $t where $($where_bounds)*);
    };
    
    (impl {$($generics:tt),*} for ref $t:ty where $($where_bounds:tt)*) => {
        impl<$($generics),*> $crate::RefOrVal for &$t 
        where
            $($where_bounds)*
        {
            type Type = $t;
            
            #[inline]
            fn as_ref(&self) -> &Self::Type{
                *self
            }
        }
        
        impl<$($generics),*> $crate::RefOrVal for &mut $t 
        where
            $($where_bounds)*
        {
            type Type = $t;
            
            #[inline]
            fn as_ref(&self) -> &Self::Type{
                *self
            }
        }
        
        impl<$($generics),*> $crate::MutOrVal for &mut $t 
        where
            $($where_bounds)*
        {
            #[inline]
            fn as_mut(&mut self) -> &mut Self::Type{
                *self
            }
        }     
    };
}

#[cfg(test)]
mod test{
    use super::*;

    #[test]
    fn smoke_test(){
        fn test(_: impl RefOrVal){}
        
        struct S1<T>(T);
        ref_or_val!(impl <T> for ref S1<T>);
        
        let s: S1<i32> = S1(0);
        test(&s);
        
        struct S2;
        ref_or_val!(impl for S2);
        let s = S2;
        test(s);
    }
}