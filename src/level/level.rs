use crate::level::ILevel;
use crate::Empty;

/// Simple level implementation. Works with all `Block`s.
///
/// Prefer using [IntrusiveListLevel] whenever possible.
pub struct Level<Block: Empty>{
    blocks: Vec<Block>,
    empty_block_indices: Vec<usize>
}

impl<Block: Empty> Default for Level<Block> {
    #[inline]
    fn default() -> Self {
        Self{
            //Always have empty level_block at index 0.
            blocks:vec![Block::empty()],
            empty_block_indices: Vec::new()
        }
    }
}

impl<Block: Empty> ILevel for Level<Block> {
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
        if let Some(index) = self.empty_block_indices.pop(){
            index
        } else {
            let index = self.blocks.len();
            self.blocks.push(Block::empty());
            index
        }
    }

    #[inline]
    unsafe fn remove_empty_block_unchecked(&mut self, block_index: usize) {
         self.empty_block_indices.push(block_index);
    }
}