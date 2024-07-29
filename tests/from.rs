use itertools::assert_equal;
use rand::{Rng, SeedableRng};
use hi_sparse_array::{CompactSparseArray, intersection, LazySparseHierarchy, map, union};
use hi_sparse_array::SparseHierarchy;

mod common;

#[derive(Default, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct Data(usize);

// TODO: common::Array<Data>
type Array = CompactSparseArray<Data, 4>;

#[test]
fn materialize_test(){
    const RANGE: usize = common::RANGE;
    const COUNT: usize = 4000;

    let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);
    
    let mut a1 = Array::default();
    let mut a2 = Array::default();
    for _ in 0..COUNT{
        let v = rng.gen_range(0..RANGE);
        a1.insert(v, Data(v));
        a2.insert(v, Data(v));
    }
    
    let ao: Array = map(&a1, |d| d.clone()).materialize();
    assert_equal(ao.iter(), a1.iter());
    
    let ao: Array = intersection(&a1, &a2, |l, _r| l.clone()).materialize();
    assert_equal(ao.iter(), a1.iter());
    
    let ao: Array = union(&a1, &a2, |l, _r| l.unwrap().clone()).materialize();
    assert_equal(ao.iter(), a1.iter());
}