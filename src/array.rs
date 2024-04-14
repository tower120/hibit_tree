use std::marker::PhantomData;
use std::mem::{ManuallyDrop, MaybeUninit};
use crate::bit_block::BitBlock;
use crate::level_block::{HiBlock, is_bypass_block, LevelBlock};
use crate::level::ILevel;
use crate::level_masks::{DefaultState, SparseHierarchy, SparseHierarchyState};
use crate::bool_type::{BoolType};
use crate::primitive::Primitive;

// TODO: rename DataBlock to Data?

/*const*/ fn is_bypass_level<L>() -> bool
where
    L: ILevel,
    L::Block: HiBlock,
{
    is_bypass_block::<L::Block>()   
}


// TODO: bypass return duplicates from last active level
#[inline]
fn level_indices<Level1, Level2>(index: usize) 
    -> (usize/*level0*/, usize/*level1*/, usize/*level2*/)
where 
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock
{
    if is_bypass_level::<Level1>() {
        return (index, 0, 0)
    }
    
    // this should be const and act as const.
    /*const*/ let level2_block_capacity_pot_exp : usize = if Level2::Bypass::VALUE{0} else {<Level2::Block as HiBlock>::Mask::SIZE_POT_EXPONENT};
    /*const*/ let level2_block_capacity         : usize = 1 << level2_block_capacity_pot_exp;

    /*const*/ let level1_block_capacity_pot_exp: usize = <Level1::Block as HiBlock>::Mask::SIZE_POT_EXPONENT
                                                       + level2_block_capacity_pot_exp;
    /*const*/ let level1_block_capacity        : usize = 1 << level1_block_capacity_pot_exp;
    
    // index / LEVEL1_BLOCK_CAP
    let level0 = index >> level1_block_capacity_pot_exp;
    // index % LEVEL1_BLOCK_CAP
    let level0_remainder = index & (level1_block_capacity - 1);
    
    if is_bypass_level::<Level2>() {
        let level1 = level0_remainder;
        return (level0, level1, 0);
    }
    
    // level0_remainder / LEVEL2_BLOCK_CAP
    let level1 = level0_remainder >> level2_block_capacity_pot_exp;

    // level0_remainder % LEVEL2_BLOCK_CAP = index % LEVEL2_BLOCK_CAP % DATA_BLOCK_CAP
    let level1_remainder = index & (
        (level1_block_capacity-1) & (level2_block_capacity-1)
    );

    let level2 = level1_remainder;
    (level0, level1, level2)    
}

pub struct SparseBlockArray<Level0Block, Level1, Level2, DataLevel>
/*where
    Level0Block: HiBlock, 
    Level1   : ILevel,
    Level2   : ILevel,
    DataLevel: ILevel*/
{
    level0: Level0Block,
    level1: Level1,
    level2: Level2,
    data  : DataLevel,
}
impl<Level0Block, Level1, Level2, DataLevel> Default for
    SparseBlockArray<Level0Block, Level1, Level2, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock,
    DataLevel: ILevel
{
    #[inline]
    fn default() -> Self {
        if is_bypass_level::<Level1>(){
            assert!(
                is_bypass_level::<Level2>(), 
                "Level bypass sequence should start from level2."
            );    
        }
        
        Self{
            level0: LevelBlock::empty(),
            level1: Default::default(),
            level2: Default::default(),
            data  : Default::default(),
        }
    }
}

