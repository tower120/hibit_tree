use std::marker::PhantomData;

/// Marker for container's item [Default] requirement. 
pub struct ReqDefault<const B: bool = true>;

pub trait DefaultRequirement{
    const REQUIRED: bool;
}

impl<const B: bool> DefaultRequirement for ReqDefault<B>{
    const REQUIRED: bool = B;
} 

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