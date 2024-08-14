pub(crate) mod map;
pub(crate) mod intersection;
/*pub(crate) mod union;
pub(crate) mod multi_union3;
pub(crate) mod multi_intersection2;*/
//pub(crate) mod multi_intersection3;
pub(crate) mod multi_intersection4;
pub(crate) mod multi_fold;
// TODO: hide. dev only
//pub mod multi_intersection_fold;

pub use map::{Map, MapFunction};
pub use multi_fold::MultiFold;
pub use intersection::Intersection;
/*pub use union::Union;
pub use multi_intersection2::{MultiIntersection, MultiIntersectionResolveIter};
pub use multi_union3::{MultiUnion, MultiUnionResolveIter};*/

pub use multi_intersection4::{MultiIntersection, StateResolveIter};