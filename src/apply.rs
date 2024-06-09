use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::SparseHierarchy;
use crate::const_utils::const_int::ConstInteger;
use crate::const_utils::const_array::ConstArray;
use crate::op::BinaryOp;
use crate::sparse_hierarchy::SparseHierarchyState;
use crate::utils::Borrowable;

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

    Op: BinaryOp<
        LevelMask = <B1::Borrowed as SparseHierarchy>::LevelMaskType,
        Left  = <B1::Borrowed as SparseHierarchy>::DataType,
        Right = <B2::Borrowed as SparseHierarchy>::DataType,
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

    type DataType = Op::Out;
    type Data<'a> = Op::Out where Self:'a;
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

    Op: BinaryOp<
        LevelMask = <B1::Borrowed as SparseHierarchy>::LevelMaskType,
        Left  = <B1::Borrowed as SparseHierarchy>::DataType,
        Right = <B2::Borrowed as SparseHierarchy>::DataType,
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
        -> <Self::This as SparseHierarchy>::LevelMask<'a> 
    {
        let mask1 = self.s1.select_level_bock(
            this.s1.borrow(), level_n, level_index
        );
        let mask2 = self.s2.select_level_bock(
            this.s2.borrow(), level_n, level_index
        );
        
        let mask = this.op.lvl_op(mask1, mask2);
        mask
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