// TODO: rename mod to sparse_array.rs

use std::marker::PhantomData;
use std::mem;
use std::ops::ControlFlow;
use std::ptr::{NonNull, null};
use crate::bit_block::BitBlock;
use crate::level_block::{HiBlock, is_bypass_block, LevelBlock};
use crate::level::{bypass_level, bypass_level_ref, BypassLevel, ILevel};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::bool_type::{BoolType};
use crate::const_int::{const_for, ConstInt, ConstInteger, ConstIntVisitor};
use crate::primitive::Primitive;
use crate::primitive_array::Array;
use crate::PrimitiveArray;

// TODO: rename DataBlock to Data?
#[deprecated]
/*const*/ fn is_bypass_level<L>() -> bool
where
    L: ILevel,
    L::Block: HiBlock,
{
    is_bypass_block::<L::Block>()   
}



/*#[inline]
fn level_indices2<Levels>(index: usize) 
    -> Levels::LevelIndices
where
    Levels: ArrayLevels,
    Levels::LevelIndices: Default,
{
    //let mut indices: Levels::LevelIndices = Default::default();
    match Levels::LevelCount::N{
        1 => {
            let tuple = level_indices::<BypassLevel, BypassLevel>(index);
            PrimitiveArray::from_array([tuple.0])
            /*[0,1,2].from
            indices.as_mut()[0] = tuple.0;*/
        },
        2 => {
            let tuple = level_indices::<Levels::L1, BypassLevel>(index);
            PrimitiveArray::from_array([tuple.0, tuple.1])
            /*indices.as_mut()[0] = tuple.0;
            indices.as_mut()[1] = tuple.1;*/
        },
        3 => {
            let tuple = level_indices::<Levels::L1, Levels::L2>(index);
            PrimitiveArray::from_array([tuple.0, tuple.1, tuple.2])
            /*indices.as_mut()[0] = tuple.0;
            indices.as_mut()[1] = tuple.1;
            indices.as_mut()[2] = tuple.2;*/
        },
        _ => unreachable!()
    } 
    //indices
}*/


// Compile-time loop inside. Ends up with just N ANDs and SHRs.
#[inline]
pub(crate) fn level_indices_new<LevelMask, LevelIndices>(index: usize) 
    -> LevelIndices
where
    LevelMask: BitBlock,
    LevelIndices: PrimitiveArray<Item = usize> + Default
{
    let mut level_indices = LevelIndices::default();
    
    let mut level_remainder = index;
    let level_count = LevelIndices::CAP;
    for level in 0..level_count - 1{
        //let rev_level = level_count - level;
        let level_capacity_exp = LevelMask::SIZE_POT_EXPONENT * (level_count - level - 1);
        let level_capacity = 1 << level_capacity_exp;
        
        // level_remainder / level_capacity_exp
        let level_index = level_remainder >> level_capacity_exp;
        
        // level_remainder % level_capacity_exp
        level_remainder = level_remainder & (level_capacity - 1);
        
        level_indices.as_mut()[level] = level_index; 
    }
    
    *level_indices.as_mut().last_mut().unwrap() = level_remainder; 
    
    level_indices
}

/*
// Exactly same generated code as with level_indices_new 
#[inline]
pub(crate) fn level_indices_new2<LevelMask, LevelIndices>(index: usize) 
    -> LevelIndices
where
    LevelMask: BitBlock,
    LevelIndices: PrimitiveArray<Item = usize> + Default
{
    let mut level_indices = LevelIndices::default();
    
    let mut level_remainder = index;
    let mut level_capacities_1_anded = usize::MAX; 
    let level_count = LevelIndices::CAP - 1;
    for level in 0..level_count{
        let rev_level = level_count - level;
        let level_capacity_exp = LevelMask::SIZE_POT_EXPONENT * rev_level;
        let level_capacity = 1 << level_capacity_exp;
        
        let level_index = level_remainder >> level_capacity_exp;
        
        //level_remainder = level_remainder & (level_capacity - 1);
        // Alternatively:
        // level_remainder = index & ((level_capacity - 1) & .. all prev (level_capacity - 1));
        level_capacities_1_anded &= (level_capacity - 1);
        level_remainder = level_remainder & level_capacities_1_anded;
        
        
        level_indices.as_mut()[level] = level_index; 
    }
    
    *level_indices.as_mut().last_mut().unwrap() = level_remainder; 
    
    level_indices
}
*/

