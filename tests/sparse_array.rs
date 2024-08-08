//! SparseArray general tests

mod common;

use itertools::assert_equal;
use rand::{Rng, SeedableRng};
use rand::prelude::SliceRandom;
use hi_sparse_array::{config, SparseHierarchy};
use hi_sparse_array::utils::LendingIterator;

#[derive(Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct Data(usize);

type Array = common::Array<Data>;

/*#[test]
fn smoke_test(){
    {
        let mut array = hi_sparse_array::SparseArray::<config::width_64::depth_2, Data>::default();
        array.insert(10, Data(10));
    }
    
    let mut array = Array::default();
    //array.insert(143231, Data(143231));
    //array.insert(175928, Data(175928));
}*/

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
fn insert_test2(){
    #[cfg(miri)]
    const COUNT: usize = 2000;
    #[cfg(not(miri))]
    const COUNT: usize = 200_000;
    
    let mut a = Array::default();
    for i in 0..COUNT{
        *a.get_or_insert(i) = Data(i);
    }
    
    for i in 0..COUNT{
        let v = a.get(i).unwrap();
        assert_eq!(v, &Data(i));
    }

    // with LendingIterator
    let mut iter = a.iter();
    let mut i = 0;
    while let Some((key, value)) = LendingIterator::next(&mut iter) {
        assert_eq!(key, value.0);
        assert_eq!(key, i);
        i += 1;
    }

    // with Iterator
    assert_equal(a.iter().map(|(key, value)| value.0), 0..COUNT);
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