use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{BitAnd, Mul};
use wide::f32x4;
use hi_sparse_array::{apply, BitBlock, Empty};
use hi_sparse_array::level_block::Block;
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::const_utils::ConstFalse;
use hi_sparse_array::level::SingleBlockLevel;
use hi_sparse_array::BinaryOp;
use hi_sparse_array::SparseHierarchy;
use hi_sparse_array::utils::Take;

#[derive(Clone)]
struct DataBlock(f32x4);
impl Mul for DataBlock{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Empty for DataBlock{
    fn empty() -> Self {
        Self(f32x4::ZERO)
    }

    fn is_empty(&self) -> bool {
        // ??? 
        self.0 == f32x4::ZERO
    }
}

type Lvl0Block = Block<u64, [u8; 64]>;
type Lvl1Block = Block<u64, [u16; 64]>;
type SparseArray = hi_sparse_array::SparseArray<
    (
        SingleBlockLevel<Lvl0Block>,
        //Level<Lvl1Block>,
    ),
    DataBlock
>;

#[derive(Default)]
struct SparseVector {
    sparse_array: SparseArray
}

impl SparseVector{
    // TODO: This is actually set_non_empty!
    pub fn set(&mut self, index: usize, value: f32){
        const BLOCK_SIZE: usize = 4;
        let block_index   = index / BLOCK_SIZE;
        let in_block_index= index % BLOCK_SIZE;        
        
        let block = self.sparse_array.get_mut(block_index);
        unsafe{
            *block.0.as_array_mut().get_unchecked_mut(in_block_index) = value;
        }
    }
}

pub struct MulOp<M, D>(PhantomData<(M, D)>);
impl<M, D> Default for MulOp<M, D>{
    fn default() -> Self {
        Self(PhantomData)
    }
} 

impl<M, D> BinaryOp for MulOp<M, D>
where
    M: BitBlock + BitAnd<Output = M>, 
    D: Mul<Output = D> + Empty + Clone
{
    const EXACT_HIERARCHY: bool = false;
    type SKIP_EMPTY_HIERARCHIES = ConstFalse;
    
    type LevelMask = M;
    fn lvl_op(&self, left: impl Take<M>, right: impl Take<M>) -> Self::LevelMask {
        left.take_or_clone() & right.take_or_clone()
    }

    type Left  = D;
    type Right = D;
    type Out   = D;
    fn data_op(&self, left: impl Borrow<D> + Take<D>, right: impl Borrow<D> + Take<D>) -> Self::Out {
        left.take_or_clone() * right.take_or_clone()
    }
}

// TODO: should be lazy in all ways.
/// Per-element multiplication
pub fn mul<'a>(v1: &'a SparseVector, v2: &'a SparseVector) 
    -> /*Apply<
        MulOp<u64, DataBlock>, 
        &'a SparseArray, 
        &'a SparseArray,
    >*/
    impl SparseHierarchy<DataType=DataBlock> + 'a
{
    apply(
        MulOp::default(),
        &v1.sparse_array,
        &v2.sparse_array
    )
}

pub fn dot(v1: &SparseVector, v2: &SparseVector) -> f32 {
    let m = mul(v1, v2);
    let iter = CachingBlockIter::new(&m);
    let mut sum = f32x4::ZERO;
    iter.for_each(|(index, block)|{
        sum += block.borrow().0;
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