#[test]
fn test_level_indices_new(){
    {
        let indices = level_indices_new::<u64, [usize; 2]>(65);
        assert_eq!(indices, [1, 1]);
    }
    {
        let lvl0 = 262_144; // Total max capacity
        let lvl1 = 4096;
        let lvl2 = 64;
        let indices = level_indices_new::<u64, [usize; 3]>(lvl1*2 + lvl2*3 + 4);
        assert_eq!(indices, [2, 3, 4]);
    }
    {
        let indices = level_indices_new::<u64, [usize; 3]>(32);
        assert_eq!(indices, [0, 0, 32]);
    }
    {
        let indices = level_indices_new::<u64, [usize; 2]>(32);
        assert_eq!(indices, [0, 32]);
    }    
    {
        let indices = level_indices_new::<u64, [usize; 1]>(32);
        assert_eq!(indices, [32]);
    }
}


// TODO: bypass return duplicates from last active level?
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
    /*const*/ let level2_block_capacity_pot_exp : usize = if is_bypass_level::<Level2>(){0} else {<Level2::Block as HiBlock>::Mask::SIZE_POT_EXPONENT};
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


// TODO: Can be removed
pub trait HiLevel: ILevel<Block: HiBlock>{}
impl<T: ILevel<Block: HiBlock>> HiLevel for T{}


pub trait Visitor<Mask> {
    type Out;
    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L) -> Self::Out
    where
        L: ILevel,
        L::Block: HiBlock<Mask = Mask>
    ;
}

pub trait MutVisitor {
    type Out;
    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &mut L) -> Self::Out
    where
        L: ILevel,
        L::Block: HiBlock
    ;
}

pub trait FoldVisitor {
    type Acc;
    fn visit<I: ConstInteger, L>(&mut self, i: I, acc: Self::Acc, level: &L) -> Self::Acc
    where
        L: ILevel,
        L::Block: HiBlock
    ;
}

pub trait FoldMutVisitor {
    type Acc;
    fn visit<I: ConstInteger, L>(&mut self, i: I, acc: Self::Acc, level: &mut L) -> Self::Acc
    where
        L: ILevel,
        L::Block: HiBlock
    ;
}


// TODO: HiLevels?
pub trait ArrayLevels{
    //const LEVEL_COUNT: ConstInteger;
    type LevelCount: ConstInteger;
    type LevelIndices : PrimitiveArray<Item = usize> + Default;
    
    // TODO: Use const* u8 directly?
    /// Starts from level1. Since level0 block fixed.
    //type LevelBlockPtrs: Array<Item = Option<NonNull<u8>> /*CAP = Self::LEVEL_COUNT*/> + Default;
    
    // Need this for SparseHierarchy
    //type DataBlockIndices : PrimitiveArray<Item = usize>;

    fn new() -> Self;
    
    // type L0: ILevel<Block: HiBlock>;
    // type L1: ILevel<Block: HiBlock>/*<Mask = <Self::L0 as HiBlock>::Mask>> where <<Self as ArrayLevels>::L0 as ILevel>::Block: HiBlock*/;
    // type L2: ILevel<Block: HiBlock/*<Mask = <Self::L0 as HiBlock>::Mask>*/>;
    
    type Mask: BitBlock;
    
/*    // TODO: one common mask?
    type L0: ILevel<Block: HiBlock>;
    type L1: HiLevel<Block: HiBlock<Mask = <<Self::L0 as ILevel>::Block as HiBlock>::Mask>>;
    type L2: HiLevel;   */
    
    
    fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, visitor: V) -> V::Out;
    fn visit_mut<I: ConstInteger, V: MutVisitor>(&mut self, i: I, visitor: V) -> V::Out;
    
    
    /*// TODO: Remove
    fn foreach(&mut self, visitor: impl Visitor);*/
    
    fn fold<Acc>(&self, acc: Acc, visitor: impl FoldVisitor<Acc=Acc>) -> Acc;
    fn fold_mut<Acc>(&mut self, acc: Acc, visitor: impl FoldMutVisitor<Acc=Acc>) -> Acc;
}

