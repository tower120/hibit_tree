use std::any::Any;
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitOr, Mul};
use criterion::{black_box, Criterion, criterion_group, criterion_main};
use rand::{Rng, SeedableRng};
use hi_sparse_array::{apply, BitBlock, Empty, fold, SparseArray};
use hi_sparse_array::level_block::{Block, SmallBlock};
use hi_sparse_array::Iter;
use hi_sparse_array::const_utils::{ConstFalse, ConstTrue};
use hi_sparse_array::level::{IntrusiveListLevel, SingleBlockLevel};
use hi_sparse_array::BinaryOp;
use hi_sparse_array::compact_sparse_array2::CompactSparseArray;
use hi_sparse_array::ops::union2::union2;
use hi_sparse_array::ops::union3::union;
use hi_sparse_array::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use hi_sparse_array::utils::Take;

type Lvl0Block = Block<u64, [u8;64]>;
type Lvl1Block = Block<u64, [u16;64]>;
type SmallLvl1Block = SmallBlock<u64, [u8;1], [u16;64], [u16;7]>;

/*type Lvl0Block = Block<wide::u64x2, [u8;128]>;
type Lvl1Block = Block<wide::u64x2, [u16;128]>;*/

#[derive(Clone, Default)]
struct DataBlock(u64);
impl BitAnd for DataBlock{
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAnd for &DataBlock{
    type Output = DataBlock;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        DataBlock(self.0 & rhs.0)
    }
}

impl BitOr for DataBlock{
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}
impl Empty for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
}

type BlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>), /*IntrusiveListLevel<*/DataBlock/*>*/>;
type SmallBlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<SmallLvl1Block>), DataBlock>;
type CompactArray = CompactSparseArray<DataBlock, 2>;

pub struct AndOp<M, LD>(PhantomData<(M, LD)>);
impl<M, LD> Default for AndOp<M, LD>{
    fn default() -> Self {
        Self(PhantomData)
    }
} 

impl<M, LD> BinaryOp for AndOp<M, LD>
where
    M: BitBlock + BitAnd<Output = M>, 
    LD: BitAnd<Output = LD> + Empty,
    for<'a> &'a LD: BitAnd<&'a LD, Output = LD>
{
    const EXACT_HIERARCHY: bool = true;
    type SKIP_EMPTY_HIERARCHIES = ConstTrue;
     
    type LevelMask = M;
    #[inline]
    fn lvl_op(&self, left: impl Take<M>, right: impl Take<M>) -> Self::LevelMask {
        left.take_or_clone() | right.take_or_clone()
    }

    type Left  = LD;
    type Right = LD;
    type Out   = LD;
    #[inline]
    fn data_op(&self, left: impl Borrow<LD>, right: impl Borrow<LD>) -> Self::Out {
        //left.into_owned() & right.into_owned()
        left.borrow() & right.borrow()
    }
}

/*pub struct OrOp<L0, L1, L2, LD>(PhantomData<(L0, L1, L2, LD)>);
impl<L0, L1, L2, LD> Default for OrOp<L0, L1, L2, LD> {
    fn default() -> Self {
        Self(PhantomData)
    }
} 

impl<L0, L1, L2, LD> Op for OrOp<L0, L1, L2, LD>
where
    L0: BitBlock + BitOr<Output = L0>, 
    L1: BitBlock + BitOr<Output = L1>, 
    L2: BitBlock + BitOr<Output = L2>, 
    LD: BitOr<Output = LD>
{
    const EXACT_HIERARCHY: bool = false;
    const SKIP_EMPTY_HIERARCHIES: bool = false;
     
    type Level0Mask = L0;
    #[inline]
    fn lvl0_op(&self, left: impl IntoOwned<L0>, right: impl IntoOwned<L0>) -> Self::Level0Mask {
        left.into_owned() | right.into_owned()
    }

    type Level1Mask = L1;
    #[inline]
    fn lvl1_op(&self, left: impl IntoOwned<L1>, right: impl IntoOwned<L1>) -> Self::Level1Mask {
        left.into_owned() | right.into_owned()
    }
    
    type Level2Mask = L2;
    #[inline]
    fn lvl2_op(&self, left: impl IntoOwned<L2>, right: impl IntoOwned<L2>) -> Self::Level2Mask {
        left.into_owned() | right.into_owned()
    }

    type DataBlock = LD;
    #[inline]
    fn data_op(&self, left: impl Borrow<LD> + IntoOwned<LD>, right: impl Borrow<LD> + IntoOwned<LD>) -> Self::DataBlock {
        left.into_owned() | right.into_owned()
    }
}
*/
fn fold_iter(list: &[BlockArray]) -> impl Any {
    let op = AndOp(PhantomData);
    
    let init  = unsafe{ list.get_unchecked(0) };
    let other = unsafe{ list.get_unchecked(1..) };
    
    
    let fold = fold(op, init, other.iter());
    
    let mut s = 0;
    for (_, i) in Iter::new(&fold){
        s += i.0;
    }
    s
}

