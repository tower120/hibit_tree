pub(crate) mod map;
pub use map::{Map, MapFunction};


pub(crate) mod multi_fold;
pub use multi_fold::MultiFold;


pub(crate) mod intersection;
pub use intersection::Intersection;


pub(crate) mod union;
pub use union::Union;


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