use std::borrow::Borrow;
use std::collections::HashMap;
use std::fs::ReadDir;
use std::marker::PhantomData;
use std::ops::{BitAnd, Mul};
use criterion::{black_box, Criterion, criterion_group, criterion_main};
use hi_sparse_array::{apply, Apply, BitBlock, EmptyBitBlock, IntoOwned, Op, reduce, Reduce, SparseBlockArray};
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

type BlockArray = SparseBlockArray<Lvl0Block, Level<Lvl1Block>, BypassLevel/*Level<Lvl1Block>*/, Level<DataBlock>>;


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
    const SKIP_EMPTY_HIERARCHIES: bool = false;
     
    type Level0Mask = L0;
    #[inline]
    fn lvl0_op(&self, left: impl IntoOwned<L0>, right: impl IntoOwned<L0>) -> Self::Level0Mask {
        left.into_owned() & right.into_owned()
    }

    type Level1Mask = L1;
    #[inline]
    fn lvl1_op(&self, left: impl IntoOwned<L1>, right: impl IntoOwned<L1>) -> Self::Level1Mask {
        left.into_owned() & right.into_owned()
    }
    
    type Level2Mask = L2;
    #[inline]
    fn lvl2_op(&self, left: impl IntoOwned<L2>, right: impl IntoOwned<L2>) -> Self::Level2Mask {
        left.into_owned() & right.into_owned()
    }

    type DataBlock = LD;
    #[inline]
    fn data_op(&self, left: impl Borrow<LD> + IntoOwned<LD>, right: impl Borrow<LD> + IntoOwned<LD>) -> Self::DataBlock {
        left.into_owned() & right.into_owned()
    }
}


fn reduce_iter(list: &[BlockArray]) -> u64 {
    let list = list.iter();
    
    let and_op: AndOp<u64, u64, EmptyBitBlock, DataBlock> = AndOp(PhantomData);
    let reduce: Reduce<_, _, BlockArray> = reduce(and_op, list/*.iter().copied()*/);
    
    let mut s = 0;
    for (_, i) in CachingBlockIter::new(&reduce){
        s += i.0;
    }
    s
}

fn apply_iter(array1: &BlockArray, array2: &BlockArray) -> u64 {
    let and_op: AndOp<u64, u64, EmptyBitBlock, DataBlock> = AndOp(PhantomData);
    let reduce: Apply<_, _, _, BlockArray, BlockArray> = apply(and_op, array1, array2);
    
    let mut s = 0;
    for (_, i) in CachingBlockIter::new(&reduce){
        s += i.0;
    }
    s
}


pub fn bench_iter(c: &mut Criterion) {
    let mut block_array1 = BlockArray::default();
    let mut block_array2 = BlockArray::default();
    let mut block_array3 = BlockArray::default();
    let mut block_array4 = BlockArray::default();

    for i in 0..100{
        *block_array1.get_or_insert(i*20) = DataBlock(i as u64);
        *block_array2.get_or_insert(i*20) = DataBlock(i as u64);
        *block_array3.get_or_insert(i*20) = DataBlock(i as u64);
        *block_array4.get_or_insert(i*20) = DataBlock(i as u64);
    }
    let arrays = [block_array1, block_array2, block_array3];

    //c.bench_function("apply", |b| b.iter(|| apply_iter(black_box(&block_array1), black_box(&block_array2))));
    //c.bench_function("reduce", |b| b.iter(|| reduce_iter(black_box(&block_array1), black_box(&block_array2))));
    c.bench_function("reduce", |b| b.iter(|| reduce_iter(/*black_box(*/&arrays/*.iter()*//*)*/)));
    
}

criterion_group!(benches_iter, bench_iter);
criterion_main!(benches_iter);