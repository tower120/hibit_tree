use std::borrow::Borrow;
use std::marker::PhantomData;
use std::mem;
use std::mem::MaybeUninit;
use std::ptr::addr_of_mut;
use crate::bit_block::BitBlock;
use crate::{LevelMasks, IntoOwned};
use crate::level_masks::LevelMasksIter;

// TODO: unused now
/// &mut MaybeUninit<(T0, T1)> = (&mut MaybeUninit<T0>, &mut MaybeUninit<T1>)
#[inline] 
fn uninit_as_mut_pair<T0, T1>(pair: &mut MaybeUninit<(T0, T1)>)
    -> (&mut MaybeUninit<T0>, &mut MaybeUninit<T1>)
{
    unsafe{
        let ptr  = pair.as_mut_ptr();
        let ptr0 = addr_of_mut!((*ptr).0);
        let ptr1 = addr_of_mut!((*ptr).1);
        (
            &mut* mem::transmute::<_, *mut MaybeUninit<T0>>(ptr0),
            &mut* mem::transmute::<_, *mut MaybeUninit<T1>>(ptr1)
        )
    }
}

// TODO: move out from apply.
// We need more advanced GAT in Rust to make `DataBlock<'a>` work here 
// in a meaningful way.
// For now, should be good enough as-is for Apply.
pub trait Op {
    /// Check and skip empty hierarchies? Any value is safe. Use `false` as default.
    /// 
    /// This incurs some performance overhead, but can greatly reduce
    /// algorithmic complexity of some [Reduce] operations.
    /// 
    /// # In-depth
    /// 
    /// For example, merge operation will OR level1 mask, and some of the
    /// raised bits of resulting bitmask will not correspond to the raised bits
    /// of each source bitmask:
    /// ```text
    /// L 01100001      
    /// R 00101000
    /// ----------
    /// O 01101001    
    /// ```
    /// R's second bit is 0, but resulting bitmask's bit is 1.
    /// This means that in level2 R's second block's mask will be loaded, 
    /// thou its empty and can be skipped.
    /// 
    /// [Reduce] cache hierarchy blocks for faster traverse. And when this flag
    /// is raised - it checks and does not add empty blocks to the cache list. 
    ///
    /// Notice though, that such thing cannot happen with intersection. 
    /// So trying to apply such optimization there, would be a waste of resources.   
    /// 
    const SKIP_EMPTY_HIERARCHIES: bool;
    
    type Level0Mask: BitBlock;
    fn lvl0_op(&self,
        left : impl Borrow<Self::Level0Mask> + IntoOwned<Self::Level0Mask>,
        right: impl Borrow<Self::Level0Mask> + IntoOwned<Self::Level0Mask>
    ) -> Self::Level0Mask;
    
    type Level1Mask: BitBlock;
    fn lvl1_op(&self,
        left : impl Borrow<Self::Level1Mask> + IntoOwned<Self::Level1Mask>,
        right: impl Borrow<Self::Level1Mask> + IntoOwned<Self::Level1Mask>
    ) -> Self::Level1Mask;
    
    type Level2Mask: BitBlock;
    fn lvl2_op(&self,
        left : impl Borrow<Self::Level2Mask> + IntoOwned<Self::Level2Mask>,
        right: impl Borrow<Self::Level2Mask> + IntoOwned<Self::Level2Mask>
    ) -> Self::Level2Mask;
    
    type DataBlock;
    fn data_op(&self,
        left : impl Borrow<Self::DataBlock> + IntoOwned<Self::DataBlock>,
        right: impl Borrow<Self::DataBlock> + IntoOwned<Self::DataBlock>
    ) -> Self::DataBlock;
}

pub struct Apply<Op, B1, B2, T1, T2>{
    pub(crate) op: Op,
    pub(crate) s1: B1,
    pub(crate) s2: B2,
    pub(crate) phantom: PhantomData<(T1, T2)>,
}