// TODO: macro impl?

impl<L0> ArrayLevels for (L0,)
where
    L0: ILevel,
    L0::Block: HiBlock,
{
    type LevelCount = ConstInt<1>;
    type LevelIndices = [usize; 1];

    fn new() -> Self {
        (L0::default(),)
    }

    type Mask = <L0::Block as HiBlock>::Mask;
    
    fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, mut visitor: V) -> V::Out {
        match i.value() {
            0 => visitor.visit(i, &self.0),
            _ => unreachable!()
        }
    }
    
    fn visit_mut<I: ConstInteger, V: MutVisitor>(&mut self, i: I, mut visitor: V) -> V::Out {
        match i.value() {
            0 => visitor.visit(i, &mut self.0),
            _ => unreachable!()
        }
    }
    
    /*fn foreach(&mut self, mut visitor: impl Visitor){
        visitor.visit::<0, _>(&mut self.0);
    }*/
    
    fn fold<Acc>(&self, acc: Acc, mut visitor: impl FoldVisitor<Acc = Acc>) -> Acc {
        visitor.visit(ConstInt::<0>::DEFAULT, acc, &self.0)
    }

    fn fold_mut<Acc>(&mut self, acc: Acc, mut visitor: impl FoldMutVisitor<Acc = Acc>) -> Acc {
        visitor.visit(ConstInt::<0>::DEFAULT, acc, &mut self.0)
    }
}

impl<L0, L1> ArrayLevels for (L0, L1)
where
    L0: ILevel,
    L0::Block: HiBlock,
    L1: ILevel,
    L1::Block: HiBlock<Mask = <L0::Block as HiBlock>::Mask>,
{
    type LevelCount = ConstInt<2>;
    type LevelIndices = [usize; 2];

    fn new() -> Self {
        (L0::default(), L1::default())
    }
    
    type Mask = <L0::Block as HiBlock>::Mask;

    fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, mut visitor: V) -> V::Out {
        match i.value(){
            0 => visitor.visit(i, &self.0),
            1 => visitor.visit(i, &self.1),
            _ => unreachable!()
        }
    }
    
    fn visit_mut<I: ConstInteger, V: MutVisitor>(&mut self, i: I, mut visitor: V) -> V::Out {
        match i.value(){
            0 => visitor.visit(i, &mut self.0),
            1 => visitor.visit(i, &mut self.1),
            _ => unreachable!()
        }
    }
    
    /*fn foreach(&mut self, mut visitor: impl Visitor){
        visitor.visit::<0, _>(&mut self.0);
    }*/
    fn fold<Acc>(&self, mut acc: Acc, mut visitor: impl FoldVisitor<Acc = Acc>) -> Acc {
        acc = visitor.visit(ConstInt::<0>::DEFAULT, acc, &self.0);
        visitor.visit(ConstInt::<1>::DEFAULT, acc, &self.1)
    }
    

    fn fold_mut<Acc>(&mut self, mut acc: Acc, mut visitor: impl FoldMutVisitor<Acc = Acc>) -> Acc {
        acc = visitor.visit(ConstInt::<0>::DEFAULT, acc, &mut self.0);
        visitor.visit(ConstInt::<1>::DEFAULT, acc, &mut self.1)
    }
}

