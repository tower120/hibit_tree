use std::mem;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, ControlFlow};
use crate::bit_queue::{ArrayBitQueue, BitQueue, EmptyBitQueue, PrimitiveBitQueue};
use crate::bit_utils;
use crate::utils::Array;

pub trait BitBlock
    : Eq
    + BitAnd<Output = Self>
    + for<'a> BitAndAssign<&'a Self>
    + BitOr<Output = Self>
    + for<'a> BitOrAssign<&'a Self>
    + Sized + Clone + 'static
{
    /// Size in bits
    const SIZE: usize;
    
    fn zero() -> Self;
    
    #[inline]
    fn is_zero(&self) -> bool{
        self == &Self::zero()
    }
    
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
        let array = self.as_array().as_ref();
        if Self::Array::CAP == 1{
            unsafe{
                bit_utils::get_bit_unchecked(*array.get_unchecked(0), bit_index)
            }
        } else {
            unsafe{
                bit_utils::get_array_bit_unchecked(array, bit_index)
            }
        }
    }
    
    /// Returns [Break] if traverse was interrupted (`f` returns [Break]).
    /// 
    /// [Break]: ControlFlow::Break
    #[inline]
    fn traverse_bits<F>(&self, f: F) -> ControlFlow<()>
    where
        F: FnMut(usize) -> ControlFlow<()>
    {
        let array = self.as_array().as_ref();
        if Self::Array::CAP == 1 {
            let primitive = unsafe{ *array.get_unchecked(0) };
            bit_utils::traverse_one_bits(primitive, f)
        } else {
            bit_utils::traverse_array_one_bits(array, f)
        }
    }    
    
    type BitsIter: BitQueue;
    fn into_bits_iter(self) -> Self::BitsIter;

    type Array: Array<Item = u64>;
    fn as_array(&self) -> &Self::Array;
    fn as_array_mut(&mut self) -> &mut Self::Array;
    
    fn count_ones(&self) -> usize;
}

impl BitBlock for u64{
    const SIZE: usize = 64;

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

    #[inline]
    fn count_ones(&self) -> usize {
        u64::count_ones(*self) as usize
    }
}

#[cfg(feature = "simd")]
#[cfg_attr(docsrs, doc(cfg(feature = "simd")))]
impl BitBlock for wide::u64x2{
    const SIZE: usize = 128;

    #[inline]
    fn zero() -> Self {
        wide::u64x2::ZERO
    }

    type BitsIter = ArrayBitQueue<u64, 2>;

    #[inline]
    fn into_bits_iter(self) -> Self::BitsIter {
        ArrayBitQueue::new(self.to_array())
    }

    type Array = [u64; 2];

    #[inline]
    fn as_array(&self) -> &Self::Array {
        self.as_array_ref()
    }

    #[inline]
    fn as_array_mut(&mut self) -> &mut Self::Array {
        self.as_array_mut()
    }

    #[inline]
    fn count_ones(&self) -> usize {
        let this = self.as_array_ref();
        (
            u64::count_ones(this[0]) + u64::count_ones(this[1])
        ) 
        as usize
    }
}

#[cfg(feature = "simd")]
#[cfg_attr(docsrs, doc(cfg(feature = "simd")))]
impl BitBlock for wide::u64x4{
    const SIZE: usize = 256;

    #[inline]
    fn zero() -> Self {
        wide::u64x4::ZERO
    }

    type BitsIter = ArrayBitQueue<u64, 4>;

    #[inline]
    fn into_bits_iter(self) -> Self::BitsIter {
        ArrayBitQueue::new(self.to_array())
    }

    type Array = [u64; 4];

    #[inline]
    fn as_array(&self) -> &Self::Array {
        self.as_array_ref()
    }

    #[inline]
    fn as_array_mut(&mut self) -> &mut Self::Array {
        self.as_array_mut()
    }
    
    #[inline]
    fn count_ones(&self) -> usize {
        let this = self.as_array_ref();
        (
            u64::count_ones(this[0]) 
            + u64::count_ones(this[1])
            + u64::count_ones(this[2])
            + u64::count_ones(this[3])
        ) 
        as usize
    }
    
}