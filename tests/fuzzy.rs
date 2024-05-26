//! Fuzzy tests

mod common;

use std::collections::HashMap;
use itertools::assert_equal;
use rand::{Rng, SeedableRng};
use hi_sparse_array::{MaybeEmpty, SparseArray};
use hi_sparse_array::sparse_hierarchy::SparseHierarchy;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct Data(usize);
impl MaybeEmpty for Data{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
} 

type Array = common::Array<Data>;
type Map = HashMap<usize, Data>;

#[test]
fn fuzzy_test(){
    const RANGE: usize = common::RANGE;
    const COUNT: usize = 4000;
    
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);    

    let mut a = Array::default();
    let mut h = Map::default();
    
    fn check(rng: &mut impl Rng, a: &Array, h: &Map) {
        // iter + unordered_iter
        {
            let a_items: Vec<_> = a.iter().map(|(_,d)|d).collect();
            
            let mut a_unordered_items: Vec<_> = a.unordered_iter().map(|(_,d)|d).collect();
            a_unordered_items.sort();
            
            let mut h_items: Vec<_> = h.iter().map(|(_,d)|d).collect();
            h_items.sort();
            
            assert_equal(&a_items, &a_unordered_items);
            assert_equal(&a_unordered_items, &h_items);
        }
        
        // get
        for (k, v) in h {
            let d = a.get(*k).0;
            assert_eq!(d, v.0);
        }
        
        // random get
        for _ in 0..COUNT {
            let v = rng.gen_range(0..RANGE);
            let d1 = a.get(v).0;
            let d2 = match h.get(&v){
                None => 0,
                Some(d) => d.0
            };
            assert_eq!(d1, d2);
        }          
    }
 
    for _ in 0..10 {
        // insert
        for _ in 0..rng.gen_range(0..COUNT) {
            let v = rng.gen_range(0..RANGE);
            *a.get_or_insert(v) = Data(v);
            h.insert(v, Data(v));
        }
        check(&mut rng, &a, &h);   
        
        // remove
        for i in 0..rng.gen_range(0..COUNT) {
            let v = rng.gen_range(0..RANGE);
            a.remove(v);
            h.remove(&v);
        }
        check(&mut rng, &a, &h);
    }
    
    // remove all
    for k in h.keys() {
        a.remove(*k);
    }
    h.clear();
    check(&mut rng, &a, &h);
}