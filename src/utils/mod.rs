mod primitive;
mod array;
mod take;
mod borrowable;
mod function;
mod lending_iterator;

pub use primitive::*;
pub use array::*;
pub use take::*;
pub use borrowable::*;
pub use function::*;
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