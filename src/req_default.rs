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


#[deprecated = "use MakeDefault instead"]
pub(crate) trait DefaultInit {
    unsafe fn init_default(value: *mut u8);
}
pub(crate) struct DefaultInitFor<T, R>(PhantomData<(T, R)>);
impl<T: Default> DefaultInit for DefaultInitFor<T, ReqDefault> {
    #[inline]
    unsafe fn init_default(value: *mut u8) {
        value.cast::<T>().write(T::default())
    }
}
impl<T> DefaultInit for DefaultInitFor<T, ReqDefault<ConstFalse>> {
    #[inline]
    unsafe fn init_default(_: *mut u8) {
        // nothing
    }
}



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