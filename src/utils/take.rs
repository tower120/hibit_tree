/// Means of converting T/&T to value.
/// 
/// Allows to store/pass reference or value generically.
/// In conjunction with [Borrow] can be used as something like C++'s &&T 
/// (forwarding reference), that can be taken by ref or value.
/// 
/// Everything zero overhead.
pub trait Take<T>{
    /// Takes self as T.
    /// 
    /// # Panics
    /// 
    /// Panics at compile-time if Self is &T.
    fn take(self) -> T;
    
    /// Returned Option variant can
    /// be used for compile-time switch. 
    fn try_take(self) -> Option<T>;
    
    /// Return self as is for T, clone for &T.
    fn take_or_clone(self) -> T where T: Clone;
}

impl<T> Take<T> for T{
    #[inline]
    fn take(self) -> T{
        self
    }
    
    #[inline]
    fn try_take(self) -> Option<T>{
        Some(self)
    }
    
    #[inline]
    fn take_or_clone(self) -> T{
        self
    }
}

impl<T> Take<T> for &T{
    #[inline]
    fn take(self) -> T{
        const{ panic!("Trying to take &T by value.") }
    }
    
    #[inline]
    fn try_take(self) -> Option<T>{
        None
    }
    
    #[inline]
    fn take_or_clone(self) -> T where T: Clone{
        self.clone()
    }
}