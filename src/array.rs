use std::mem::MaybeUninit;
use crate::bit_block::BitBlock;
use crate::level_block::{HiBlock, LevelBlock};
use crate::level::ILevel;
use crate::{LevelMasks, LevelMasksBorrow};
use crate::bool_type::BoolType;
use crate::level_masks::{LevelMasksIter, NoState};
use crate::primitive::Primitive;

// TODO: rename DataBlock to Data?

#[inline]
fn level_indices<Level1>(index: usize) 
    -> (usize/*level0*/, usize/*level1*/)
where 
    Level1: ILevel,
    Level1::Block: HiBlock
{
    if Level1::Bypass::VALUE {
        return (index, 0)
    }
    
    // this should be const and act as const.
    /*const*/ let level1_block_capacity_pot_exp: usize = <Level1::Block as HiBlock>::Mask::SIZE_POT_EXPONENT;
    /*const*/ let level1_block_capacity        : usize = 1 << level1_block_capacity_pot_exp;

    // index / LEVEL1_BLOCK_CAP
    let level0 = index >> level1_block_capacity_pot_exp;
    // index % LEVEL1_BLOCK_CAP
    let level0_remainder = index & (level1_block_capacity - 1);

    let level1 = level0_remainder;

    (level0, level1)
}

pub struct SparseBlockArray<Level0Block, Level1, DataLevel>
where
    Level0Block: HiBlock, 
    Level1   : ILevel,
    DataLevel: ILevel
{
    level0: Level0Block,
    level1: Level1,
    data  : DataLevel,
}
impl<Level0Block, Level1, DataLevel> Default for
    SparseBlockArray<Level0Block, Level1, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    DataLevel: ILevel
{
    #[inline]
    fn default() -> Self {
        Self{
            level0: LevelBlock::empty(),
            level1: Default::default(),
            data  : Default::default(),
        }
    }
}

impl<Level0Block, Level1, DataLevel> 
    SparseBlockArray<Level0Block, Level1, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    DataLevel: ILevel
{
    #[inline]
    fn level_indices(index: usize) -> (usize/*level0*/, usize/*level1*/) {
        level_indices::<Level1>(index)
    }
    
    // get_mut
    
    /// Fail to do so will brake TRUSTED_HIERARCHY container promise.
    /// 
    /// # Safety
    /// 
    /// Pointed level_block at `index` must exist and be empty.
    pub unsafe fn remove_empty_unchecked(&mut self, index: usize){
        todo!()
    }
    
    /// Inserts and return empty level_block, if not exists.
    /// 
    /// If returned DataBlock will end up empty - you MUST
    /// call [remove_empty_unchecked].
    pub fn get_or_insert(&mut self, index: usize) -> &mut DataLevel::Block {
        //assert!(Self::is_in_range(index), "index out of range!");

        // That's indices to the next level
        let (level0_index, level1_index) = Self::level_indices(index);
        
        let data_block_index = 
        if Level1::Bypass::VALUE {
             unsafe{
                self.level0.get_or_insert(level0_index, ||{
                    let block_index = self.data.insert_empty_block();
                    Primitive::from_usize(block_index)
                })
            }.as_usize()
        } else {
            // 1. Level0
            let level1_block_index = unsafe{
                self.level0.get_or_insert(level0_index, ||{
                    let block_index = self.level1.insert_empty_block();
                    Primitive::from_usize(block_index)
                })
            }.as_usize();
    
            // 2. Level1
            unsafe{
                let level1_block = self.level1.blocks_mut().get_unchecked_mut(level1_block_index);
                level1_block.get_or_insert(level1_index, ||{
                    let block_index = self.data.insert_empty_block();
                    Primitive::from_usize(block_index)
                })
            }.as_usize()
        };

        // 3. Data level
        unsafe{
            let data_block = self.data.blocks_mut().get_unchecked_mut(data_block_index);
            data_block
        }        
    }
    
    // TODO: Refactor - LevelMasks have data_block
    /// # Safety
    /// 
    /// `index` must be within SparseBlockArray range.
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &DataLevel::Block {
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



impl<Level0Block, Level1, DataLevel> LevelMasks for 
    SparseBlockArray<Level0Block, Level1, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    DataLevel: ILevel,
    DataLevel::Block: Clone,
{
    type Level0MaskType = Level0Block::Mask;
    type Level0Mask<'a> = &'a Level0Block::Mask where Self: 'a;
    #[inline]
    fn level0_mask(&self) -> Self::Level0Mask<'_> {
        self.level0.mask()
    }

    type Level1Bypass   = Level1::Bypass;
    type Level1MaskType = <Level1::Block as HiBlock>::Mask;
    type Level1Mask<'a> = &'a <Level1::Block as HiBlock>::Mask where Self: 'a;
    #[inline]
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask<'_> {
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index);
        level1_block.mask()
    }

    type DataBlockType = DataLevel::Block;
    type DataBlock<'a> = &'a DataLevel::Block where Self: 'a;
    #[inline]
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize) -> Self::DataBlock<'_> {
        // TODO: bypass
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index);

        let data_block_index = level1_block.get_or_zero(level1_index).as_usize();
        let data_block = self.data.blocks().get_unchecked(data_block_index);
        data_block
    }
}

impl<Level0Block, Level1, DataLevel> LevelMasksIter for 
    SparseBlockArray<Level0Block, Level1, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    DataLevel: ILevel,
    DataLevel::Block: Clone,
{
    type IterState = NoState<Self>;
    
    /// Points to the element in the heap. Guaranteed to be stable.
    type Level1BlockMeta = <Level1::Block as HiBlock>::Meta; 

    #[inline]
    unsafe fn init_level1_block_meta(
        &self,
        _: &mut Self::IterState,
        level1_block_meta: &mut MaybeUninit<Self::Level1BlockMeta>,
        level0_index: usize
    ) -> (Self::Level1Mask<'_>, bool) {
        let level1_block_index = self.level0.get_or_zero(level0_index);
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index.as_usize());
        level1_block_meta.write( Self::Level1BlockMeta::from(level1_block) );
        (level1_block.mask(), !level1_block_index.is_zero())
    }

    #[inline]
    unsafe fn data_block_from_meta(
        &self,
        level1_block_meta: &Self::Level1BlockMeta,
        level1_index: usize
    ) -> Self::DataBlock<'_> {
        let data_block_index = if Level1::Bypass::VALUE{
            let level0_index = level1_index;
            self.level0.get_or_zero(level0_index).as_usize()
        } else {
            let level1_block = level1_block_meta.as_ref();
            let data_block_index = level1_block.get_or_zero(level1_index).as_usize();
            data_block_index
        };
        
        self.data.blocks().get_unchecked(data_block_index)
    }
}

impl <Level0Block, Level1, DataLevel> LevelMasksBorrow
    for SparseBlockArray<Level0Block, Level1, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    DataLevel: ILevel,
    DataLevel::Block: Clone,
{
    type Type = Self;
}

impl <Level0Block, Level1, DataLevel> LevelMasksBorrow
    for &SparseBlockArray<Level0Block, Level1, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    DataLevel: ILevel,
    DataLevel::Block: Clone,
{
    type Type = SparseBlockArray<Level0Block, Level1, DataLevel>;
}