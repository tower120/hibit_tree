#![feature(closure_lifetime_binder)]

use itertools::assert_equal;
use rand::{Rng, SeedableRng};
//use hi_sparse_array::{CompactSparseArray, intersection, LazySparseHierarchy, map, map2, union};
use hi_sparse_array::{CompactSparseArray, FromSparseHierarchy, intersection, LazySparseHierarchy, map, union};
use hi_sparse_array::const_utils::ConstUsize;
use hi_sparse_array::SparseHierarchy;
use hi_sparse_array::RegularSparseHierarchy;
use hi_sparse_array::utils::{Borrowable, UnaryFunction};

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

    {
        let mut a1: CompactSparseArray<_, 4> = Default::default();
        let i0 = 0;
        a1.insert(0, i0);
        
        let m1 = map(a1, |d: &usize| -> usize { d.clone() } );
        let m2 = map(m1, |d: usize| d.clone() );
        // let mut a2: CompactSparseArray<_, 4> = m2.materialize();
        
        /*fn test2<
            T: SparseHierarchy<LevelCount=ConstUsize<4>> 
        >(a: T)
            //-> CompactSparseArray<usize, 4>
        {
            let t = a.get(12);
            //a.materialize()
        }
        test2(a1);*/
        
        /*
        // This not work
        fn test<'a, T: Borrowable<Borrowed: SparseHierarchy<'a, LevelMaskType=u64, LevelCount=ConstUsize<4>, DataType=usize>>>(a: T)
            //-> CompactSparseArray<usize, 4>
        {
            a.borrow().get(12);
        }
        test(a1);
        */
        
        //let a1c: CompactSparseArray<_, 4> = m1.materialize();
        //v.push(&i0);
        
        //let u = intersection(&a1, &a1, |_, _| 1 );
        /*let m1 = map(a1, |v|v.clone());
        let m2 = map(&m1, |v|v.clone());
        let m3 = map(&m2, |v|v.clone());*/
        //intersection(u, &a2, |_, _|2);    
    }
    
    
    let ao: Array = map(&a1, |d: &Data| d.clone()).materialize();
    assert_equal(ao.iter(), a1.iter());
    
    let ao: Array = map(intersection(&a1, &a2), |(l, _r) : (&Data, &Data)| l.clone()).materialize();
    assert_equal(ao.iter(), a1.iter());
    
    let ao: Array = union(&a1, &a2)
        .map(|(l, _r) : (Option<&Data>, Option<&Data>)| l.unwrap().clone())
        .materialize();
    assert_equal(ao.iter(), a1.iter());
}