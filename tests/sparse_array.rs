//! SparseArray general tests

mod common;

use rand::{Rng, SeedableRng};
use rand::prelude::SliceRandom;
use hi_sparse_array::{Empty, SparseArray};
use hi_sparse_array::SparseHierarchy;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct Data(usize);
impl Empty for Data {
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
} 

type Array = common::Array<Data>;

#[test]
fn insert_test(){
    const RANGE: usize = common::RANGE;
    const COUNT: usize = 4000;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);
    
    let mut small_block_array = Array::default();
    for _ in 0..COUNT{
        let v = rng.gen_range(0..RANGE);
        *small_block_array.get_mut(v) = Data(v);
    }
}

#[test]
fn remove_test(){
    let mut a = Array::default();
    *a.get_mut(1) = Data(1);
    *a.get_mut(2) = Data(2);
    *a.get_mut(400) = Data(400);
    
    a.remove(1);
    a.remove(2);
    a.remove(400);
}