/*fn fold_w_empty_iter(list: &[BlockArray]) -> u64 {
    let and_op: OrOp<u64, u64, EmptyBitBlock, DataBlock> = OrOp(PhantomData);
    let empty = Empty::<u64, u64, EmptyBitBlock, DataBlock>::default();
    
    let fold = fold(and_op, &empty, list.iter());
    
    let mut s = 0;
    for (_, i) in CachingBlockIter::new(&fold){
        s += i.0;
    }
    s
}*/

fn apply_small_iter(array1: &SmallBlockArray, array2: &SmallBlockArray) -> u64 {
    let and_op = AndOp(PhantomData);
    let reduce = apply(and_op, array1, array2);
    
    let mut s = 0;
    for (_, i) in Iter::new(&reduce){
        s += i.0;
    }
    s
}

fn apply_iter(array1: &BlockArray, array2: &BlockArray) -> u64 {
    let and_op = AndOp(PhantomData);
    let reduce = apply(and_op, array1, array2);
    
    let mut s = 0;
    for (_, i) in Iter::new(&reduce){
        s += i.0;
    }
    s
}

fn union2_iter<A1, A2>(array1: &A1, array2: &A2) -> u64
where
    A1: SparseHierarchy<DataType = DataBlock>,
    A2: SparseHierarchy<DataType = DataBlock, LevelCount = A1::LevelCount, LevelMaskType = A1::LevelMaskType>
{
    let union = union(array1, array2, |v0, v1|{
        let v0 = v0.map_or(0, |v|v.0);
        let v1 = v1.map_or(0, |v|v.0);
        DataBlock(v0 + v1)
    });
    
    let mut s = 0;
    for (_, i) in union.iter(){
        s += i.0;
    }
    s
}


pub fn bench_iter(c: &mut Criterion) {
    let mut block_array1 = BlockArray::default();
    let mut block_array2 = BlockArray::default();
    let mut block_array3 = BlockArray::default();
    let mut block_array4 = BlockArray::default();
    
    let mut small_block_array1 = SmallBlockArray::default();
    let mut small_block_array2 = SmallBlockArray::default();
    
    let mut compact_array1 = CompactArray::default();
    let mut compact_array2 = CompactArray::default();
    
    let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);

    for _ in 0..100{
        let i1 = rng.gen_range(0..100);
        let i2 = rng.gen_range(0..100);
        let i3 = rng.gen_range(0..100);
        let i4 = rng.gen_range(0..100);
        
        *block_array1.get_mut(i1*20) = DataBlock(i1 as u64);
        *block_array2.get_mut(i2*20) = DataBlock(i2 as u64);
        *block_array3.get_mut(i3*20) = DataBlock(i3 as u64);
        *block_array4.get_mut(i4*20) = DataBlock(i4 as u64);
        
        *small_block_array1.get_mut(i1*20) = DataBlock(i1 as u64);
        *small_block_array2.get_mut(i2*20) = DataBlock(i2 as u64);
        
        *compact_array1.get_or_insert(i1*20) = DataBlock(i1 as u64);
        *compact_array2.get_or_insert(i2*20) = DataBlock(i2 as u64);
    }
    let arrays = [block_array1, block_array2/*, block_array3*/];

    //c.bench_function("fold", |b| b.iter(|| fold_iter(black_box(&arrays))));
    c.bench_function("apply", |b| b.iter(|| apply_iter(black_box(&arrays[0]), black_box(&arrays[1]))));
    c.bench_function("array union2", |b| b.iter(|| union2_iter(black_box(&arrays[0]), black_box(&arrays[1]))));
    //return;
    
    //c.bench_function("apply_small", |b| b.iter(|| apply_small_iter(black_box(&small_block_array1), black_box(&small_block_array2))));
    c.bench_function("union2", |b| b.iter(|| union2_iter(black_box(&compact_array1), black_box(&compact_array2))));
    //c.bench_function("fold_w_empty", |b| b.iter(|| fold_w_empty_iter(black_box(&arrays))));
}

criterion_group!(benches_iter, bench_iter);
criterion_main!(benches_iter);