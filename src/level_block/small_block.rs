use std::mem::{ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::ops::ControlFlow::Continue;
use std::ptr;
use crate::{BitBlock, MaybeEmpty, MaybeEmptyIntrusive};
use crate::level_block::HiBlock;
use crate::utils::{Array, Primitive};

#[repr(C)]
union BigSmallArray<BlockIndices, SmallBlockIndices, MaskU64Populations>
where
    BlockIndices: Array,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    big: (u8, ManuallyDrop<Box<BlockIndices>>),
    
    /// First element in `MaskU64Populations` is always 0.
    /// 
    /// SmallBlockIndices len = MaskU64Populations.last() + mask.last().count_ones().  
    small: (MaskU64Populations, SmallBlockIndices::UninitArray)
}

impl<BlockIndices, SmallBlockIndices, MaskU64Populations> From<Box<BlockIndices>> for BigSmallArray<BlockIndices, SmallBlockIndices, MaskU64Populations>
where
    BlockIndices: Array,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    #[inline]
    fn from(array: Box<BlockIndices>) -> Self {
        Self{
            big: (1, ManuallyDrop::new(array))
        }
    }
}

impl<BlockIndices, SmallBlockIndices, MaskU64Populations> From<(MaskU64Populations, SmallBlockIndices::UninitArray)> for BigSmallArray<BlockIndices, SmallBlockIndices, MaskU64Populations>
where
    BlockIndices: Array,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    #[inline]
    fn from(small: (MaskU64Populations, SmallBlockIndices::UninitArray)) -> Self {
        debug_assert!(small.0.as_ref().first().unwrap().is_zero());
        Self{ small }
    }
}

impl<BlockIndices, SmallBlockIndices, MaskU64Populations> Clone for BigSmallArray<BlockIndices, SmallBlockIndices, MaskU64Populations>
where
    BlockIndices: Array + Copy,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    #[inline]
    fn clone(&self) -> Self {
        unsafe{
            if self.is_big(){
                Self{big: (1, self.big.1.clone())}
            } else {
                Self{small: self.small}
            }
        }
    }
}

impl<BlockIndices, SmallBlockIndices, MaskU64Populations> BigSmallArray<BlockIndices, SmallBlockIndices, MaskU64Populations>
where
    BlockIndices: Array,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    #[inline]
    fn is_small(&self) -> bool {
        unsafe{ self.big.0 == 0 }
    }
    #[inline]
    fn is_big(&self) -> bool {
        !self.is_small()
    }
}

impl<BlockIndices, SmallBlockIndices, MaskU64Populations> Drop for BigSmallArray<BlockIndices, SmallBlockIndices, MaskU64Populations>
where
    BlockIndices: Array,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy
{
    #[inline]
    fn drop(&mut self) {
        if self.is_big(){
            unsafe{ ManuallyDrop::drop(&mut self.big.1); }
        }
    }
}

/// TODO: Copy description from hi_sparse_bitset
#[derive(Clone)]
pub struct SmallBlock<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices>
where
    BlockIndices: Array + Copy,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    mask: Mask,
    big_small: BigSmallArray<BlockIndices, SmallBlockIndices, MaskU64Populations>
}

impl<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices> SmallBlock<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices>
where
    Mask: BitBlock,
    BlockIndices: Array + Copy,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    // This can be the only small_array_index() -> Option<usize> function.
    // However, unwrap_unchecked() provides no guarantees, and I don't want to risk
    // the CHECK_FOR_EMPTY block not being elided.
    /// number of 1 bits in mask before `index` bit.
    ///
    /// # Safety
    /// 
    /// * small must be active.
    /// * `index` must be set.
    #[inline]
    unsafe fn small_array_index_impl<const CHECK_FOR_EMPTY: bool>(&self, index: usize) 
        -> Option<usize> 
    {
        let u64_index =
            if Mask::size() == 64 {
                0
            } else {
                index / 64
            };
        let bit_index =
            if Mask::size() == 64 {
                index
            } else {
                index % 64
            };
        let mut block = *self.mask.as_array().as_ref().get_unchecked(u64_index);
        
        if CHECK_FOR_EMPTY {
            let block_mask: u64 = 1 << bit_index;
            let masked_block = block & block_mask;
            if masked_block.is_zero(){
                return None;
            }
        }        
        
        let mask = !(u64::MAX << bit_index);
        block &= mask;
        
        let offset = if MaskU64Populations::CAP == 1 {
            // first always zero
            0
        } else {
            let mask_u64_populations = &self.big_small.small.0;
            *mask_u64_populations.as_ref().get_unchecked(u64_index)
        };
        Some(offset as usize + block.count_ones() as usize)
    }
    
    #[inline]
    unsafe fn small_array_index_unchecked(&self, index: usize) -> usize {
        self.small_array_index_impl::<false>(index).unwrap_unchecked()
    }
    
    #[inline]
    unsafe fn try_small_array_index(&self, index: usize) -> Option<usize> {
        self.small_array_index_impl::<true>(index)
    }
    
    #[inline]
    unsafe fn small_array_len(&self) -> usize {
        // TODO: Consider storing len directly
        
        let population_at_last_mask_block_start = if MaskU64Populations::CAP == 1 {
            0
        } else {
            let mask_u64_populations = &self.big_small.small.0;
            *mask_u64_populations.as_ref().last().unwrap_unchecked() as usize
        };
        
        let last_mask_block_population =
            self.mask.as_array().as_ref().last().unwrap_unchecked().count_ones() as usize;
        
        population_at_last_mask_block_start + last_mask_block_population
    }    
    
    /// # Safety
    /// 
    /// * `index` must not be set.
    /// * `mask`'s corresponding bit must be 0.
    #[inline]
    unsafe fn insert_unchecked(&mut self, index: usize, value: BlockIndices::Item){
        if self.big_small.is_big(){
            let array = self.big_small.big.1.deref_mut();
            *array.deref_mut().as_mut().get_unchecked_mut(index) = value;
        } else {
            let len = self.small_array_len();
            if len == SmallBlockIndices::CAP {
                // TODO: as non-inline function?
                // move to Big
                let mut big: Box<BlockIndices> = Box::new(unsafe{MaybeUninit::zeroed().assume_init()});
                let big_array = big.deref_mut().as_mut(); 
                let mut i = 0;
                 
                let array = &mut self.big_small.small.1;
                self.mask.traverse_bits(|index|{
                    let value = array.as_ref().get_unchecked(i).assume_init_read();
                    i += 1;
                    
                    *big_array.get_unchecked_mut(index) = value;
                    Continue(()) 
                });
                *big_array.get_unchecked_mut(index) = value;
                self.big_small = BigSmallArray::from(big);
            } else {
                let inner_index = self.small_array_index_unchecked(index);
                let (mask_u64_populations, array) = &mut self.big_small.small;
                unsafe{
                    let p: *mut _ = array.as_mut().as_mut_ptr().add(inner_index);
                    // Shift everything over to make space. (Duplicating the
                    // `index`th element into two consecutive places.)
                    ptr::copy(p, p.offset(1), len - inner_index);
                    // Write it in, overwriting the first copy of the `index`th
                    // element.
                    ptr::write(p, MaybeUninit::new(value));
                }
                
                for i in (index/64)+1..Mask::size()/64 {
                    *mask_u64_populations.as_mut().get_unchecked_mut(i) += 1;
                }
            }
        }
        self.mask.set_bit::<true>(index);
    }      
}


