use criterion::{black_box, Criterion, criterion_group, criterion_main};
use hi_sparse_array::SparseBlockArray;
use hi_sparse_array::block::{Block, FixedHiBlock};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::simple_iter::SimpleBlockIter;

type Lvl0Block = FixedHiBlock<u64, [u8;64]>;
type Lvl1Block = Lvl0Block;

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


pub fn bench_iter(c: &mut Criterion) {
    let mut block_array = BlockArray::default();
    let mut vec = Vec::default();
    for i in 0..3000{
        *block_array.get_or_insert(i) = DataBlock(i as u64);
        vec.push(DataBlock(i as u64));
    }

    c.bench_function("block array", |b| b.iter(|| array_iter(black_box(&block_array))));
    c.bench_function("vec", |b| b.iter(|| vec_iter(black_box(&vec))));
}

criterion_group!(benches_iter, bench_iter);
criterion_main!(benches_iter);