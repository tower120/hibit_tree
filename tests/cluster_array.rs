use hi_sparse_array::block::{Block, LevelBlock};
use hi_sparse_array::cluster_block::ClusterBlock;
use hi_sparse_array::simple_iter::SimpleBlockIter;
use hi_sparse_array::small_block::CompactBlock;
use hi_sparse_array::SparseBlockArray;
use itertools::assert_equal;

#[derive(Clone, Debug)]
struct DataBlock(u64);
impl LevelBlock for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }

    fn as_u64_mut(&mut self) -> &mut u64 {
        &mut self.0
    }

    fn restore_empty_u64(&mut self) {
        self.0 = 0;
    }
}

#[test]
fn insert_test(){
    type Lvl0Block = Block<u64, [u8;64]>;
    type ClusterLvl1Block = ClusterBlock<u64, [u16;4], [u16;16]>;
    type Array = SparseBlockArray<Lvl0Block, ClusterLvl1Block, DataBlock>;
    
    let mut array: Array = Default::default(); 
    
    let range = 0..3000;
    for i in range.clone(){
        *array.get_or_insert(i as usize) = DataBlock(i as u64);
    }

    let values = SimpleBlockIter::new(&array).map(|(_,v)|v.0);
    assert_equal(values, range.clone());
}