impl<Level0Block, Level1, Level2, DataLevel> 
    SparseBlockArray<Level0Block, Level1, Level2, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock,
    DataLevel: ILevel
{
    #[inline]
    fn level_indices(index: usize) -> (usize/*level0*/, usize/*level1*/, usize/*level2*/) {
        level_indices::<Level1, Level2>(index)
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
        let (level0_index, level1_index, level2_index) = Self::level_indices(index);
        
        let data_block_index = 
        if is_bypass_level::<Level1>() {
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
            
            let level1_block = unsafe{ self.level1.blocks_mut().get_unchecked_mut(level1_block_index) }; 
            if is_bypass_level::<Level2>() {
                // 2. Level1
                unsafe{
                    level1_block.get_or_insert(level1_index, ||{
                        let block_index = self.data.insert_empty_block();
                        Primitive::from_usize(block_index)
                    })
                }.as_usize()
            } else {
                // 2. Level1
                let level2_block_index = unsafe{
                    level1_block.get_or_insert(level1_index, ||{
                        let block_index = self.level2.insert_empty_block();
                        Primitive::from_usize(block_index)
                    })
                }.as_usize();
                    
                // 3. Level2
                unsafe{
                    let level2_block = self.level2.blocks_mut().get_unchecked_mut(level2_block_index);
                    level2_block.get_or_insert(level2_index, ||{
                        let block_index = self.data.insert_empty_block();
                        Primitive::from_usize(block_index)
                    })
                }.as_usize()
            }
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
        todo!()
        /*let (level0_index, level1_index) = Self::level_indices(index);
        
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index);
        let data_block_index = level1_block.get_or_zero(level1_index).as_usize();
        let data_block = self.data.blocks().get_unchecked(data_block_index);
        data_block*/
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



impl<Level0Block, Level1, Level2, DataLevel> SparseHierarchy for 
    SparseBlockArray<Level0Block, Level1, Level2, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock,
    DataLevel: ILevel,
    DataLevel::Block: Clone,
{
    const EXACT_HIERARCHY: bool = true;
    
    type Level0MaskType = Level0Block::Mask;
    type Level0Mask<'a> = &'a Level0Block::Mask where Self: 'a;
    #[inline]
    fn level0_mask(&self) -> Self::Level0Mask<'_> {
        self.level0.mask()
    }
    
    type Level1MaskType = <Level1::Block as HiBlock>::Mask;
    type Level1Mask<'a> = &'a <Level1::Block as HiBlock>::Mask where Self: 'a;
    #[inline]
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask<'_> {
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index);
        level1_block.mask()
    }
    
    type Level2MaskType = <Level2::Block as HiBlock>::Mask;
    type Level2Mask<'a> = &'a <Level2::Block as HiBlock>::Mask where Self: 'a;
    #[inline]
    unsafe fn level2_mask(&self, level0_index: usize, level1_index: usize) -> Self::Level2Mask<'_> {
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        let level1_block = self.level1.blocks().get_unchecked(level1_block_index);

        let level2_block_index = level1_block.get_or_zero(level1_index).as_usize();
        let level2_block = self.level2.blocks().get_unchecked(level2_block_index);
        
        level2_block.mask()
    }    

    type DataBlockType = DataLevel::Block;
    type DataBlock<'a> = &'a DataLevel::Block where Self: 'a;
    #[inline]
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize, level2_index: usize) -> Self::DataBlock<'_> {
        let level1_block_index = self.level0.get_or_zero(level0_index).as_usize();
        
        let data_block_index =
        if is_bypass_level::<Level1>(){
            level1_block_index
        } else {
            let level1_block = self.level1.blocks().get_unchecked(level1_block_index);
            let level2_block_index = level1_block.get_or_zero(level1_index).as_usize();
            
            if is_bypass_level::<Level2>(){
                level2_block_index
            } else {
                let level2_block = self.level2.blocks().get_unchecked(level2_block_index);
                level2_block.get_or_zero(level2_index).as_usize()
            }
        };

        self.data.blocks().get_unchecked(data_block_index)
    }

    type State = SparseBlockArrayState<Level0Block, Level1, Level2, DataLevel>;
}

pub struct SparseBlockArrayState<Level0Block, Level1, Level2, DataLevel>
where
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock,
{
    /// Points to the element in the heap. Guaranteed to be stable.
    level1_block_meta: <Level1::Block as HiBlock>::Meta,
    /// Points to the element in the heap. Guaranteed to be stable.
    level2_block_meta: <Level2::Block as HiBlock>::Meta,

    phantom_data: PhantomData<SparseBlockArray<Level0Block, Level1, Level2, DataLevel>>
}

impl<Level0Block, Level1, Level2, DataLevel> SparseHierarchyState for 
    SparseBlockArrayState<Level0Block, Level1, Level2, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock,
    DataLevel: ILevel,
    DataLevel::Block: Clone,
{
    type This = SparseBlockArray<Level0Block, Level1, Level2, DataLevel>;

    #[inline]
    fn new(_: &Self::This) -> Self {
        Self{
            level1_block_meta: Default::default(),
            level2_block_meta: Default::default(),
            phantom_data: PhantomData
        }
    }

    #[inline]
    unsafe fn select_level1<'a>(
        &mut self,
        this: &'a Self::This,
        level0_index: usize
    ) -> (<Self::This as SparseHierarchy>::Level1Mask<'a>, bool){
        let level1_block_index = this.level0.get_or_zero(level0_index);
        let level1_block = this.level1.blocks().get_unchecked(level1_block_index.as_usize());
        self.level1_block_meta = From::from(level1_block);
        (level1_block.mask(), !level1_block_index.is_zero())
    }
    
    #[inline]
    unsafe fn select_level2<'a>(
        &mut self,
        this: &'a Self::This,
        level1_index: usize
    ) -> (<Self::This as SparseHierarchy>::Level2Mask<'a>, bool){
        let level1_block = self.level1_block_meta.as_ref();
        let level2_block_index = level1_block.get_or_zero(level1_index);
        let level2_block = this.level2.blocks().get_unchecked(level2_block_index.as_usize());
        self.level2_block_meta = From::from(level2_block);
        (level2_block.mask(), !level2_block_index.is_zero())        
    }
    
    #[inline]
    unsafe fn data_block<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::DataBlock<'a> {
        let data_block_index = 
        if is_bypass_level::<Level1>(){
            this.level0.get_or_zero(level_index).as_usize()
        } else if is_bypass_level::<Level2>(){
            let level1_block = self.level1_block_meta.as_ref();
            level1_block.get_or_zero(level_index).as_usize()
        } else {
            let level2_block = self.level2_block_meta.as_ref();
            level2_block.get_or_zero(level_index).as_usize()
        };
        this.data.blocks().get_unchecked(data_block_index)        
    }
}


/*impl <Level0Block, Level1, Level2, DataLevel> LevelMasksBorrow
    for SparseBlockArray<Level0Block, Level1, Level2, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock,
    DataLevel: ILevel,
    DataLevel::Block: Clone,
{
    type Type = Self;
}

impl <Level0Block, Level1, Level2, DataLevel> LevelMasksBorrow
    for &SparseBlockArray<Level0Block, Level1, Level2, DataLevel>
where
    Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock,
    DataLevel: ILevel,
    DataLevel::Block: Clone,
{
    type Type = SparseBlockArray<Level0Block, Level1, Level2, DataLevel>;
}*/