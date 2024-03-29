use std::collections::HashMap;
use criterion::{black_box, Criterion, criterion_group, criterion_main};
use hi_sparse_array::SparseBlockArray;
use hi_sparse_array::block::{Block, FixedHiBlock};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::simple_iter::SimpleBlockIter;
use hi_sparse_array::small_block::CompactBlock;

type Lvl0Block = FixedHiBlock<u64, [u8;64]>;
type Lvl1Block = FixedHiBlock<u64, [u16;64]>;
type CompactLvl1Block = CompactBlock<u64, [u8;1], [u16;64], [u16;7]>;

#[derive(Clone)]
struct DataBlock(u64);
impl Block for DataBlock{
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

type BlockArray = SparseBlockArray<Lvl0Block, Lvl1Block, DataBlock>;
type SmallBlockArray = SparseBlockArray<Lvl0Block, CompactLvl1Block, DataBlock>;

fn small_array_iter(array: &SmallBlockArray) -> u64 {
    let mut s = 0;
    for (_, i) in CachingBlockIter::new(array){
        s += i.0;
    }
    s
}

fn array_iter(array: &BlockArray) -> u64 {
    let mut s = 0;
    for (_, i) in CachingBlockIter::new(array){
        s += i.0;
    }
    s
}

fn vec_iter(array: &Vec<DataBlock>) -> u64 {
    let mut s = 0;
    for i in array{
        s += i.0;
    }
    s
}

fn hashmap_iter(array: &HashMap<u64, DataBlock>) -> u64 {
    let mut s = 0;
    for (_, i) in array.iter(){
        s += i.0;
    }
    s
}

pub fn bench_iter(c: &mut Criterion) {
    let mut block_array = BlockArray::default();
    let mut small_block_array = SmallBlockArray::default();
    let mut vec = Vec::default();
    let mut hashmap = HashMap::default();
    for i in 0..3000{
        *block_array.get_or_insert(i) = DataBlock(i as u64);
        *small_block_array.get_or_insert(i) = DataBlock(i as u64);
        vec.push(DataBlock(i as u64));
        hashmap.insert(i as u64, DataBlock(i as u64));
    }

    c.bench_function("hashmap", |b| b.iter(|| hashmap_iter(black_box(&hashmap))));
    c.bench_function("small block array", |b| b.iter(|| small_array_iter(black_box(&small_block_array))));
    c.bench_function("block array", |b| b.iter(|| array_iter(black_box(&block_array))));
    c.bench_function("vec", |b| b.iter(|| vec_iter(black_box(&vec))));
    
}

criterion_group!(benches_iter, bench_iter);
criterion_main!(benches_iter);