use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{BitAnd, Mul};
use hi_sparse_array::{BitBlock, EmptyBitBlock, fold, IntoOwned, Op, /*reduce, Reduce, */SparseArray};
use hi_sparse_array::level_block::{LevelBlock, Block, SmallBlock, ClusterBlock};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::level::{BypassLevel, Level};

type Lvl0Block = Block<u64, [u8;64]>;
type Lvl1Block = Block<u64, [u16;64]>;

#[derive(Clone)]
struct DataBlock(u64);
impl BitAnd for DataBlock{
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl LevelBlock for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }

    fn as_u64_mut(&mut self) -> &mut u64 {
        &mut self.0
    }

    fn restore_empty_u64(&mut self) {
        self.0 = 0;
    }
}

type BlockArray = SparseArray<Lvl0Block, Level<Lvl1Block>, /*BypassLevel*/Level<Lvl1Block>, Level<DataBlock>>;


pub struct AndOp<L0, L1, L2, LD>(PhantomData<(L0, L1, L2, LD)>);
impl<L0, L1, L2, LD> Default for AndOp<L0, L1, L2, LD>{
    fn default() -> Self {
        Self(PhantomData)
    }
} 

impl<L0, L1, L2, LD> Op for AndOp<L0, L1, L2, LD>
where
    L0: BitBlock + BitAnd<Output = L0>, 
    L1: BitBlock + BitAnd<Output = L1>, 
    L2: BitBlock + BitAnd<Output = L2>, 
    LD: BitAnd<Output = LD>
{
    const EXACT_HIERARCHY: bool = false;
    const SKIP_EMPTY_HIERARCHIES: bool = true;
    
    type Level0Mask = L0;
    fn lvl0_op(&self, left: impl IntoOwned<L0>, right: impl IntoOwned<L0>) -> Self::Level0Mask {
        left.into_owned() & right.into_owned()
    }

    type Level1Mask = L1;
    fn lvl1_op(&self, left: impl IntoOwned<L1>, right: impl IntoOwned<L1>) -> Self::Level1Mask {
        left.into_owned() & right.into_owned()
    }
    
    type Level2Mask = L2;
    fn lvl2_op(&self, left: impl IntoOwned<L2>, right: impl IntoOwned<L2>) -> Self::Level2Mask {
        left.into_owned() & right.into_owned()
    }

    type DataBlock = LD;
    fn data_op(&self, left: impl Borrow<LD> + IntoOwned<LD>, right: impl Borrow<LD> + IntoOwned<LD>) -> Self::DataBlock {
        left.into_owned() & right.into_owned()
    }
}


fn array_iter(array1: &BlockArray, array2: &BlockArray) -> u64 {
    //let list = [array1, array2];
    let list = [array2];
    
    let and_op = AndOp(PhantomData);
    //let reduce: Reduce<_, _, BlockArray> = reduce(and_op, list.iter().map(|a|*a));
    let reduce = fold(and_op, array1, list.iter().map(|a|*a));
    
    let mut s = 0;
    for (_, i) in CachingBlockIter::new(&reduce){
        s += i.0;
    }
    s
}

// Same as bench/reduce_and, but we can debug it here
#[test]
pub fn bench_iter() {
    let mut block_array1 = BlockArray::default();
    let mut block_array2 = BlockArray::default();

    for i in 0..3000{
        *block_array1.get_or_insert(i) = DataBlock(i as u64);
        *block_array2.get_or_insert(i) = DataBlock(i as u64);
    }

    array_iter(&block_array1, &block_array2);
}
