use std::array;
use std::mem::MaybeUninit;

/// [Item; CAP]
pub trait Array
    : AsRef<[Self::Item]> 
    + AsMut<[Self::Item]>
    + Sized
{
    type Item;
    const CAP: usize;
    
    fn from_fn<F>(f: F) -> Self
    where
        F: FnMut(usize) -> Self::Item;
    
    /// Array of MaybeUninit items
    type UninitArray: Array<Item = MaybeUninit<Self::Item>>;
    fn uninit_array() -> Self::UninitArray;
}

impl<T, const N: usize> Array for [T; N]{
    type Item = T;
    const CAP: usize = N;
         
    type UninitArray = [MaybeUninit<Self::Item>; N];
    
    #[inline]
    fn from_fn<F>(f: F) -> Self 
    where 
        F: FnMut(usize) -> T 
    {
        array::from_fn(f)
    }

    #[inline]
    fn uninit_array() -> Self::UninitArray {
        unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() }
    }
}