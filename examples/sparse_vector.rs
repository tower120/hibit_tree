use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{BitAnd, Mul};
use wide::f32x4;
use hi_sparse_array::{Apply, apply, BitBlock, Op, IntoOwned, SparseArray};
use hi_sparse_array::bit_queue::EmptyBitQueue;
use hi_sparse_array::level_block::{LevelBlock, Block};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::level::{BypassLevel, Level};
//use hi_sparse_array::simple_iter::SimpleBlockIter;

#[derive(Clone)]
struct DataBlock(f32x4);
impl Mul for DataBlock{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl LevelBlock for DataBlock{
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

#[derive(Eq, PartialEq, Copy, Clone)]
struct EmptyMask;
impl BitBlock for EmptyMask{
    const SIZE_POT_EXPONENT: usize = 0;

    fn zero() -> Self {
        Self
    }

    type BitsIter = EmptyBitQueue;

    fn into_bits_iter(self) -> Self::BitsIter {
        EmptyBitQueue
    }

    type Array = [u64; 0];

    fn as_array(&self) -> &Self::Array {
        todo!()
    }

    fn as_array_mut(&mut self) -> &mut Self::Array {
        todo!()
    }
} 
impl BitAnd for EmptyMask{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

type Lvl0Block = Block<u64, [u8; 64]>;
type Lvl1Block = Block<u64, [u16; 64]>;
type SparseArray = SparseArray<
    Lvl0Block,
    //Level<Lvl1Block>,
    BypassLevel<EmptyMask>,
    BypassLevel<EmptyMask>,
    Level<DataBlock>
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

pub struct MulOp<L0, L1, L2, LD>(PhantomData<(L0, L1, L2, LD)>);
impl<L0, L1, L2, LD> Default for MulOp<L0, L1, L2, LD>{
    fn default() -> Self {
        Self(PhantomData)
    }
} 

impl<L0, L1, L2, LD> Op for MulOp<L0, L1, L2, LD>
where
    L0: BitBlock + BitAnd<Output = L0>, 
    L1: BitBlock + BitAnd<Output = L1>, 
    L2: BitBlock + BitAnd<Output = L2>, 
    LD: Mul<Output = LD>
{
    const EXACT_HIERARCHY: bool = false;
    const SKIP_EMPTY_HIERARCHIES: bool = false;
    
    type Level0Mask = L0;
    fn lvl0_op(&self, left: impl IntoOwned<L0>, right: impl IntoOwned<L0>) -> Self::Level0Mask {
        left.into_owned() & right.into_owned()
    }

    type Level1Mask = L1;
    fn lvl1_op(&self, left: impl IntoOwned<L1>, right: impl IntoOwned<L1>) -> Self::Level1Mask {
        left.into_owned() & right.into_owned()
    }
    
    type Level2Mask = L2;
    fn lvl2_op(&self, left: impl IntoOwned<L2>, right: impl IntoOwned<L2>) -> Self::Level2Mask {
        left.into_owned() & right.into_owned()
    }

    type DataBlock = LD;
    fn data_op(&self, left: impl Borrow<LD> + IntoOwned<LD>, right: impl Borrow<LD> + IntoOwned<LD>) -> Self::DataBlock {
        left.into_owned() * right.into_owned()
    }
}

// TODO: should be lazy in all ways.
/// Per-element multiplication
pub fn mul<'a>(v1: &'a SparseVector, v2: &'a SparseVector) 
    -> Apply<
        MulOp<u64, EmptyMask, EmptyMask, DataBlock>, 
        &'a SparseArray, 
        &'a SparseArray,
        SparseArray,
        SparseArray
    >
{
    apply(
        MulOp::default(),
        &v1.sparse_array,
        &v2.sparse_array
    )
}

pub fn dot(v1: &SparseVector, v2: &SparseVector) -> f32 {
    let m = mul(v1, v2);
    let iter =
        CachingBlockIter::new(&m);
        //SimpleBlockIter::new(&m);
    let mut sum = f32x4::ZERO;
    iter.for_each(|(index, block)|{
        sum += block.0;
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