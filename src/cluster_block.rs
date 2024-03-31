//! Experimental.
//! 
//! Theoretically could be used for super-big 8kb LevelBlock with 65000 elements.
//! 
//! # Principle of work
//! 
//! ```text
//!                                128 bit                   
//!               16 bit │ 16 bit                            
//! Block       └──────── ─────────────────────────────────┘ 
//!                u32   │  NULL     u32   ...   x16           sub-blocks
//!              └───┬──┘ └──────┘ └───┬──┘                  
//!                  │                 │                     
//!                  └────┐            └───────┐             
//!                       ▼                    ▼             
//! Level array       16 values            16 values             buckets
//!              └─────────────────┘  └─────────────────┘    
//! ```
//! Not implemented, but several sub-blocks can point to the same bucket:
//! ```text
//!                                128 bit                    
//!                                                           
//!               16 bit   16 bit                             
//!              0101110 │  0000  │                           
//! Block       └──────── ──────── ────────────────────────┘  
//!         offset:0 ptr │        │                           
//!               u8 u24    NULL    offset:4  ...  x16        
//!              └───┬──┘│└──────┘│└────┬───┘                 
//!                  │                  │                     
//!                  └────┐  ┌──────────┘                     
//!                       ▼  ▼                                
//! Bucket Array      16 values            16 values       ...
//!              └─────────────────┘  └─────────────────┘     
//!                                                           
//!  offset - is in-bucket offset                             
//! ``` 
//! 
//! # Benchmark results
//! 
//! It is suprisingly **NOT** faster than SmallBlock, even when 
//! SmallBlock never switches to Big. When SmallBlock allowed to
//! switch to Big storage - SmallBlock is observably faster.   
//! 

use std::marker::PhantomData;
use std::mem::{MaybeUninit, size_of};
use std::ptr;
use std::slice::{from_raw_parts, from_raw_parts_mut};
use arrayvec::ArrayVec;
use crate::block::{LevelBlock, HiBlock};
use crate::{BitBlock, Primitive, PrimitiveArray};

type SubBlockMask = u16;
const SUB_BLOCK_SIZE: usize = size_of::<SubBlockMask>()*8;

/*type GlobalSubBlock = [u16;16];
static mut global_sub_block_storage: Vec<GlobalSubBlock> = Vec::new();*/

/// SubBlock size MUST be 16!
pub struct ClusterBlock<Mask, SubBlockIndices/*: PrimitiveArray*/, SubBlock/*: PrimitiveArray*/>{
    mask: Mask,
    sub_blocks: SubBlockIndices,
    
    // Move to level
    sub_block_storage: ArrayVec<SubBlock, 4>,
    phantom: PhantomData<SubBlock>
}

impl<Mask, SubBlockIndices, SubBlock> ClusterBlock<Mask, SubBlockIndices, SubBlock>
where
    Mask: BitBlock,
    SubBlockIndices: PrimitiveArray,
    SubBlock: PrimitiveArray
{
    #[inline]
    fn sub_masks(mask: &Mask) -> &[SubBlockMask]{
        let array = mask.as_array().as_ref();
        unsafe{
            from_raw_parts(
                array.as_ptr() as *const SubBlockMask,
                Mask::Array::CAP * (64 / SUB_BLOCK_SIZE) 
            )
        }
    } 
    
    #[inline]
    fn sub_masks_mut(mask: &mut Mask) -> &mut [SubBlockMask]{
        let array = mask.as_array_mut().as_mut();
        unsafe{
            from_raw_parts_mut(
                array.as_mut_ptr() as *mut SubBlockMask,
                Mask::Array::CAP * (64 / SUB_BLOCK_SIZE) 
            )
        }
    } 
}


