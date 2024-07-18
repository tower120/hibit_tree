mod intersection;

//pub mod union2;
mod union3;

//mod multi_union2;
mod multi_union3;

//mod multi_intersection;
mod multi_intersection2;
//pub mod multi_intersection3;

pub use intersection::{Intersection, intersection};
pub use union3::{Union, UnionResolve, union};

pub use multi_intersection2::{MultiIntersection, MultiIntersectionResolveIter, multi_intersection};
pub use multi_union3::{MultiUnion, MultiUnionResolveIter, multi_union};