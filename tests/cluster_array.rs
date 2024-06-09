use hi_sparse_array::level_block::{Block, ClusterBlock};
use hi_sparse_array::{Empty, SparseArray};
use itertools::assert_equal;
use hi_sparse_array::Iter;
use hi_sparse_array::level::{IntrusiveListLevel, Level, SingleBlockLevel};

#[derive(Clone, Debug)]
struct DataBlock(u64);
impl Empty for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }
}

#[test]
fn insert_test(){
    type Lvl0Block = Block<u64, [u8;64]>;
    type ClusterLvl1Block = ClusterBlock<u64, [u16;4], [u16;16]>;
    type Array = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<ClusterLvl1Block>), DataBlock>;
    
    let mut array: Array = Default::default(); 
    
    let range = 0..3000;
    for i in range.clone(){
        *array.get_mut(i as usize) = DataBlock(i as u64);
    }

    let values = Iter::new(&array).map(|(_,v)|v.0);
    assert_equal(values, range.clone());
}