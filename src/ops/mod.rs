pub(crate) mod map;
pub(crate) mod intersection;
//pub(crate) mod union;
pub(crate) mod multi_fold;

pub use map::{Map, MapFunction};
pub use multi_fold::MultiFold;
pub use intersection::Intersection;
//pub use union::Union;

pub(crate) mod _multi_intersection4;
pub mod multi_intersection4{
    pub use super::_multi_intersection4::{
        Data,
        DataUnchecked,
        State,
        StateData
    };
}
pub use _multi_intersection4::MultiIntersection;

pub(crate) mod _multi_union4;
pub mod multi_union4{
    pub use super::_multi_union4::{
        Data,
        DataUnchecked,
        State,
        StateData,
    };
}
pub use _multi_union4::MultiUnion;