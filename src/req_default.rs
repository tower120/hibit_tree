use std::marker::PhantomData;
use crate::const_utils::{ConstBool, ConstFalse, ConstTrue};

/// Marker for container's item [Default] requirement.
#[derive(Default, Copy, Clone)] 
pub struct ReqDefault<B: ConstBool = ConstTrue>(B);

pub trait DefaultRequirement: Default + Copy + Clone {
    type Required: ConstBool;
}
impl<B: ConstBool> DefaultRequirement for ReqDefault<B>{
    type Required = B;
} 

pub trait IsReqDefault{}
impl IsReqDefault for ReqDefault<ConstTrue>{}


pub(crate) trait MakeDefault<T> {
    fn make_default() -> T;
}
pub(crate) struct MakeDefaultFor<T, R>(PhantomData<(T, R)>);
impl<T: Default> MakeDefault<T> for MakeDefaultFor<T, ReqDefault> {
    #[inline]
    fn make_default() -> T{
        T::default()
    }
}
impl<T> MakeDefault<T> for MakeDefaultFor<T, ReqDefault<ConstFalse>> {
    #[inline]
    fn make_default() -> T {
        unreachable!("T does not implement Default.")
    }
}