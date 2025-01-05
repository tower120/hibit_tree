pub(crate) mod map;
pub use map::{Map};

pub(crate) mod intersection;
pub use intersection::Intersection;

pub(crate) mod union;
pub use union::Union;

pub(crate) mod _multi_intersection;
pub mod multi_intersection {
    pub use super::_multi_intersection::{
        Data,
        DataUnchecked,
        DataOrDefault,
        Cursor,
        CursorData
    };
}
pub use _multi_intersection::MultiIntersection;

pub(crate) mod _multi_union;
pub mod multi_union {
    pub use super::_multi_union::{
        Data,
        DataUnchecked,
        DataOrDefault,
        Cursor,
        CursorData,
    };
}
pub use _multi_union::MultiUnion;