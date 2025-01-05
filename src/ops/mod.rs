/*pub(crate) mod map;
pub use map::{Map, MapFunction};*/

// pub(crate) mod iterate_with_default;

pub(crate) mod map2_1;
pub use map2_1::{Map};

/*pub(crate) mod multi_map_fold;
pub use multi_map_fold::MultiMapFold;*/

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
//mod multi_map;

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