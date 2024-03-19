use std::marker::PhantomData;
use crate::bit_block::BitBlock;
use crate::block::{Block, HiBlock};
use crate::level::Level;
use crate::{LevelMasks, ref_or_val};
use crate::primitive::Primitive;

// TODO: rename DataBlock to Data?

#[inline]
fn level_indices<Level1Block: HiBlock>(index: usize) 
    -> (usize/*level0*/, usize/*level1*/)
{
    // this should be const and act as const.
    /*const*/ let level1_block_capacity_pot_exp: usize = Level1Block::Mask::SIZE_POT_EXPONENT;
    /*const*/ let level1_block_capacity        : usize = 1 << level1_block_capacity_pot_exp;

    // index / LEVEL1_BLOCK_CAP
    let level0 = index >> level1_block_capacity_pot_exp;
    // index % LEVEL1_BLOCK_CAP
    let level0_remainder = index & (level1_block_capacity - 1);

    let level1 = level0_remainder;

    (level0, level1)
}

pub struct SparseBlockArray<Level0Block, Level1Block, DataBlock>
where
    Level0Block: HiBlock,
    Level1Block: HiBlock,
    DataBlock: Block,
{
    level0: Level0Block,
    level1: Level<Level1Block>,
    data  : Level<DataBlock>,
    //phantom: PhantomData<Conf>
}
impl<Level0Block, Level1Block, DataBlock> Default for
    SparseBlockArray<Level0Block, Level1Block, DataBlock>
where
    Level0Block: HiBlock,
    Level1Block: HiBlock,
    DataBlock: Block,
{
    #[inline]
    fn default() -> Self {
        Self{
            level0: Block::empty(),
            level1: Default::default(),
            data: Default::default(),
        }
    }
}

impl<Level0Block, Level1Block, DataBlock> 
    SparseBlockArray<Level0Block, Level1Block, DataBlock>
where
    Level0Block: HiBlock,
    Level1Block: HiBlock,
    DataBlock: Block,
{
    #[inline]
    fn level_indices(index: usize) -> (usize/*level0*/, usize/*level1*/) {
        level_indices::<Level1Block>(index)
    }
    
    // get_mut
    
    /// Fail to do so will brake TRUSTED_HIERARCHY container promise.
    /// 
    /// # Safety
    /// 
    /// Pointed block at `index` must exists and be empty
    pub unsafe fn remove_empty_unchecked(&mut self, index: usize){
        todo!()
    }
    
    /// Inserts and return empty block, if not exists.
    /// 
    /// If returned DataBlock will end up empty - you MUST
    /// call [remove_empty_unchecked].
    pub fn get_or_insert(&mut self, index: usize) -> &mut DataBlock {
        //assert!(Self::is_in_range(index), "index out of range!");

        // That's indices to next level
        let (level0_index, level1_index) = Self::level_indices(index);

        // 1. Level0
        let level1_block_index = unsafe{
            self.level0.get_or_insert(level0_index, ||{
                let block_index = self.level1.insert_empty_block();
                Primitive::from_usize(block_index)
            })
        }.as_usize();

        // 2. Level1
        let data_block_index = unsafe{
            let level1_block = self.level1.blocks_mut().get_unchecked_mut(level1_block_index);
            level1_block.get_or_insert(level1_index, ||{
                let block_index = self.data.insert_empty_block();
                Primitive::from_usize(block_index)
            })
        }.as_usize();

        // 3. Data level
        unsafe{
            let data_block = self.data.blocks_mut().get_unchecked_mut(data_block_index);
            data_block
        }        
    }
    
    /// # Safety
    /// 
    /// `index` must be within SparseBlockArray range.
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &DataBlock {
        let (level0_index, level1_index) = Self::level_indices(index);
        
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index);
        let data_block_index = level1_block.get_or_zero(level1_index).as_usize();
        let data_block = self.data.blocks().get_unchecked(data_block_index);
        data_block
    }
    
/*    // TODO: There could be safe NonEmptyDataBlock
    /// # Safety
    ///
    /// * `block` must be non-empty.
    /// Will panic, if `index` is out of range.
    pub unsafe fn set_non_empty_unchecked(&mut self, index: usize, block: DataBlock){
        //assert!(Self::is_in_range(index), "index out of range!");

        // That's indices to next level
        let (level0_index, level1_index) = Self::level_indices(index);

        // 1. Level0
        let level1_block_index = unsafe{
            self.level0.get_or_insert(level0_index, ||{
                let block_index = self.level1.insert_block();
                Primitive::from_usize(block_index)
            })
        }.as_usize();

        // 2. Level1
        let data_block_index = unsafe{
            let level1_block = self.level1.blocks_mut().get_unchecked_mut(level1_block_index);
            level1_block.get_or_insert(level1_index, ||{
                let block_index = self.data.insert_block();
                Primitive::from_usize(block_index)
            })
        }.as_usize();

        // 3. Data level
        unsafe{
            let data_block = self.data.blocks_mut().get_unchecked_mut(data_block_index);
            data_block.mask_mut().set_bit::<true>(data_index);
        }
    }  */  
}



impl<Level0Block, Level1Block, DataBlock> LevelMasks for 
    SparseBlockArray<Level0Block, Level1Block, DataBlock>
where
    Level0Block: HiBlock,
    Level1Block: HiBlock,
    DataBlock: Block + Clone,
{
    type Level0Mask = Level0Block::Mask;
    #[inline]
    fn level0_mask(&self) -> Self::Level0Mask {
        self.level0.mask().clone()
    }

    type Level1Mask = Level1Block::Mask;
    #[inline]
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask {
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index);
        level1_block.mask().clone()
    }

    type DataBlock = DataBlock;
    #[inline]
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize) -> Self::DataBlock {
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index);

        let data_block_index = level1_block.get_or_zero(level1_index).as_usize();
        let data_block = self.data.blocks().get_unchecked(data_block_index);
        data_block.clone()
    }
}

ref_or_val!(
    impl <Level0Block, Level1Block, DataBlock> for 
        ref SparseBlockArray<Level0Block, Level1Block, DataBlock>
    where
        Level0Block: HiBlock,
        Level1Block: HiBlock,
        DataBlock: Block,
);