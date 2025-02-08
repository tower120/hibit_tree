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
    
    unsafe fn assume_init_array(uninit_array: Self::UninitArray) -> Self;
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
        [const { MaybeUninit::uninit() }; N]
    }

    #[inline]
    unsafe fn assume_init_array(uninit_array: Self::UninitArray) -> Self {
        MaybeUninit::array_assume_init(uninit_array)
        
        // Compiler SHOULD optimize away this copy.
        // This should be just transmute, but compiler can't work with "dependently-sized types"
        //mem::transmute_copy(&uninit_array)
        /*mem::transmute::<
            [MaybeUninit::<Self::Item>; N],
            [Self::Item; N]
        >(uninit_array)*/
    }
}