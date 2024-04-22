// TODO: rename mod to array.rs?

use std::{array, mem};
use std::mem::{ManuallyDrop, MaybeUninit};
use crate::const_int::{ConstInt, ConstInteger};
use crate::primitive::Primitive;

/// [Item; CAP]
pub trait Array
    : AsRef<[Self::Item]> 
    + AsMut<[Self::Item]>
{
    type Item;
    const CAP: usize;
    
    fn from_fn<F>(f: F) -> Self
    where
        F: FnMut(usize) -> Self::Item;
}

impl<T, const N: usize> Array for [T; N]{
    type Item = T;
    const CAP: usize = N;

    fn from_fn<F>(f: F) -> Self 
    where 
        F: FnMut(usize) -> T 
    {
        array::from_fn(f)
    }
}

pub trait PrimitiveArray: Array<Item: Primitive> + Copy {
    // TODO: move to Array
    type UninitArray: UninitPrimitiveArray<UninitItem = Self::Item>;
    
    #[deprecated]
    #[inline]
    fn from_array<const N: usize>(array: [Self::Item; N]) -> Self {
        if Self::CAP != N{
            panic!("Wrong array len!");
        }
        
        unsafe{
            // Ala transmute_unchecked.
            // transmute is safe since OneBitsIter<P> transparent to P.
            // Should be just mem::transmute(array).
            mem::transmute_copy(&ManuallyDrop::new(array))
        }
    }    
}

impl<T, const N: usize> PrimitiveArray for [T; N]
where
    T: Primitive
{
    type UninitArray = [MaybeUninit<Self::Item>; N];
}

pub trait UninitPrimitiveArray
    : AsRef<[MaybeUninit<Self::UninitItem>]> 
    + AsMut<[MaybeUninit<Self::UninitItem>]> 
    + Copy
{
    //type Item? 
    type UninitItem: Primitive;
    const CAP: usize;
    
    fn uninit_array() -> Self;
}
impl<T, const N: usize> UninitPrimitiveArray for [MaybeUninit<T>; N]
where
    T: Primitive
{
    type UninitItem = T;
    const CAP: usize = N;
    
    #[inline]
    fn uninit_array() -> Self{
        // From Rust MaybeUninit::uninit_array() :
        // SAFETY: An uninitialized `[MaybeUninit<_>; LEN]` is valid.
        unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() }        
    }
}