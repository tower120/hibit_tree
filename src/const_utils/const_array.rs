use std::mem::ManuallyDrop;
use std::ptr;
use crate::utils::Array;
use crate::const_utils::{ConstUsize, ConstInteger};

// TODO: All Arrays can be ConstArrays.
//       Make ConstArray Array. 
/// [ConstInteger]-sized [Array]. 
pub trait ConstArray: Array {
    type Cap: ConstInteger;
    
    // TODO: split machinery is not used anymore.
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

pub type ArrayOf<T, C: ConstInteger> = C::ArrayOf<T>;

// TODO: remove, use ArrayOf 
/// [ConstArray] with size `C` and type `T` items.
pub type ConstArrayType<T, C: ConstInteger> = C::ArrayOf<T>;

// TODO: remove, use ArrayOf
/// Copyable [ConstArray] with size `C` and type `T` items.
pub type ConstCopyArrayType<T: Copy, C: ConstInteger> = C::CopyArrayOf<T>;

/*/// [ConstInteger] friendly [PrimitiveArray]
pub trait ConstPrimitiveArray
    : ConstArray<DecArray: PrimitiveArray>
    + PrimitiveArray
{}
impl<T: ConstArray<DecArray: PrimitiveArray> + PrimitiveArray> ConstPrimitiveArray for T {}*/