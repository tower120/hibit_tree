use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::bit_block::BitBlock;
use crate::{SparseHierarchy, IntoOwned, Borrowable};
use crate::const_utils::const_bool::ConstBool;
use crate::const_utils::const_int::ConstInteger;
use crate::const_utils::const_array::ConstArray;
use crate::level_block::MaybeEmpty;
use crate::sparse_hierarchy::{DefaultState, SparseHierarchyState};

// TODO: move out from apply.
// We need more advanced GAT in Rust to make `DataBlock<'a>` work here 
// in a meaningful way.
// For now, should be good enough as-is for Apply.
pub trait Op {
    const EXACT_HIERARCHY: bool;
    
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
    type SKIP_EMPTY_HIERARCHIES: ConstBool;
    
    type LevelMask: BitBlock;
    fn lvl_op(&self,
        left : impl Borrow<Self::LevelMask> + IntoOwned<Self::LevelMask>,
        right: impl Borrow<Self::LevelMask> + IntoOwned<Self::LevelMask>
    ) -> Self::LevelMask;
    
    // TODO: rename
    type DataBlockL;
    type DataBlockR;
    type DataBlockO: MaybeEmpty;
    fn data_op(&self,
        left : impl Borrow<Self::DataBlockL> + IntoOwned<Self::DataBlockL>,
        right: impl Borrow<Self::DataBlockR> + IntoOwned<Self::DataBlockR>
    ) -> Self::DataBlockO;
}

pub struct Apply<Op, B1, B2>{
    pub(crate) op: Op,
    pub(crate) s1: B1,
    pub(crate) s2: B2,
}

impl<Op, B1, B2> SparseHierarchy for Apply<Op, B1, B2>
where
    B1: Borrowable<Borrowed: SparseHierarchy>,
    B2: Borrowable<
        Borrowed: SparseHierarchy<
            LevelCount    = <B1::Borrowed as SparseHierarchy>::LevelCount,
            LevelMaskType = <B1::Borrowed as SparseHierarchy>::LevelMaskType,
        >
    >,

    Op: self::Op<
        LevelMask  = <B1::Borrowed as SparseHierarchy>::LevelMaskType,
        DataBlockL = <B1::Borrowed as SparseHierarchy>::DataType,
        DataBlockR = <B2::Borrowed as SparseHierarchy>::DataType,
    >
{
    const EXACT_HIERARCHY: bool = Op::EXACT_HIERARCHY;
    type LevelCount = <B1::Borrowed as SparseHierarchy>::LevelCount;

    type LevelMaskType = <B1::Borrowed as SparseHierarchy>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self:'a;
    #[inline]
    unsafe fn level_mask<I>(&self, level_indices: I)
        -> Self::LevelMask<'_>
    where 
        I: ConstArray<Item=usize> + Copy
    {
        let s1 = self.s1.borrow(); 
        let s2 = self.s2.borrow();
        self.op.lvl_op(
            s1.level_mask(level_indices),
            s2.level_mask(level_indices)
        )
    }

    type DataType = Op::DataBlockO;
    type Data<'a> = Op::DataBlockO where Self:'a;
    #[inline]
    unsafe fn data_block<I>(&self, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        let s1 = self.s1.borrow(); 
        let s2 = self.s2.borrow();
        self.op.data_op(
            s1.data_block(level_indices),
            s2.data_block(level_indices)
        )
    }

    #[inline]
    fn empty_data(&self) -> Self::Data<'_> {
        <Op::DataBlockO as MaybeEmpty>::empty()
    }

    type State = ApplyState<Op, B1, B2>;
}

pub struct ApplyState<Op, B1, B2>
where
    B1: Borrowable<Borrowed: SparseHierarchy>,
    B2: Borrowable<Borrowed: SparseHierarchy>,
{
    s1: <B1::Borrowed as SparseHierarchy>::State, 
    s2: <B2::Borrowed as SparseHierarchy>::State,
    phantom_data: PhantomData<Apply<Op, B1, B2>>
}

impl<Op, B1, B2> SparseHierarchyState for ApplyState<Op, B1, B2>
where
    B1: Borrowable<Borrowed: SparseHierarchy>,
    B2: Borrowable<
        Borrowed: SparseHierarchy<
            LevelCount    = <B1::Borrowed as SparseHierarchy>::LevelCount,
            LevelMaskType = <B1::Borrowed as SparseHierarchy>::LevelMaskType,
        >
    >,

    Op: self::Op<
        LevelMask  = <B1::Borrowed as SparseHierarchy>::LevelMaskType,
        DataBlockL = <B1::Borrowed as SparseHierarchy>::DataType,
        DataBlockR = <B2::Borrowed as SparseHierarchy>::DataType,
    >
{
    type This = Apply<Op, B1, B2>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        Self{
            s1: SparseHierarchyState::new(this.s1.borrow()), 
            s2: SparseHierarchyState::new(this.s2.borrow()),
            phantom_data: PhantomData
        }
    }
    
    #[inline]
    unsafe fn select_level_bock<'a, N: ConstInteger>(&mut self, this: &'a Self::This, level_n: N, level_index: usize) 
        -> (<Self::This as SparseHierarchy>::LevelMask<'a>, bool) 
    {
        let (mask1, _) = self.s1.select_level_bock(
            this.s1.borrow(), level_n, level_index
        );
        let (mask2, _) = self.s2.select_level_bock(
            this.s2.borrow(), level_n, level_index
        );
        
        let mask = this.op.lvl_op(mask1, mask2);
        let is_empty = mask.is_zero();
        (mask, !is_empty)
    }

    #[inline]
    unsafe fn data_block<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::Data<'a> 
    {
        let m0 = self.s1.data_block(
            this.s1.borrow(), level_index
        );
        let m1 = self.s2.data_block(
            this.s2.borrow(), level_index
        );
        this.op.data_op(m0, m1)        
    }
}

impl<Op, B1, B2> Borrowable for Apply<Op, B1, B2>{ 
    type Borrowed = Apply<Op, B1, B2>; 
}
impl<Op, B1, B2> Borrowable for &Apply<Op, B1, B2>{ 
    type Borrowed = Apply<Op, B1, B2>; 
}