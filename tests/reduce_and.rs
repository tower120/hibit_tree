use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{BitAnd, Mul};
use hi_sparse_array::{BitBlock, fold, IntoOwned, Op, /*reduce, Reduce, */SparseArray};
use hi_sparse_array::level_block::{Block, MaybeEmpty};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::const_utils::ConstTrue;
use hi_sparse_array::level::{IntrusiveListLevel, Level, SingleBlockLevel};

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
impl MaybeEmpty for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
}

type BlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>), /*IntrusiveList*/Level<DataBlock>>;


pub struct AndOp<M, LD>(PhantomData<(M, LD)>);
impl<M, LD> Default for AndOp<M, LD>{
    fn default() -> Self {
        Self(PhantomData)
    }
} 

impl<M, LD> Op for AndOp<M, LD>
where
    M: BitBlock + BitAnd<Output = M>, 
    LD: BitAnd<Output = LD> + MaybeEmpty
{
    const EXACT_HIERARCHY: bool = false;
    type SKIP_EMPTY_HIERARCHIES = ConstTrue;
    
    type LevelMask = M;
    fn lvl_op(&self, left: impl IntoOwned<M>, right: impl IntoOwned<M>) -> Self::LevelMask {
        left.into_owned() & right.into_owned()
    }

    type DataBlockL = LD;
    type DataBlockR = LD;
    type DataBlockO = LD;
    fn data_op(&self, left: impl Borrow<LD> + IntoOwned<LD>, right: impl Borrow<LD> + IntoOwned<LD>) -> Self::DataBlockO {
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
