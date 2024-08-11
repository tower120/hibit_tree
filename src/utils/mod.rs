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
