use std::collections::HashMap;
use criterion::{black_box, Criterion, criterion_group, criterion_main};
use hi_sparse_array::{Empty, SparseArray};
use hi_sparse_array::compact_sparse_array2::CompactSparseArray;
use hi_sparse_array::level_block::{Block, ClusterBlock, SmallBlock};
use hi_sparse_array::Iter;
use hi_sparse_array::level::{IntrusiveListLevel, SingleBlockLevel};

type Lvl0Block = Block<u64, [u8;64]>;
type Lvl1Block = Block<u64, [u16;64]>;
type Lvl2Block = Block<u64, [u32;64]>;
type CompactLvl1Block = SmallBlock<u64, [u8;1], [u16;64], [u16;7]>;
type ClusterLvl1Block = ClusterBlock<u64, [u16;4], [u16;16]>;

#[derive(Clone, Default)]
struct DataBlock(u64);
impl Empty for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
}

type BlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>), DataBlock>;
type SmallBlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<CompactLvl1Block>), DataBlock>;
type ClusterBlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<ClusterLvl1Block>), DataBlock>;
type CompactArray = CompactSparseArray<DataBlock, 2>;

fn compact_array_iter(array: &CompactArray) -> u64 {
    use hi_sparse_array::sparse_hierarchy2::SparseHierarchy2;
    
    let mut s = 0;
    for (_, i) in array.iter(){
        s += i.0;
    }
    s
}

fn cluster_array_iter(array: &ClusterBlockArray) -> u64 {
    let mut s = 0;
    for (_, i) in Iter::new(array){
        s += i.0;
    }
    s
}

fn small_array_iter(array: &SmallBlockArray) -> u64 {
    let mut s = 0;
    for (_, i) in Iter::new(array){
        s += i.0;
    }
    s
}

fn array_iter(array: &BlockArray) -> u64 {
    let mut s = 0;
    for (_, i) in Iter::new(array){
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
    let mut cluster_block_array = ClusterBlockArray::default();
    let mut compact_array = CompactArray::default();
    let mut vec = Vec::default();
    let mut hashmap = HashMap::default();
    for i in 0..3000{
        *block_array.get_mut(i) = DataBlock(i as u64);
        *small_block_array.get_mut(i) = DataBlock(i as u64);
        *cluster_block_array.get_mut(i) = DataBlock(i as u64);
        *compact_array.get_or_insert(i) = DataBlock(i as u64);
        vec.push(DataBlock(i as u64));
        hashmap.insert(i as u64, DataBlock(i as u64));
    }

    c.bench_function("level_block array", |b| b.iter(|| array_iter(black_box(&block_array))));
    c.bench_function("small level_block array", |b| b.iter(|| small_array_iter(black_box(&small_block_array))));
    //c.bench_function("cluster level_block array", |b| b.iter(|| cluster_array_iter(black_box(&cluster_block_array))));
    c.bench_function("compact array", |b| b.iter(|| compact_array_iter(black_box(&compact_array))));
    c.bench_function("vec", |b| b.iter(|| vec_iter(black_box(&vec))));
    c.bench_function("hashmap", |b| b.iter(|| hashmap_iter(black_box(&hashmap))));
}

criterion_group!(benches_iter, bench_iter);
criterion_main!(benches_iter);