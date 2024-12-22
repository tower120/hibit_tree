use std::marker::PhantomData;

/// Marker for container's item [Default] requirement.
#[derive(Default, Copy, Clone)] 
pub struct ReqDefault<const B: bool = true>;

pub trait DefaultRequirement: Default {
    const REQUIRED: bool;
}
impl<const B: bool> DefaultRequirement for ReqDefault<B>{
    const REQUIRED: bool = B;
} 

pub trait IsReqDefault{}
impl IsReqDefault for ReqDefault<true>{}


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
impl<T> DefaultInit for DefaultInitFor<T, ReqDefault<false>> {
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
impl<T> MakeDefault<T> for MakeDefaultFor<T, ReqDefault<false>> {
    #[inline]
    fn make_default() -> T {
        unreachable!("T does not implement Default.")
    }
}