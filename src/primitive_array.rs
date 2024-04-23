// TODO: rename mod to array?

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
    
    type UninitArray: UninitArray<UninitItem = Self::Item>;
    fn uninit_array() -> Self::UninitArray{
        Self::UninitArray::uninit()
    }
}

pub trait UninitArray: Array<Item = MaybeUninit<Self::UninitItem>>{
    type UninitItem;    
    fn uninit() -> Self;
}

impl<T, const N: usize> Array for [T; N]{
    type Item = T;
    const CAP: usize = N;
         
    type UninitArray = [MaybeUninit<Self::Item>; N];
    
    fn from_fn<F>(f: F) -> Self 
    where 
        F: FnMut(usize) -> T 
    {
        array::from_fn(f)
    }
}

impl<T, const N: usize> UninitArray for [MaybeUninit<T>; N]{
    type UninitItem = T;

    #[inline]
    fn uninit() -> Self {
        // From Rust MaybeUninit::uninit_array() :
        // SAFETY: An uninitialized `[MaybeUninit<_>; LEN]` is valid.
        unsafe { MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init() }
    }
}

pub trait PrimitiveArray: Array<Item: Primitive, UninitArray: Copy> + Copy {}
impl<T: Array<Item: Primitive, UninitArray: Copy> + Copy> PrimitiveArray for T {}

pub trait UninitPrimitiveArray: UninitArray<UninitItem: Primitive> + Copy{}
impl <T: UninitPrimitiveArray + Copy> UninitPrimitiveArray for T{} 