impl<Mask, SubBlockIndices, SubBlock> LevelBlock for ClusterBlock<Mask, SubBlockIndices, SubBlock>
where
    Mask: BitBlock,
    SubBlockIndices: PrimitiveArray
{
    // TODO: this should accept Level as arg
    #[inline]
    fn empty() -> Self {
        Self{
            mask: Mask::zero(),
            // All indices 0.
            sub_blocks: unsafe{MaybeUninit::zeroed().assume_init()},
            sub_block_storage: Default::default(),
            
            phantom: PhantomData
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        todo!()
    }

    #[inline]
    fn as_u64_mut(&mut self) -> &mut u64 {
        unsafe{
            self.mask.as_array_mut().as_mut().get_unchecked_mut(0)
        }
    }

    #[inline]
    fn restore_empty_u64(&mut self) {
        *self.as_u64_mut() = 0;
    }
}

impl<Mask, SubBlockIndices, SubBlock> HiBlock for ClusterBlock<Mask, SubBlockIndices, SubBlock>
where
    Mask: BitBlock,
    SubBlockIndices: PrimitiveArray,
    SubBlock: PrimitiveArray<Item = u16>
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

    type Item = SubBlock::Item;

    #[inline]
    unsafe fn get_or_zero(&self, index: usize) -> Self::Item {
        let sub_block_index     = index / SUB_BLOCK_SIZE; 
        let sub_block_bit_index = index % SUB_BLOCK_SIZE;
        let mut sub_block_mask  = *Self::sub_masks(&self.mask).get_unchecked(sub_block_index);

        // Return zero if bit not set
        {
            let block_mask: SubBlockMask = 1 << sub_block_bit_index;
            let masked_block = sub_block_mask & block_mask;
            if masked_block.is_zero(){
                return Primitive::ZERO;
            }
        }
        
        let mask = !(SubBlockMask::MAX << sub_block_bit_index);
        sub_block_mask &= mask;
        let sub_block_inner_index = sub_block_mask.count_ones() as usize;
        
        let sub_block_storage = 
                //&global_sub_block_storage.as_ref().unwrap_unchecked()
                &self.sub_block_storage;
        
        let sub_block_index_pointer = self.sub_blocks.as_ref().get_unchecked(sub_block_index).as_usize();
        let sub_block = /*self.*/sub_block_storage.get_unchecked(sub_block_index_pointer);
        *sub_block.as_ref().get_unchecked(sub_block_inner_index)
    }

    unsafe fn get_or_insert(&mut self, index: usize, mut f: impl FnMut() -> Self::Item) -> Self::Item {
        let sub_block_index     = index / SUB_BLOCK_SIZE; 
        let sub_block_bit_index = index % SUB_BLOCK_SIZE;
        let sub_block_mask_mut  = Self::sub_masks_mut(&mut self.mask).get_unchecked_mut(sub_block_index);
        let mut sub_block_mask  = *sub_block_mask_mut;
        
        let sub_block_storage = 
            //&mut global_sub_block_storage.as_mut().unwrap_unchecked()
            &mut self.sub_block_storage;
        
        let sub_block_index_pointer = 
            if sub_block_mask == 0 {
                // allocate new block
                sub_block_storage.push(unsafe{MaybeUninit::zeroed().assume_init()});
                //self.sub_block_storage.push(unsafe{MaybeUninit::zeroed().assume_init()});
                let index = /*self.*/sub_block_storage.len() - 1;
                *self.sub_blocks.as_mut().get_unchecked_mut(sub_block_index) = Primitive::from_usize(index); 
                index
            } else {
                self.sub_blocks.as_ref().get_unchecked(sub_block_index).as_usize()
            };
        
        let sub_block = /*self.*/sub_block_storage.get_unchecked_mut(sub_block_index_pointer);
        
        let sub_block_inner_index = {
            let mask = !(SubBlockMask::MAX << sub_block_bit_index);
            sub_block_mask &= mask;
            sub_block_mask.count_ones() as usize
        };

        // try insert
        {
            let block_mask: SubBlockMask = 1 << sub_block_bit_index;
            let masked_block = sub_block_mask & block_mask;
            if masked_block.is_zero(){
                // 1. update sub_block_mask
                *sub_block_mask_mut |= block_mask;
                
                // 2. insert block_index
                let block_index = f();
                unsafe{
                    let len = sub_block_mask.count_ones() as usize;
                    let p: *mut _ = sub_block.as_mut().as_mut_ptr().add(sub_block_inner_index);
                    // Shift everything over to make space. (Duplicating the
                    // `index`th element into two consecutive places.)
                    ptr::copy(p, p.offset(1), len - sub_block_inner_index);
                    // Write it in, overwriting the first copy of the `index`th
                    // element.
                    ptr::write(p, block_index);
                }                
                return block_index;
            }
        }      
        
        // get
        *sub_block.as_ref().get_unchecked(sub_block_inner_index)            
    }

    unsafe fn remove_unchecked(&mut self, index: usize) {
        todo!()
    }
}