impl<L0, L1, L2> ArrayLevels for (L0, L1, L2)
where
    L0: ILevel,
    L0::Block: HiBlock,
    L1: ILevel,
    L1::Block: HiBlock<Mask = <L0::Block as HiBlock>::Mask>,
    L2: ILevel,
    L2::Block: HiBlock<Mask = <L0::Block as HiBlock>::Mask>,
{
    type LevelCount = ConstInt<3>;
    type LevelIndices = [usize; 3];

    fn new() -> Self {
        (L0::default(), L1::default(), L2::default())
    }
    
    type Mask = <L0::Block as HiBlock>::Mask;

    fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, mut visitor: V) -> V::Out {
        match i.value(){
            0 => visitor.visit(i, &self.0),
            1 => visitor.visit(i, &self.1),
            2 => visitor.visit(i, &self.2),
            _ => unreachable!()
        }
    }
    
    fn visit_mut<I: ConstInteger, V: MutVisitor>(&mut self, i: I, mut visitor: V) -> V::Out {
        match i.value(){
            0 => visitor.visit(i, &mut self.0),
            1 => visitor.visit(i, &mut self.1),
            2 => visitor.visit(i, &mut self.2),
            _ => unreachable!()
        }
    }
    
    /*fn foreach(&mut self, mut visitor: impl Visitor){
        visitor.visit::<0, _>(&mut self.0);
    }*/
    fn fold<Acc>(&self, mut acc: Acc, mut visitor: impl FoldVisitor<Acc = Acc>) -> Acc {
        acc = visitor.visit(ConstInt::<0>::DEFAULT, acc, &self.0);
        acc = visitor.visit(ConstInt::<1>::DEFAULT, acc, &self.1);
        visitor.visit(ConstInt::<2>::DEFAULT, acc, &self.2)
    }
    
    fn fold_mut<Acc>(&mut self, mut acc: Acc, mut visitor: impl FoldMutVisitor<Acc = Acc>) -> Acc {
        acc = visitor.visit(ConstInt::<0>::DEFAULT, acc, &mut self.0);
        acc = visitor.visit(ConstInt::<1>::DEFAULT, acc, &mut self.1);
        visitor.visit(ConstInt::<2>::DEFAULT, acc, &mut self.2)
    }
}


/*impl<L0, L1> ArrayLevels for (L0, L1){
    const LEVEL_COUNT: usize = 2;
    type DataBlockIndices = [usize; 2];
}
impl<L0, L1, L2> ArrayLevels for (L0, L1, L2){
    const LEVEL_COUNT: usize = 3;
    type DataBlockIndices = [usize; 3];
}*/


pub struct SparseBlockArray<Levels, DataLevel> {
    levels: Levels,
    data  : DataLevel,
}
impl<Levels, DataLevel> Default for
    SparseBlockArray<Levels, DataLevel>
where
    Levels: ArrayLevels,
    DataLevel: ILevel
{
    #[inline]
    fn default() -> Self {        
        Self{
            levels: Levels::new(),
            data  : Default::default(),
        }
    }
}

