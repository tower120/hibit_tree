use std::borrow::Borrow;

pub(crate) mod primitive;
pub(crate) mod array;

pub use primitive::*;
pub use array::*;

pub trait Borrowable: Borrow<Self::Borrowed>{
    type Borrowed;
}

// Not necessary?
/*pub trait BorrowableMut: Borrowable + BorrowMut<Self::Borrowed>{}*/

// TODO: remove
/// convert T to value. noop for value, clone - for reference.
///
/// # Note
///
/// Surprisingly, there is no such thing in `std`. The closest one
/// is `Cow` enum, with runtime overhead.
pub trait IntoOwned<T>{
    fn into_owned(self) -> T;
}

impl<T> IntoOwned<T> for T{
    #[inline]
    fn into_owned(self) -> T{
        self
    }
}

impl<T: Clone> IntoOwned<T> for &T{
    #[inline]
    fn into_owned(self) -> T{
        self.clone()
    }
}


/// Means of converting T/&T to value.
/// 
/// Allows to store/pass reference or pointer generically.
/// In conjunction with [Borrow] can be used as C++'s &&T (forwarding reference).
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