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


// TODO: unite with IntoOwned
pub trait TryTake<T>{
    /// Returned Option variant can
    /// be used for compile-time switch. 
    fn try_take(self) -> Option<T>;
    fn take_or_clone(self) -> T where T: Clone;
}

impl<T> TryTake<T> for T{
    #[inline]
    fn try_take(self) -> Option<T>{
        Some(self)
    }
    
    #[inline]
    fn take_or_clone(self) -> T{
        self
    }
}

impl<T> TryTake<T> for &T{
    #[inline]
    fn try_take(self) -> Option<T>{
        None
    }
    
    #[inline]
    fn take_or_clone(self) -> T where T: Clone{
        self.clone()
    }
}