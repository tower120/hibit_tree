use std::marker::PhantomData;
use std::ptr::{NonNull, null};
use crate::bit_block::BitBlock;
use crate::level_block::HiBlock;
use crate::level::{ILevel, Level};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::const_int::{const_for, ConstInt, ConstInteger, ConstIntVisitor};
use crate::primitive::Primitive;
use crate::primitive_array::{Array, ConstArray, ConstArrayType};

// TODO: make public
// Compile-time loop inside. Ends up with N (AND + SHR)s.
#[inline]
pub(crate) fn level_indices<LevelMask, LevelsCount>(index: usize)
     -> ConstArrayType<usize, LevelsCount>
where
    LevelMask: BitBlock,
    LevelsCount: ConstInteger,
{
    // TODO: need uninit?
    let mut level_indices = ConstArrayType::<usize, LevelsCount>::from_fn(|_|0);
    
    let mut level_remainder = index;
    let level_count = LevelsCount::VALUE;
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

#[test]
fn test_level_indices_new(){
    {
        let indices = level_indices::<u64, ConstInt<2>>(65);
        assert_eq!(indices, [1, 1]);
    }
    {
        let lvl0 = 262_144; // Total max capacity
        let lvl1 = 4096;
        let lvl2 = 64;
        let indices = level_indices::<u64, ConstInt<3>>(lvl1*2 + lvl2*3 + 4);
        assert_eq!(indices, [2, 3, 4]);
    }
    {
        let indices = level_indices::<u64, ConstInt<3>>(32);
        assert_eq!(indices, [0, 0, 32]);
    }
    {
        let indices = level_indices::<u64, ConstInt<2>>(32);
        assert_eq!(indices, [0, 32]);
    }    
    {
        let indices = level_indices::<u64, ConstInt<1>>(32);
        assert_eq!(indices, [32]);
    }
}


// TODO: Can be removed
pub trait HiLevel: ILevel<Block: HiBlock>{}
impl<T: ILevel<Block: HiBlock>> HiLevel for T{}


pub trait Visitor<Mask> {
    type Out;
    fn visit<I: ConstInteger, L>(self, i: I, level: &L) -> Self::Out
    where
        L: ILevel,
        L::Block: HiBlock<Mask = Mask>;
}

pub trait MutVisitor<Mask> {
    type Out;
    fn visit<I: ConstInteger, L>(self, i: I, level: &mut L) -> Self::Out
    where
        L: ILevel,
        L::Block: HiBlock<Mask=Mask>;
}

pub trait FoldVisitor<Mask> {
    type Acc;
    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L, acc: Self::Acc) -> Self::Acc
    where
        L: ILevel,
        L::Block: HiBlock<Mask=Mask>;
}

pub trait FoldMutVisitor<Mask> {
    type Acc;
    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &mut L, acc: Self::Acc) -> Self::Acc
    where
        L: ILevel,
        L::Block: HiBlock<Mask=Mask>;
}


pub trait SparseArrayLevels: Default {
    type LevelCount: ConstInteger;
    type Mask: BitBlock;
    
    fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, visitor: V) -> V::Out;
    fn visit_mut<I: ConstInteger, V: MutVisitor<Self::Mask>>(&mut self, i: I, visitor: V) -> V::Out;
    
    // TODO: consider "visit" accept usize, and implement fold here.
    fn fold<Acc>(&self, acc: Acc, visitor: impl FoldVisitor<Self::Mask, Acc=Acc>) -> Acc;
    fn fold_mut<Acc>(&mut self, acc: Acc, visitor: impl FoldMutVisitor<Self::Mask, Acc=Acc>) -> Acc;
}