impl<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices> MaybeEmpty for SmallBlock<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices>
where
    Mask: BitBlock,
    BlockIndices: Array + Copy,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    #[inline]
    fn empty() -> Self {
        Self{
            mask: Mask::zero(),
            big_small:
            BigSmallArray::from(
                (
                /*mask_u64_populations:*/ unsafe{MaybeUninit::zeroed().assume_init()},
                /*array:*/ SmallBlockIndices::uninit_array()
                )
            )
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.mask.is_zero()
    }
}

impl<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices> MaybeEmptyIntrusive for SmallBlock<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices>
where
    Mask: BitBlock,
    BlockIndices: Array + Copy,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    #[inline]
    fn as_u64_mut(&mut self) -> &mut u64 {
        unsafe{
            self.mask.as_array_mut().as_mut().get_unchecked_mut(0)
        }
    }

    #[inline]
    fn restore_empty(&mut self) {
        *self.as_u64_mut() = 0;
    }
}


impl<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices> HiBlock for SmallBlock<Mask, MaskU64Populations, BlockIndices, SmallBlockIndices>
where
    Mask: BitBlock,
    BlockIndices: Array<Item: Primitive> + Copy,
    SmallBlockIndices: Array<Item=BlockIndices::Item, UninitArray: Copy>,
    MaskU64Populations: Array<Item=u8> + Copy,
{
    type Mask = Mask;

    #[inline]
    fn mask(&self) -> &Self::Mask {
        &self.mask
    }

    #[inline]
    unsafe fn mask_mut(&mut self) -> &mut Self::Mask {
        &mut self.mask
    }

    type Item = BlockIndices::Item;
    
    #[inline]
    unsafe fn get_or_zero(&self, index: usize) -> Self::Item {
        if self.big_small.is_big(){
            let array = self.big_small.big.1.deref();
            *array.deref().as_ref().get_unchecked(index)
        } else {
            if let Some(small_array_index) = self.try_small_array_index(index) {
                let (_, array) = &self.big_small.small;
                array.as_ref().get_unchecked(small_array_index).assume_init_read()
            } else {
                Primitive::ZERO
            }
        }        
    }
    
    #[inline]
    unsafe fn get_or_insert(&mut self, index: usize, mut f: impl FnMut() -> Self::Item) -> Self::Item {
        let mut block_index = self.get_or_zero(index);
        if block_index.is_zero(){
            block_index = f();
            self.insert_unchecked(index, block_index);
        }
        block_index
    }

    #[inline]
    unsafe fn remove_unchecked(&mut self, index: usize) {
        let prev = self.mask.set_bit::<false>(index);
        debug_assert!(prev);
        
        if self.big_small.is_big(){
            let array = self.big_small.big.1.deref_mut();
            // TODO: go back to small at small/2 size? 
            *array.deref_mut().as_mut().get_unchecked_mut(index) = Primitive::ZERO;
        } else {
            let inner_index = self.small_array_index_unchecked(index);
            let len = self.small_array_len();
            
            let (mask_u64_populations, array) = &mut self.big_small.small;
            unsafe{
                let p: *mut _ = array.as_mut().as_mut_ptr().add(inner_index);
                ptr::copy(p.offset(1), p, len - inner_index);
            }
            
            for i in (index/64)+1..Mask::size()/64 {
                *mask_u64_populations.as_mut().get_unchecked_mut(i) -= 1;
            }            
        }
    }

    #[inline]
    unsafe fn set_unchecked(&mut self, index: usize, item: Self::Item) {
        if self.big_small.is_big(){
            let array = self.big_small.big.1.deref_mut();
            *array.deref_mut().as_mut().get_unchecked_mut(index) = item;
        } else {
            let inner_index = self.small_array_index_unchecked(index);
            let array = self.big_small.small.1.as_mut();
            array.get_unchecked_mut(inner_index).write(item);
        }
    }
}
