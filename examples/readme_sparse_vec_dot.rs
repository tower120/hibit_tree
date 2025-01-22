use hibit_tree::{intersection, HibitTree, Tree};
use hibit_tree::config::_64bit;

fn main(){
    type SparseVec = Tree<f32, _64bit<4>>;
    
    let mut v1: SparseVec  = Default::default();
    v1.insert(10, 1.0);
    v1.insert(20, 10.0);
    v1.insert(30, 100.0);
    
    let mut v2: SparseVec = Default::default();
    v2.insert(10, 1.0);
    v2.insert(30, 0.5);
    
    let mul = intersection(&v1, &v2)            // lazy element-wise mul
        .map(|(e1, e2): (&f32, &f32)| e1 * e2);
    
    // Only 2 element pairs are actually multiplied, 
    // everything else never touched. 
    let dot: f32 = mul.iter().map(|(_index, element)| element).sum();
    
    assert_eq!(dot, 51.0);
}