use crate::level::ILevel;
use crate::MaybeEmptyIntrusive;

/// Level that uses intrusive list for an empty blocks list.
#[derive(Clone)]
pub struct IntrusiveListLevel<Block: MaybeEmptyIntrusive>{
    blocks: Vec<Block>,
    
    /// Single linked list of empty level_block indices.
    /// Mask of empty level_block used as a "next free level_block".
    /// u64::MAX - terminator.
    root_empty_block: u64,
}

impl<Block: MaybeEmptyIntrusive> Default for IntrusiveListLevel<Block> {
    #[inline]
    fn default() -> Self {
        Self{
            //Always have empty level_block at index 0.
            blocks:vec![Block::empty()],
            root_empty_block: u64::MAX,
        }
    }
}

impl<Block: MaybeEmptyIntrusive> IntrusiveListLevel<Block> {
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
            empty_block.restore_empty();
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

impl<Block: MaybeEmptyIntrusive> ILevel for IntrusiveListLevel<Block> {
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