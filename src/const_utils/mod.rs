mod const_bool;
pub use const_bool::*;

mod const_int;
pub use const_int::*;

mod const_array;
pub use const_array::*;

mod cond_type;
pub use cond_type::*;

mod const_loop;
pub use const_loop::*;

/// const-friendly max.
macro_rules! max {
    ($l:expr, $r:expr) => {
        if $l > $r {
            $l
        } else {
            $r
        }
    }
}
pub(crate) use max; 