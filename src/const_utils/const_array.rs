use std::mem::ManuallyDrop;
use std::ptr;
use crate::{Array, Primitive};
use crate::const_utils::{ConstUsize, ConstInteger};

/// [ConstInteger]-sized [Array]. 
pub trait ConstArray: Array {
    type Cap: ConstInteger;
    
    /// Self array decremented in size.
    type DecArray: ConstArray<Item=Self::Item, Cap=<Self::Cap as ConstInteger>::Dec>;  
    fn split_last(self) -> (Self::DecArray, Self::Item);
}

impl<T, const N: usize> ConstArray for [T; N]
where
    ConstUsize<N>: ConstInteger
{
    type Cap = ConstUsize<N>;
    
    /// Array with N-1 size/cap.
    type DecArray = ConstArrayType<Self::Item, <Self::Cap as ConstInteger>::Dec>;

    #[inline]
    fn split_last(self) -> (Self::DecArray, Self::Item) {
        let this = ManuallyDrop::new(self);
        let left = unsafe{
            Array::from_fn(|i| { 
                ptr::read(this.as_ref().get_unchecked(i))
            })
        };
        let right = unsafe{
            ptr::read(this.as_ref().last().unwrap())
        };
        (left, right)
    }
}

pub type ConstArrayType<T, C: ConstInteger> = C::SelfSizeArray<T>;

pub type ConstCopyArrayType<T: Copy, C: ConstInteger> = C::SelfSizeCopyArray<T>;

/*/// [ConstInteger] friendly [PrimitiveArray]
pub trait ConstPrimitiveArray
    : ConstArray<DecArray: PrimitiveArray>
    + PrimitiveArray
{}
impl<T: ConstArray<DecArray: PrimitiveArray> + PrimitiveArray> ConstPrimitiveArray for T {}*/