impl<Levels, DataLevel> SparseBlockArray<Levels, DataLevel>
where
    /* Level0Block: HiBlock,
    Level1: ILevel,
    Level1::Block: HiBlock,
    Level2: ILevel,
    Level2::Block: HiBlock, */
    Levels: ArrayLevels,
    DataLevel: ILevel
{
    #[inline]
    fn level_indices(index: usize) -> (usize/*level0*/, usize/*level1*/, usize/*level2*/) {
        todo!()
        //level_indices::<Level1, Level2>(index)
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
        
        struct V<Levels, DataLevel>
        where
            Levels: ArrayLevels
        {
            this: NonNull<SparseBlockArray<Levels, DataLevel>>, 
            level_indices: Levels::LevelIndices
        }
        
        impl<Levels: ArrayLevels, DataLevel: ILevel> FoldMutVisitor for V<Levels, DataLevel>{
            type Acc = usize;
            fn visit<I: ConstInteger, L: ILevel>(&mut self, i: I, level_index: usize, level: &mut L) -> usize
            where
                L::Block: HiBlock
            {
            unsafe{
                let block = level.blocks_mut().get_unchecked_mut(level_index);
                block.get_or_insert(self.level_indices.as_ref()[I::VALUE], ||{
                    let block_index = 
                        if I::VALUE == Levels::LevelCount::VALUE - 1{
                            self.this.as_mut().data.insert_empty_block()
                        } else {
                            struct Insert;
                            impl MutVisitor for Insert {
                                type Out = usize;
                                fn visit<I:ConstInteger, L: ILevel>(&mut self, i: I, level: &mut L) -> usize {
                                    level.insert_empty_block()
                                }
                            }
                            self.this.as_mut().levels.visit_mut(i.next(), Insert)
                        };
                    Primitive::from_usize(block_index)
                }).as_usize()
            }
            }
        }
        let i = level_indices_new::<Levels::Mask, Levels::LevelIndices>(index);
        
        let this = NonNull::new(self).unwrap();
        //self.levels.foreach(V{this, level_indices: i, level_index: 0});
        let data_block_index = self.levels.fold_mut(0, V{this, level_indices: i});
        
        /*//assert!(Self::is_in_range(index), "index out of range!");

        // That's indices to the next level
        let (level0_index, level1_index, level2_index) = Self::level_indices(index);
        
        let data_block_index = 
        if 
            Levels::LEVEL_COUNT == 1
            //is_bypass_level::<Level1>() 
        {
             unsafe{
                self.levels.levels().0.get_or_insert(level0_index, ||{
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
        };*/

        // 3. Data level
        unsafe{
            let data_block = self.data.blocks_mut().get_unchecked_mut(data_block_index);
            data_block
        }  
    }
    
/*    // TODO: Refactor - LevelMasks have data_block
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
    }*/
    
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



impl<Levels, DataLevel> SparseHierarchy for SparseBlockArray<Levels, DataLevel>
where
    Levels: ArrayLevels,
    DataLevel: ILevel,
    DataLevel::Block: Clone
{
    const EXACT_HIERARCHY: bool = true;
    
    //const LEVELS_COUNT: usize = Levels::LEVEL_COUNT;
    type LevelCount = Levels::LevelCount;
    //type LevelMaskType = <<Levels::L0 as ILevel>::Block as HiBlock>::Mask;
    type LevelMaskType = Levels::Mask;
    type LevelMask<'a> where Self: 'a = &'a Self::LevelMaskType;

    /*fn level_mask<const N: usize>(&self, level_indices: [usize; N]) -> Self::LevelMask<'_> {
        todo!()
    }*/

    type DataBlockIndices = Levels::LevelIndices;
    type DataBlockType = DataLevel::Block;
    type DataBlock<'a> where Self: 'a = &'a Self::DataBlockType;
    
    #[inline]
    unsafe fn data_block(&self, level_indices: Self::DataBlockIndices) -> Self::DataBlock<'_> {
        struct V<LevelIndices>(LevelIndices);
        impl<LevelIndices: PrimitiveArray<Item=usize>> FoldVisitor for V<LevelIndices>{
            type Acc = usize;
            fn visit<I: ConstInteger, L>(&mut self, i: I, level_block_index: usize, level: &L) 
                -> Self::Acc 
            where 
                L: ILevel, 
                L::Block: HiBlock 
            {
                unsafe{
                    let block = level.blocks().get_unchecked(level_block_index);
                    let in_block_index = self.0.as_ref().get_unchecked(I::VALUE).as_usize();
                    block.get_or_zero(in_block_index).as_usize()
                }
            }
        }
        let data_block_index = self.levels.fold(0, V(level_indices));
        self.data.blocks().get_unchecked(data_block_index)
    }

    type State = SparseBlockArrayState<Levels, DataLevel>;
}

pub struct SparseBlockArrayState<Levels, DataLevel>
where
    Levels: ArrayLevels
{
    /// [*const u8; Levels::LevelCount-1]
    /// 
    /// Level0 skipped - we can get it from self/this.
    level_block_ptrs: 
        <<Levels::LevelCount as ConstInteger>::Prev as ConstInteger>
        ::Array<*const u8>,
    phantom_data: PhantomData<SparseBlockArray<Levels, DataLevel>>
}