impl<Op, B1, B2, T1, T2> LevelMasks for Apply<Op, B1, B2, T1, T2>
where
    B1: Borrow<T1>,
    B2: Borrow<T2>,

    T1: LevelMasks,

    T2: LevelMasks<
        Level0MaskType = T1::Level0MaskType,
        Level1MaskType = T1::Level1MaskType,
        Level2MaskType = T1::Level2MaskType,
        DataBlockType  = T1::DataBlockType,
    >,

    Op: self::Op<
        Level0Mask = T1::Level0MaskType,
        Level1Mask = T1::Level1MaskType,
        Level2Mask = T1::Level2MaskType,
        DataBlock  = T1::DataBlockType,
    >
{
    type Level0MaskType = T1::Level0MaskType;
    type Level0Mask<'a> = Self::Level0MaskType where Self:'a;
    #[inline]
    fn level0_mask(&self) -> Self::Level0Mask<'_> {
        let s1 = self.s1.borrow(); 
        let s2 = self.s2.borrow();
        self.op.lvl0_op(s1.level0_mask(), s2.level0_mask())
    }

    type Level1MaskType = T1::Level1MaskType;
    type Level1Mask<'a> = Self::Level1MaskType where Self:'a;
    #[inline]
    unsafe fn level1_mask(&self, level0_index: usize) -> Self::Level1Mask<'_> {
        let s1 = self.s1.borrow(); 
        let s2 = self.s2.borrow();
        self.op.lvl1_op(
            s1.level1_mask(level0_index),
            s2.level1_mask(level0_index)
        )
    }
    
    type Level2MaskType = T1::Level2MaskType;
    type Level2Mask<'a> = Self::Level2MaskType where Self:'a;
    #[inline]
    unsafe fn level2_mask(&self, level0_index: usize, level1_index: usize) -> Self::Level2Mask<'_> {
        let s1 = self.s1.borrow(); 
        let s2 = self.s2.borrow();
        self.op.lvl2_op(
            s1.level2_mask(level0_index, level1_index),
            s2.level2_mask(level0_index, level1_index)
        )
    }

    type DataBlockType = Op::DataBlock;
    type DataBlock<'a> = Op::DataBlock where Self:'a;
    #[inline]
    unsafe fn data_block(&self, level0_index: usize, level1_index: usize, level2_index: usize) -> Self::DataBlock<'_> {
        let s1 = self.s1.borrow(); 
        let s2 = self.s2.borrow();
        self.op.data_op(
            s1.data_block(level0_index, level1_index, level2_index),
            s2.data_block(level0_index, level1_index, level2_index)
        )
    }
}

pub struct ApplyIterState<T1, T2>
where
    T1: LevelMasksIter,
    T2: LevelMasksIter,
{
    s1: <T1 as LevelMasksIter>::IterState, 
    s2: <T2 as LevelMasksIter>::IterState,
}

impl<Op, B1, B2, T1, T2> LevelMasksIter for Apply<Op, B1, B2, T1, T2>
where
    B1: Borrow<T1>,
    B2: Borrow<T2>,

    T1: LevelMasksIter,

    T2: LevelMasksIter<
        Level0MaskType = T1::Level0MaskType,
        Level1MaskType = T1::Level1MaskType,
        Level2MaskType = T1::Level2MaskType,
        DataBlockType  = T1::DataBlockType,
    >,

    Op: self::Op<
        Level0Mask = T1::Level0MaskType,
        Level1Mask = T1::Level1MaskType,
        Level2Mask = T1::Level2MaskType,
        DataBlock  = T1::DataBlockType,
    >
{
    type IterState = ApplyIterState<T1, T2>;
    
    #[inline]
    fn make_state(&self) -> Self::IterState {
        ApplyIterState{
            s1: self.s1.borrow().make_state(), 
            s2: self.s2.borrow().make_state(),
        }
    }
    
    #[inline]
    unsafe fn init_level1_block_meta(
        &self,
        state: &mut Self::IterState,
        level0_index: usize
    ) -> (Self::Level1Mask<'_>, bool) {
        let (mask1, _) = self.s1.borrow().init_level1_block_meta(
            &mut state.s1, level0_index
        );
        let (mask2, _) = self.s2.borrow().init_level1_block_meta(
            &mut state.s2, level0_index
        );
        
        let mask = self.op.lvl1_op(mask1, mask2);
        let is_empty = mask.is_zero();
        (mask, !is_empty)
    }
    
    #[inline]
    unsafe fn init_level2_block_meta(
        &self,
        state: &mut Self::IterState,
        level1_index: usize
    ) -> (Self::Level2Mask<'_>, bool) {
        let (mask1, _) = self.s1.borrow().init_level2_block_meta(
            &mut state.s1, level1_index
        );
        let (mask2, _) = self.s2.borrow().init_level2_block_meta(
            &mut state.s2, level1_index
        );
        
        let mask = self.op.lvl2_op(mask1, mask2);
        let is_empty = mask.is_zero();
        (mask, !is_empty)
    }

    #[inline]
    unsafe fn data_block_from_meta(
        &self,
        state: &Self::IterState,
        level_index: usize
    ) -> Self::DataBlock<'_> {
        let m0 = self.s1.borrow().data_block_from_meta(
            &state.s1, level_index
        );
        let m1 = self.s2.borrow().data_block_from_meta(
            &state.s2, level_index
        ); 
        self.op.data_op(m0, m1)
    }
}

// TODO: other array read operations