use std::borrow::Borrow;
use wide::f32x4;
use hibit_tree::{config, intersection, ReqDefault};
use hibit_tree::HibitTree;
use hibit_tree::RegularHibitTree;
use hibit_tree::Iter;

type SparseArray = hibit_tree::Tree<f32x4, config::_64bit<2>, ReqDefault>;

#[derive(Default)]
struct SparseVector {
    sparse_array: SparseArray
}

impl SparseVector{
    pub fn set(&mut self, index: usize, value: f32){
        const BLOCK_SIZE: usize = 4;
        let block_index   = index / BLOCK_SIZE;
        let in_block_index= index % BLOCK_SIZE;        
        
        let block = self.sparse_array.get_or_insert(block_index);
        unsafe{
            *block.as_array_mut().get_unchecked_mut(in_block_index) = value;
        }
    }
}

/// Per-element multiplication
pub fn mul<'a>(v1: &'a SparseVector, v2: &'a SparseVector) 
    -> impl RegularHibitTree<Data=f32x4> + 'a
{
    intersection(&v1.sparse_array, &v2.sparse_array)
        .map(|(l, r): (&f32x4, &f32x4)| *l * *r )
}

pub fn dot(v1: &SparseVector, v2: &SparseVector) -> f32 {
    let m = mul(v1, v2);
    let iter = Iter::new(&m);
    let mut sum = f32x4::ZERO;
    iter.for_each(|(index, block)|{
        sum += block;
    });
    sum.reduce_add()
}


fn main(){
    let mut v1 = SparseVector::default();
    let mut v2 = SparseVector::default();
    
    let INDEX_MUL: usize = 1; 
    
    v1.set(10*INDEX_MUL, 1.0);
    v1.set(20*INDEX_MUL, 10.0);
    v1.set(30*INDEX_MUL, 100.0);
    
    v2.set(10*INDEX_MUL, 1.0);
    v2.set(30*INDEX_MUL, 0.5);
    
    let d = dot(&v1, &v2);
    assert_eq!(d, 51.0 )
}