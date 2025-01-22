mod primitive;
pub use primitive::*;

mod array;
pub use array::*;

mod take;
pub use take::*;

mod borrowable;
pub use borrowable::*;

pub mod function;

mod lending_iterator;
pub use lending_iterator::*;

/// Trait for &.
pub trait Ref {
    type Type;
}

impl<T> Ref for &T {
    type Type = T;
}

/// Trait for &'a
pub trait RefLt<'a>: Ref {
    fn get_ref(self) -> &'a Self::Type;
}

impl<'a, T> RefLt<'a> for &'a T {
    fn get_ref(self) -> &'a Self::Type{
        self
    }
}