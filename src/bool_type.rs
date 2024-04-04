use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use crate::bit_queue::{BitQueue, EmptyBitQueue};
use crate::BitBlock;

pub trait BoolType: Default + Copy{
    const VALUE: bool;
}

#[derive(Default, Clone, Copy)]
pub struct TrueType;
impl BoolType for TrueType{
    const VALUE: bool = true;
}

#[derive(Default, Clone, Copy)]
pub struct FalseType;
impl BoolType for FalseType{
    const VALUE: bool = false;
}



/*pub trait BypassConst: Default + Copy{
    const VALUE: bool;
    type BypassValue<T>: CondZST<T>;
    //type MaskIter<T: BitBlock>: BitQueue;
}

#[derive(Default, Clone, Copy)]
pub struct BypassTrue;
impl BypassConst for BypassTrue{
    const VALUE: bool = true;
    type BypassValue<T> = CondZSTTrue<T>;
    //type MaskIter<T: BitBlock> = EmptyBitQueue;
}

#[derive(Default, Clone, Copy)]
pub struct BypassFalse;
impl BypassConst for BypassFalse {
    const VALUE: bool = false;
    type BypassValue<T> = CondZSTFalse<T>;
    //type MaskIter<T: BitBlock> = T::BitsIter;
}



pub trait CondEmptyBitQueue<T>: From<T> + AsMut<T>{
    // TODO: empty?
}

#[repr(transparent)]
pub struct CondEmptyBitQueueTrue<T>(EmptyBitQueue, PhantomData<T>);
impl<T> From<T> for CondEmptyBitQueueTrue<T>{
    fn from(_: T) -> Self {
        // TODO: unreachable?
        Self(EmptyBitQueue, PhantomData) 
    }
}



*/




/*pub trait CondZST<T> : From<T> + DerefMut<Target = T>{
    fn into(self) -> T;
}

pub struct CondZSTTrue<T>(PhantomData<T>);
impl<T> From<T> for CondZSTTrue<T>{
    fn from(_: T) -> Self {
        Self(PhantomData)
        //unreachable!()
    }
}
impl<T> Deref for CondZSTTrue<T>{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unreachable!()
    }
}


pub struct CondZSTFalse<T: Sized>(T);
impl<T: Sized> From<T> for CondZSTFalse<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self(value)
    }
}
impl<T: Sized> CondZST<T> for CondZSTFalse<T> {
    #[inline]
    fn into(self) -> T {
        self.0
    }
}*/