use itertools::{assert_equal, Itertools};
use hi_sparse_array::block::{Block, BypassBlock, LevelBlock};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::simple_iter::SimpleBlockIter;
use hi_sparse_array::SparseBlockArray;

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
fn bypass_test(){
    type Lvl0Block = Block<u64, [u8;64]>;
    type Array = SparseBlockArray<Lvl0Block, BypassBlock, DataBlock>;
    
    let mut array: Array = Default::default(); 
    
    let range = 0..60;
    for i in range.clone(){
        *array.get_or_insert(i as usize) = DataBlock(i as u64);
    }

    let values = CachingBlockIter::new(&array).map(|(_,v)|v.0);
    assert_equal(values, range.clone());
}