macro_rules! sparse_array_levels_impl {
    ($n:literal: [$($i:tt,)+]; $first_t:tt, $($t:tt,)* ) => {
        impl<$first_t, $($t,)*> SparseArrayLevels for ($first_t, $($t,)*)
        where
            $first_t: ILevel,
            $first_t::Block: HiBlock,
            $(
                $t: ILevel,
                $t::Block: HiBlock<Mask = <$first_t::Block as HiBlock>::Mask>,
            )*
        {
            type LevelCount = ConstInt<$n>;       
            type Mask = <$first_t::Block as HiBlock>::Mask;
    
            #[inline]
            fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, mut visitor: V) -> V::Out {
                match i.value() {
                    $(
                        $i => visitor.visit(i, &self.$i),
                    )+
                    _ => unreachable!()
                }
            }
            
            #[inline]
            fn visit_mut<I: ConstInteger, V: MutVisitor<Self::Mask>>(&mut self, i: I, mut visitor: V) -> V::Out {
                match i.value() {
                    $(
                        $i => visitor.visit(i, &mut self.$i),
                    )+
                    _ => unreachable!()
                }
            }   
            
            fn fold<Acc>(&self, mut acc: Acc, mut visitor: impl FoldVisitor<Self::Mask, Acc = Acc>) -> Acc {
                $(
                    acc = visitor.visit(ConstInt::<$i>::DEFAULT, &self.$i, acc);
                )+
                acc
            }

            fn fold_mut<Acc>(&mut self, mut acc: Acc, mut visitor: impl FoldMutVisitor<Self::Mask, Acc = Acc>) -> Acc {
                $(
                    acc = visitor.visit(ConstInt::<$i>::DEFAULT, &mut self.$i, acc);
                )+
                acc
            }
            
        }
    };
}
sparse_array_levels_impl!(1: [0,]; L0,);
sparse_array_levels_impl!(2: [0,1,]; L0,L1,);
sparse_array_levels_impl!(3: [0,1,2,]; L0,L1,L2,);
sparse_array_levels_impl!(4: [0,1,2,3,]; L0,L1,L2,L3,);

pub struct SparseArray<Levels, DataLevel> {
    levels: Levels,
    data  : DataLevel,
}
impl<Levels, DataLevel> Default for
    SparseArray<Levels, DataLevel>
where
    Levels: SparseArrayLevels,
    DataLevel: ILevel
{
    #[inline]
    fn default() -> Self {        
        Self{
            levels: Levels::default(),
            data  : Default::default(),
        }
    }
}

