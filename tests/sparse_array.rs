//! SparseArray general tests

mod common;

use rand::{Rng, SeedableRng};
use rand::prelude::SliceRandom;
use hi_sparse_array::SparseHierarchy2;

#[derive(Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct Data(usize);

type Array = common::Array<Data>;

#[test]
fn insert_test(){
    const RANGE: usize = common::RANGE;
    const COUNT: usize = 4000;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);
    
    let mut array = Array::default();
    for _ in 0..COUNT{
        let v = rng.gen_range(0..RANGE);
        array.insert(v, Data(v));
        //*array.get_or_insert(v) = Data(v);
    }
}

#[test]
fn remove_test(){
    let mut a = Array::default();
    *a.get_or_insert(1) = Data(1);
    *a.get_or_insert(2) = Data(2);
    *a.get_or_insert(400) = Data(400);
    
    a.get_or_insert(1);
    a.get_or_insert(2);
    a.get_or_insert(400);
}