use std::marker::PhantomData;
use std::ops::{BitAnd, Mul};
use wide::f32x4;
use hi_sparse_array::{Apply, apply, BitBlock, Op, SparseBlockArray};
use hi_sparse_array::block::{Block, FixedHiBlock};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::simple_iter::SimpleBlockIter;

#[derive(Clone)]
struct DataBlock(f32x4);
impl Mul for DataBlock{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Block for DataBlock{
    fn empty() -> Self {
        Self(f32x4::ZERO)
    }

    fn is_empty(&self) -> bool {
        // ??? 
        self.0 == f32x4::ZERO
    }

    fn as_u64_mut(&mut self) -> &mut u64 {
        unsafe{
            &mut*self.0.as_array_mut().as_mut_ptr().cast::<u64>()
        }
    }

    fn restore_empty_u64(&mut self) {
        // Is this correct for float??
        *self.as_u64_mut() = 0;
    }
}

type Lvl0Block = FixedHiBlock<u64, [u8; 64]>;
type Lvl1Block = FixedHiBlock<u64, [u16; 64]>;
type SparseArray = SparseBlockArray<
    Lvl0Block,
    Lvl1Block,
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
        
        let block = self.sparse_array.get_or_insert(block_index);
        unsafe{
            *block.0.as_array_mut().get_unchecked_mut(in_block_index) = value;
        }
    }
}

pub struct MulOp<L0, L1, LD>(PhantomData<(L0, L1, LD)>);
impl<L0, L1, LD> Default for MulOp<L0, L1, LD>{
    fn default() -> Self {
        Self(PhantomData)
    }
} 

impl<L0, L1, LD> Op for MulOp<L0, L1, LD>
where
    L0: BitAnd<Output = L0>, L1:BitAnd<Output = L1>, LD: Mul<Output = LD>
{
    type Level0Mask = L0;
    fn lvl0_op(left: Self::Level0Mask, right: Self::Level0Mask) -> Self::Level0Mask {
        left & right
    }

    type Level1Mask = L1;
    fn lvl1_op(left: Self::Level1Mask, right: Self::Level1Mask) -> Self::Level1Mask {
        left & right
    }

    type DataBlock = LD;
    fn data_op(left: Self::DataBlock, right: Self::DataBlock) -> Self::DataBlock {
        left * right
    }
}

// TODO: should be lazy in all ways.
/// Per-element multiplication
pub fn mul<'a>(v1: &'a SparseVector, v2: &'a SparseVector) -> Apply<MulOp<u64, u64, DataBlock>, &'a SparseArray, &'a SparseArray>{
    apply(
        Default::default(),
        &v1.sparse_array,
        &v2.sparse_array
    )
}

pub fn dot(v1: &SparseVector, v2: &SparseVector) -> f32 {
    let m = mul(v1, v2);
    let iter = CachingBlockIter::new(&m);
    let mut sum = f32x4::ZERO;
    iter.for_each(|(index, block)|{
        sum += block.0;
    });
    sum.reduce_add()
}



fn main(){
    let mut v1 = SparseVector::default();
    let mut v2 = SparseVector::default();
    
    v1.set(100, 1.0);
    v1.set(200, 10.0);
    v1.set(300, 100.0);
    
    v2.set(100, 1.0);
    v2.set(300, 0.5);
    
    let d = dot(&v1, &v2);
    assert_eq!(d, 51.0 )
}