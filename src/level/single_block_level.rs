use std::slice;
use crate::level::ILevel;
use crate::Empty;

#[derive(Clone)]
pub struct SingleBlockLevel<Block: Empty>{
    block: Block
}

impl<Block: Empty> ILevel for SingleBlockLevel<Block>{
    type Block = Block;

    #[inline]
    fn blocks(&self) -> &[Self::Block] {
        unsafe{ slice::from_raw_parts(&self.block, 1) }
    }

    #[inline]
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

impl<Block: Empty> Default for SingleBlockLevel<Block> {
    #[inline]
    fn default() -> Self {
        Self{ block: Block::empty() }
    }
}