impl<Levels, DataLevel> SparseHierarchyState for SparseBlockArrayState<Levels, DataLevel>
where
    Levels: ArrayLevels,
    DataLevel: ILevel<Block: Clone>,
{
    type This = SparseBlockArray<Levels, DataLevel>;

    fn new(this: &Self::This) -> Self {
        Self{
            // TODO: point to 0,0,0... block?
            level_block_ptrs: Array::from_fn(|_|null()),
            phantom_data: Default::default(),
        }
    }

    unsafe fn select_level_bock<'a, L: ConstInteger>(
        &mut self, level_n: L, this: &'a Self::This, level_index: usize
    )
        -> (<Self::This as SparseHierarchy>::LevelMask<'a>, bool) 
    {
        if L::VALUE == 0{
            assert_eq!(level_index, 0);
            let mask_ptr = this.levels.visit(ConstInt::<0>, V);
            struct V;
            impl<M> Visitor<M> for V{
                type Out = NonNull<M>;
                fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L) -> Self::Out 
                where 
                    L: ILevel<Block: HiBlock<Mask=M>> 
                {
                    level.blocks()[0].mask().into()
                }
            }
            let mask = unsafe{mask_ptr.as_ref()};
            return (mask, mask.is_zero());
        }
        
        // We do not store the root level's block.
        let level_block_ptrs_index = level_n.prev().value();
        
        // 1. get level_block_index from prev level. 
        let level_block_index =
        if L::VALUE == 1{
            let level_block_index = this.levels.visit(ConstInt::<0>, V(level_index));
            struct V(usize);
            impl<M> Visitor<M> for V{
                type Out = usize;
                fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L) -> Self::Out 
                where 
                    L: ILevel<Block: HiBlock> 
                {
                    unsafe{
                        level.blocks()[0].get_or_zero(self.0).as_usize()
                    }
                }
            }
            level_block_index
        } else {
            struct V(*const u8, usize);
            impl<M> Visitor<M> for V{
                type Out = usize;
                fn visit<I: ConstInteger, L>(&mut self, _: I, _: &L) -> Self::Out 
                    where L: ILevel<Block: HiBlock>
                {
                    unsafe{
                        let block = self.0 as *const L::Block;
                        let block_index = (*block).get_or_zero(self.1).as_usize();
                        block_index
                    }
                    
                }
            }
            let prev_block = self.level_block_ptrs.as_mut()[level_block_ptrs_index-1];
            let visitor = V(prev_block, level_index);
            let level_block_index = this.levels.visit(level_n.prev(), visitor);
            level_block_index
        };
        
        // 2. get block mask from level.
        struct V(usize);
        impl<M> Visitor<M> for V{
            type Out = (*const u8, NonNull<M>);
            fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L) -> Self::Out 
                where L: ILevel, L::Block: HiBlock <Mask=M>
            {
                let level_block = unsafe{ level.blocks().get_unchecked(self.0) };
                (
                    level_block as *const _ as *const u8,
                    NonNull::from(level_block.mask())
                )
            }
        }
        let visitor = V(level_block_index); 
        let (level_block_ptr, mask_ptr) = this.levels.visit(level_n, visitor);
        self.level_block_ptrs.as_mut()[level_block_ptrs_index] = level_block_ptr; 

        (mask_ptr.as_ref(), !level_block_index.is_zero())
    }

    unsafe fn data_block<'a>(&self, this: &'a Self::This, level_index: usize)
        -> <Self::This as SparseHierarchy>::DataBlock<'a> 
    {
        let last_level_index = Levels::LevelCount::default().prev();
        
        let level_block_ptr = 
            if Levels::LevelCount::VALUE == 1{
                let level_block_ptr = this.levels.visit(ConstInt::<0>, V);
                struct V;
                impl<M> Visitor<M> for V{
                    type Out = *const u8;
                    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L) -> Self::Out 
                    where 
                        L: ILevel
                    {
                        &level.blocks()[0] as *const _ as *const u8
                    }
                }
                level_block_ptr
            } else {
                // We do not store the root level's block.
                let level_block_ptrs_index = last_level_index.prev();
                let level_block_ptr = self.level_block_ptrs.as_ref()[level_block_ptrs_index.value()];
                level_block_ptr
            };
        
        
        let visitor = V(level_block_ptr, level_index);
        let data_block_index = this.levels.visit(last_level_index, visitor);
        
        // TODO: get_block_index fn?
        struct V(*const u8, usize);
        impl<M> Visitor<M> for V{
            type Out = usize;
            fn visit<I: ConstInteger, L>(&mut self, _: I, _: &L) -> Self::Out 
            where 
                L: ILevel<Block: HiBlock> 
            {
                unsafe{
                    let block: *const L::Block = mem::transmute(self.0);
                    (*block).get_or_zero(self.1).as_usize()
                }
            }
        }
        
        this.data.blocks().get_unchecked(data_block_index)
    }
}


/*
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
}*/