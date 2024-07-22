pub(crate) mod intersection;
pub(crate) mod union;
pub(crate) mod multi_union3;
pub(crate) mod multi_intersection2;

pub use intersection::{Intersection};
pub use union::Union;

pub use multi_intersection2::{MultiIntersection, MultiIntersectionResolveIter};
pub use multi_union3::{MultiUnion, MultiUnionResolveIter};