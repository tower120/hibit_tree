use std::collections::{BTreeMap, HashMap};
use criterion::{black_box, Criterion, criterion_group, criterion_main};
use rand::{Rng, SeedableRng};
use hibit_tree::{HibitTree, HibitTreeData, HibitTreeTypes, Iter, RegularHibitTree};
use hibit_tree::config::*;

type Tree = hibit_tree::Tree<DataBlock, _64bit<4>>;

#[derive(Clone, Default)]
struct DataBlock(u64);

fn hibit_tree_iter<T>(tree: &T) -> u64
where 
    T: RegularHibitTree,
    T: for<'a> HibitTreeTypes<'a, Data = &'a DataBlock>,
{
    let mut s = 0;
    for (_, i) in tree.iter(){
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

fn btree_iter(array: &BTreeMap<u64, DataBlock>) -> u64 {
    let mut s = 0;
    for (_, i) in array.iter(){
        s += i.0;
    }
    s
}

pub fn bench_iter(c: &mut Criterion) {
    let mut tree = Tree::default();
    let mut vec = Vec::default();
    let mut hashmap = HashMap::default();
    let mut btree = BTreeMap::default();
    
    let RANGE: usize = Tree::index_range().end;
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);
    for n in 0..3000{
        let i = rng.gen_range(0..RANGE);
        //let i = n;
        tree.insert(i, DataBlock(i as u64));
        vec.push(DataBlock(i as u64));
        hashmap.insert(i as u64, DataBlock(i as u64));
        btree.insert(i as u64, DataBlock(i as u64));
    }

    c.bench_function("tree", |b| b.iter(|| hibit_tree_iter(black_box(&tree))));
    c.bench_function("vec", |b| b.iter(|| vec_iter(black_box(&vec))));
    c.bench_function("hashmap", |b| b.iter(|| hashmap_iter(black_box(&hashmap))));
    c.bench_function("btree", |b| b.iter(|| btree_iter(black_box(&btree))));
}

criterion_group!(benches_iter, bench_iter);
criterion_main!(benches_iter);