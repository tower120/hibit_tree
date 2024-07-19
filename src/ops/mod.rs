pub(crate) mod intersection;

//pub mod union2;
pub(crate) mod union3;

//mod multi_union2;
pub(crate) mod multi_union3;

//mod multi_intersection;
pub(crate) mod multi_intersection2;
//pub mod multi_intersection3;

pub use intersection::{Intersection};
pub use union3::{Union, UnionResolve};

pub use multi_intersection2::{MultiIntersection, MultiIntersectionResolveIter};
pub use multi_union3::{MultiUnion, MultiUnionResolveIter};