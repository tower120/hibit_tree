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
