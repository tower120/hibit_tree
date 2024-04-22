use std::marker::PhantomData;
use std::ptr::NonNull;
use std::slice;
use crate::bit_block::{EmptyBitBlock, IEmptyBitBlock};
use crate::bool_type::{BoolType, FalseType, TrueType};
use crate::level_block::{BypassBlock, HiBlock, LevelBlock};
use crate::primitive::Primitive;

pub trait ILevel: Default {
    type Block: LevelBlock;
    
    fn blocks(&self) -> &[Self::Block];
    fn blocks_mut(&mut self) -> &mut [Self::Block];
    
    fn insert_empty_block(&mut self) -> usize;
    
    /// # Safety
    ///
    /// block_index and level_block emptiness are not checked.
    unsafe fn remove_empty_block_unchecked(&mut self, block_index: usize);
}

#[derive(Clone)]
pub struct SingleBlockLevel<Block: LevelBlock>{
    block: Block
}

impl<Block: LevelBlock> ILevel for SingleBlockLevel<Block>{
    type Block = Block;

    fn blocks(&self) -> &[Self::Block] {
        unsafe{ slice::from_raw_parts(&self.block, 1) }
    }

    fn blocks_mut(&mut self) -> &mut [Self::Block] {
        unsafe{ slice::from_raw_parts_mut(&mut self.block, 1) }
    }

    fn insert_empty_block(&mut self) -> usize {
        unreachable!()
    }

    unsafe fn remove_empty_block_unchecked(&mut self, block_index: usize) {
        unreachable!()
    }
}

impl<Block: LevelBlock> Default for SingleBlockLevel<Block> {
    #[inline]
    fn default() -> Self {
        Self{ block: Block::empty() }
    }
}

#[derive(Clone)]
pub struct Level<Block: LevelBlock>{
    blocks: Vec<Block>,
    
    /// Single linked list of empty level_block indices.
    /// Mask of empty level_block used as a "next free level_block".
    /// u64::MAX - terminator.
    root_empty_block: u64,
}

impl<Block: LevelBlock> Default for Level<Block> {
    #[inline]
    fn default() -> Self {
        Self{
            //Always have empty level_block at index 0.
            blocks:vec![Block::empty()],
            root_empty_block: u64::MAX,
        }
    }
}

impl<Block: LevelBlock> Level<Block> {
    /// Next empty level_block link
    /// 
    /// Block's mask used as index to next empty level_block
    #[inline]
    unsafe fn next_empty_block_index(block: &mut Block) -> &mut u64 {
        block.as_u64_mut()
    }
    
    #[inline]
    fn pop_empty_block(&mut self) -> Option<usize> {
        if self.root_empty_block == u64::MAX {
            return None;
        }
            
        let index = self.root_empty_block as usize;
        unsafe{
            let empty_block = self.blocks.get_unchecked_mut(index);
            let next_empty_block_index = Self::next_empty_block_index(empty_block); 
            
            // update list root 
            self.root_empty_block = *next_empty_block_index;
            
            // restore original level_block zero state
            empty_block.restore_empty_u64();
        }
        Some(index)
    }

    /// # Safety
    /// 
    /// level_block must be empty and not in use!
    #[inline]
    unsafe fn push_empty_block(&mut self, block_index: usize){
        let empty_block = self.blocks.get_unchecked_mut(block_index);
        let next_empty_block_index = Self::next_empty_block_index(empty_block);
        *next_empty_block_index = self.root_empty_block;
        
        self.root_empty_block = block_index as u64;
    }
}

impl<Block: LevelBlock> ILevel for Level<Block> {
    type Block = Block;

    #[inline]
    fn blocks(&self) -> &[Self::Block] {
        self.blocks.as_slice()
    }

    #[inline]
    fn blocks_mut(&mut self) -> &mut [Self::Block] {
        self.blocks.as_mut_slice()
    }

    #[inline]
    fn insert_empty_block(&mut self) -> usize {
        if let Some(index) = self.pop_empty_block(){
            index
        } else {
            let index = self.blocks.len();
            self.blocks.push(Block::empty());
            index
        }
    }

    #[inline]
    unsafe fn remove_empty_block_unchecked(&mut self, block_index: usize) {
        self.push_empty_block(block_index);
    }
}

// TODO: remove - all not used now
// TODO: there should be #derive(EmptyBitBlock) ?

pub(crate) const fn bypass_level<EmptyMask>() -> BypassLevel<EmptyMask>{
    BypassLevel(PhantomData)
}

pub(crate) const fn bypass_level_ref<EmptyMask>() -> &'static BypassLevel<EmptyMask>{
    let ptr: NonNull<BypassLevel<EmptyMask>> = NonNull::dangling();
    unsafe{
        ptr.as_ref()
    }
}


pub struct BypassLevel<EmptyMask/* : IEmptyBitBlock  */= EmptyBitBlock>(PhantomData<EmptyMask>);
impl<EmptyMask/* : IEmptyBitBlock */> Default for BypassLevel<EmptyMask>{
    #[inline]
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<EmptyMask/* : IEmptyBitBlock */> ILevel for BypassLevel<EmptyMask> {
    type Block = BypassBlock<EmptyMask>;

    fn blocks(&self) -> &[Self::Block] {
        unreachable!()
    }

    fn blocks_mut(&mut self) -> &mut [Self::Block] {
        unreachable!()
    }

    fn insert_empty_block(&mut self) -> usize {
        unreachable!()
    }

    unsafe fn remove_empty_block_unchecked(&mut self, block_index: usize) {
        unreachable!()
    }
}
