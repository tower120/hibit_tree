use std::mem;
use crate::bit_queue::{ArrayBitQueue, BitQueue, PrimitiveBitQueue};
use crate::bit_utils;
use crate::primitive_array::PrimitiveArray;

pub trait BitBlock: Sized + Clone {
    /// 2^N bits
    const SIZE_POT_EXPONENT: usize;
    
    /// Size in bits
    #[inline]
    /*const*/ fn size() -> usize {
        1 << Self::SIZE_POT_EXPONENT
    }
    
    fn zero() -> Self;
    
    /// Returns previous bit
    /// 
    /// `bit_index` is guaranteed to be valid
    #[inline]
    fn set_bit<const BIT: bool>(&mut self, bit_index: usize) -> bool {
        let array = self.as_array_mut().as_mut();
        if Self::Array::CAP == 1{
            unsafe{
                bit_utils::set_bit_unchecked::<BIT, _>(array.get_unchecked_mut(0), bit_index)
            }
        } else {
            unsafe{
                bit_utils::set_array_bit_unchecked::<BIT, _>(array, bit_index)
            }
        }
    }

    /// `bit_index` is guaranteed to be valid
    #[inline]
    fn get_bit(&self, bit_index: usize) -> bool{
        todo!()
    }    
    
    type BitsIter: BitQueue;
    #[inline]
    fn into_bits_iter(self) -> Self::BitsIter;

    type Array: PrimitiveArray<Item = u64>;
    fn as_array(&self) -> &Self::Array;
    fn as_array_mut(&mut self) -> &mut Self::Array;
}

impl BitBlock for u64{
    const SIZE_POT_EXPONENT: usize = 6;

    fn zero() -> Self { 0 }

    type BitsIter = PrimitiveBitQueue<u64>;
    #[inline]
    fn into_bits_iter(self) -> Self::BitsIter {
        PrimitiveBitQueue::new(self)
    }


    type Array = [u64; 1];
    #[inline]
    fn as_array(&self) -> &Self::Array {
        unsafe {
            mem::transmute::<&u64, &[u64; 1]>(self)
        }        
    }
    #[inline]
    fn as_array_mut(&mut self) -> &mut Self::Array {
        unsafe {
            mem::transmute::<&mut u64, &mut [u64; 1]>(self)
        }        
    }
}