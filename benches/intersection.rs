use std::any::Any;
use std::marker::PhantomData;
use std::borrow::Borrow;
use criterion::{black_box, Criterion, criterion_group, criterion_main};
use rand::{Rng, SeedableRng};
use hi_sparse_array::{CompactSparseArray, config, Iter, multi_intersection, /*SparseArray, */SparseHierarchy};
use hi_sparse_array::utils::LendingIterator;
//use hi_sparse_array::ops::multi_intersection_fold;

#[derive(Default)]
struct DataBlock(u64);

//type BlockArray = SparseArray<config::width_64::depth_4, DataBlock>;
type CompactArray = CompactSparseArray<DataBlock, 4>;

fn bench_multi_intersection(list: &[CompactArray]) -> u64 {
    let intersection = multi_intersection(list.iter());

    let mut sum = 0;
    let mut intersection = intersection.iter();
    while let Some((index, ds)) = intersection.next(){
        sum += ds.fold(0, |acc, d| acc+d.0)
    }
    sum
}
/*
fn bench_multi_intersection_fold(list: &[impl SparseHierarchy<DataType = DataBlock>]) -> u64 {
    let intersection = multi_intersection_fold::multi_intersection(list.iter(), 0, |acc, d| acc + d.borrow().0 );
    intersection.iter().map(|(k,v)|v).sum()
}

fn bench_multi_intersection_get(list: &[impl SparseHierarchy<DataType = DataBlock>]) -> u64 {
    let intersection = multi_intersection(list.iter(), |ds| -> u64 { 
        ds.map(|d| d.borrow().0).sum() 
    });
    
    let mut s = 0;
    for i in 0..10000 {
        if let Some(d) = intersection.get(i){
            s+=d;
        }
    }
    s
}

fn bench_multi_intersection_fold_get(list: &[impl SparseHierarchy<DataType = DataBlock>]) -> u64 {
    let intersection = multi_intersection_fold::multi_intersection(list.iter(), 0, |acc, d| acc + d.borrow().0);
    
    let mut s = 0;
    for i in 0..10000 {
        if let Some(d) = intersection.get(i){
            s+=d;
        }
    }
    s
}*/


pub fn bench_iter(c: &mut Criterion) {
    const COUNT: usize = 10000;
    const MAX_RANGE: usize = 1000;  
    
    /*let mut block_array1 = BlockArray::default();
    let mut block_array2 = BlockArray::default();
    let mut block_array3 = BlockArray::default();
    let mut block_array4 = BlockArray::default();*/
    
    let mut compact_array1 = CompactArray::default();
    let mut compact_array2 = CompactArray::default();
    let mut compact_array3 = CompactArray::default();
    let mut compact_array4 = CompactArray::default();
    
    
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);

    for _ in 0..COUNT{
        let i1 = rng.gen_range(0..MAX_RANGE);
        let i2 = rng.gen_range(0..MAX_RANGE);
        let i3 = rng.gen_range(0..MAX_RANGE);
        let i4 = rng.gen_range(0..MAX_RANGE);
        
        /*let i2 = i1;
        let i3 = i1;
        let i4 = i1;*/
        
        /*
        *block_array1.get_or_insert(i1*20) = DataBlock(i1 as u64);
        *block_array2.get_or_insert(i2*20) = DataBlock(i2 as u64);
        *block_array3.get_or_insert(i3*20) = DataBlock(i3 as u64);
        *block_array4.get_or_insert(i4*20) = DataBlock(i4 as u64);
        */
        
        *compact_array1.get_or_insert(i1*20) = DataBlock(i1 as u64);
        *compact_array2.get_or_insert(i2*20) = DataBlock(i2 as u64);
        *compact_array1.get_or_insert(i3*20) = DataBlock(i1 as u64);
        *compact_array2.get_or_insert(i4*20) = DataBlock(i2 as u64);
    }
    // let arrays = [block_array1, block_array2, block_array3, block_array4];
    let compact_arrays = [compact_array1, compact_array2, compact_array3, compact_array4];    
    
    // c.bench_function("bench_multi_intersection_get", |b| b.iter(|| bench_multi_intersection_get(black_box(&compact_arrays))));
    // c.bench_function("bench_multi_intersection_fold_get", |b| b.iter(|| bench_multi_intersection_fold_get(black_box(&compact_arrays))));
    
    c.bench_function("bench_multi_intersection", |b| b.iter(|| bench_multi_intersection(black_box(&compact_arrays))));
    // c.bench_function("bench_multi_intersection_fold", |b| b.iter(|| bench_multi_intersection_fold(black_box(&arrays))));
}

criterion_group!(benches_iter, bench_iter);
criterion_main!(benches_iter);
