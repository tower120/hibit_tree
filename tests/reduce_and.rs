use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{BitAnd, Mul};
use hi_sparse_array::{BitBlock, Empty, fold, SparseArray};
use hi_sparse_array::level_block::Block;
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::const_utils::ConstTrue;
use hi_sparse_array::level::{IntrusiveListLevel, SingleBlockLevel};
use hi_sparse_array::BinaryOp;
use hi_sparse_array::utils::Take;

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
impl BitAnd<&Self> for DataBlock{
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: &Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}
impl Empty for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
}

type BlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>), DataBlock>;

pub struct AndOp<M, LD>(PhantomData<(M, LD)>);
impl<M, LD> Default for AndOp<M, LD>{
    fn default() -> Self {
        Self(PhantomData)
    }
} 

impl<M, LD> BinaryOp for AndOp<M, LD>
where
    M: BitBlock + BitAnd<Output = M>, 
    LD: for<'a> BitAnd<&'a LD, Output = LD> + Empty
{
    const EXACT_HIERARCHY: bool = false;
    type SKIP_EMPTY_HIERARCHIES = ConstTrue;
    
    type LevelMask = M;
    fn lvl_op(&self, left: impl Take<M>, right: impl Take<M>) -> Self::LevelMask {
        left.take_or_clone() & right.take_or_clone()
    }

    type Left  = LD;
    type Right = LD;
    type Out   = LD;
    fn data_op(&self, acc: impl Take<LD>, right: impl Borrow<LD>) -> Self::Out {
        acc.take() & right.borrow()
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
        *block_array1.get_mut(i) = DataBlock(i as u64);
        *block_array2.get_mut(i) = DataBlock(i as u64);
    }

    array_iter(&block_array1, &block_array2);
}
