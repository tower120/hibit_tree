use hibit_tree::{intersection, RegularHibitTree, DenseTree, HibitTree};

fn main(){
    type SparseVec = DenseTree<f32, 4>;
    
    let mut v1: SparseVec  = Default::default();
    v1.insert(10, 1.0);
    v1.insert(20, 10.0);
    v1.insert(30, 100.0);
    
    let mut v2: SparseVec = Default::default();
    v2.insert(10, 1.0);
    v2.insert(30, 0.5);
    
    let mul = intersection(&v1, &v2)            // lazy element-wise mul
        .map(|(e1, e2): (&f32, &f32)| e1 * e2);
    let dot: f32 = mul.iter().map(|(_index, element)| element).sum();
    
    assert_eq!(dot, 51.0);
}