impl<Levels, DataLevel> SparseArray<Levels, DataLevel>
where
    Levels: SparseArrayLevels,
    DataLevel: ILevel
{
    #[inline]
    fn level_indices(index: usize) -> (usize/*level0*/, usize/*level1*/, usize/*level2*/) {
        todo!()
        //level_indices::<Level1, Level2>(index)
    }
    
    unsafe fn get_block_ptr(&self, level_n: impl ConstInteger, level_index: usize) -> *const u8{
        struct V(usize);
        impl<M> Visitor<M> for V{
            type Out = *const u8;
            fn visit<I: ConstInteger, L>(self, _: I, level: &L) -> Self::Out 
            where 
                L: ILevel 
            {
                unsafe {
                    level.blocks().get_unchecked(self.0) as *const _ as *const u8
                }
            }
        }
        self.levels.visit(level_n, V(level_index))
    }
    
    unsafe fn get_block_mask(
        &self, 
        level_n: impl ConstInteger, 
        level_block_ptr: *const u8,
    ) -> &Levels::Mask {
        struct V(*const u8);
        impl<M> Visitor<M> for V{
            type Out = NonNull<M>;
            fn visit<I: ConstInteger, L>(self, _: I, _: &L) -> Self::Out 
            where 
                L: ILevel<Block: HiBlock<Mask=M>> 
            {
                unsafe{
                    let block = self.0 as *const L::Block;
                    NonNull::from((*block).mask())
                }
            }
        }
        self.levels.visit(level_n, V(level_block_ptr)).as_ref()        
    }    

    unsafe fn get_block_index(
        &self, 
        level_n: impl ConstInteger, 
        level_block_ptr: *const u8, 
        index: usize
    ) -> usize {
        struct V(*const u8, usize);
        impl<M> Visitor<M> for V{
            type Out = usize;
            fn visit<I: ConstInteger, L>(self, _: I, _: &L) -> Self::Out 
            where 
                L: ILevel<Block: HiBlock> 
            {
                unsafe{
                    let block = self.0 as *const L::Block;
                    (*block).get_or_zero(self.1).as_usize()
                }
            }
        }
        self.levels.visit(level_n, V(level_block_ptr, index))
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

        let i = level_indices::<Levels::Mask, Levels::LevelCount>(index);
        
        let this = NonNull::new(self).unwrap();
        let data_block_index = self.levels.fold_mut(0, V{this, level_indices: i});
        struct V<Levels: SparseArrayLevels, DataLevel, LevelIndices> {
            this: NonNull<SparseArray<Levels, DataLevel>>, 
            level_indices: LevelIndices
        }
        impl<Levels, DataLevel, LevelIndices, M> FoldMutVisitor<M> for V<Levels, DataLevel, LevelIndices>
        where
            Levels: SparseArrayLevels, 
            DataLevel: ILevel,
            LevelIndices: Array<Item=usize>
        {
            type Acc = usize;
            fn visit<I: ConstInteger, L: ILevel>(&mut self, i: I, level: &mut L, level_index: usize) -> usize
            where
                L::Block: HiBlock
            {
            unsafe{
                let block = level.blocks_mut().get_unchecked_mut(level_index);
                block.get_or_insert(self.level_indices.as_ref()[I::VALUE], ||{
                    let block_index = 
                        if I::VALUE == Levels::LevelCount::VALUE - 1 {
                            self.this.as_mut().data.insert_empty_block()
                        } else {
                            struct Insert;
                            impl<M> MutVisitor<M> for Insert {
                                type Out = usize;
                                fn visit<I:ConstInteger, L: ILevel>(self, i: I, level: &mut L) -> usize {
                                    level.insert_empty_block()
                                }
                            }
                            self.this.as_mut().levels.visit_mut(i.inc(), Insert)
                        };
                    Primitive::from_usize(block_index)
                }).as_usize()
            }
            }
        }

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



impl<Levels, DataLevel> SparseHierarchy for SparseArray<Levels, DataLevel>
where
    Levels: SparseArrayLevels,
    DataLevel: ILevel,
    DataLevel::Block: Clone
{
    const EXACT_HIERARCHY: bool = true;
    
    type LevelCount = Levels::LevelCount;
    type LevelMaskType = Levels::Mask;
    type LevelMask<'a> where Self: 'a = &'a Self::LevelMaskType;

    /*fn level_mask<const N: usize>(&self, level_indices: [usize; N]) -> Self::LevelMask<'_> {
        todo!()
    }*/
    
    type DataBlockType = DataLevel::Block;
    type DataBlock<'a> where Self: 'a = &'a Self::DataBlockType;
    
    #[inline]
    unsafe fn data_block<I: ConstArray<Item=usize, Cap=Self::LevelCount>>(&self, level_indices: I) -> Self::DataBlock<'_> {
        let data_block_index = self.levels.fold(0, V(level_indices));
        struct V<LevelIndices>(LevelIndices);
        impl<LevelIndices: ConstArray<Item=usize>, M> FoldVisitor<M> for V<LevelIndices>{
            type Acc = usize;
            fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L, level_block_index: usize) 
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
        self.data.blocks().get_unchecked(data_block_index)
    }

    type State = SparseBlockArrayState<Levels, DataLevel>;
}

pub struct SparseBlockArrayState<Levels, DataLevel>
where
    Levels: SparseArrayLevels
{
    /// [*const u8; Levels::LevelCount-1]
    /// 
    /// Level0 skipped - we can get it from self/this.
    level_block_ptrs: ConstArrayType<
        *const u8, 
        <Levels::LevelCount as ConstInteger>::Dec
    >,
    phantom_data: PhantomData<SparseArray<Levels, DataLevel>>
}

impl<Levels, DataLevel> SparseHierarchyState for SparseBlockArrayState<Levels, DataLevel>
where
    Levels: SparseArrayLevels,
    DataLevel: ILevel<Block: Clone>,
{
    type This = SparseArray<Levels, DataLevel>;

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
            let block_ptr = this.get_block_ptr(level_n, 0);
            let mask = this.get_block_mask(level_n, block_ptr);
            return (mask, mask.is_zero());
        }
        
        // We do not store the root level's block.
        let level_block_ptrs_index = level_n.dec().value();
        
        // 1. get level_block_index from prev level. 
        let level_block_index ={
            let prev_level_block_ptr = 
                if L::VALUE == 1{
                    // get directly from root
                    this.get_block_ptr(ConstInt::<0>, 0)
                } else {
                    self.level_block_ptrs.as_mut()[level_block_ptrs_index-1]
                };
            
            let level_block_index = this.get_block_index(level_n.dec(), prev_level_block_ptr, level_index);
            level_block_index
        };
        
        // 2. get block mask from level.
        let block_ptr = this.get_block_ptr(level_n, level_block_index);
        let mask = this.get_block_mask(level_n, block_ptr);
        self.level_block_ptrs.as_mut()[level_block_ptrs_index] = block_ptr;

        (mask, !level_block_index.is_zero())
    }

    unsafe fn data_block<'a>(&self, this: &'a Self::This, level_index: usize)
        -> <Self::This as SparseHierarchy>::DataBlock<'a> 
    {
        let last_level_index = Levels::LevelCount::default().dec();
        
        let level_block_ptr = 
            if Levels::LevelCount::VALUE == 1{
                this.get_block_ptr(ConstInt::<0>, 0)
            } else {
                // We do not store the root level's block.
                let level_block_ptrs_index = last_level_index.dec();
                let level_block_ptr = self.level_block_ptrs.as_ref()[level_block_ptrs_index.value()];
                level_block_ptr
            };
        

        let data_block_index = this.get_block_index(last_level_index, level_block_ptr, level_index);
        this.data.blocks().get_unchecked(data_block_index)
    }
}