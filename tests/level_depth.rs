use std::ops::Range;
use itertools::{assert_equal, Itertools};
use hi_sparse_array::level_block::{Block, MaybeEmpty};
use hi_sparse_array::level::{ILevel, IntrusiveListLevel, Level, SingleBlockLevel};
use hi_sparse_array::{SparseArray, SparseArrayLevels};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::sparse_hierarchy::SparseHierarchy;

#[derive(Clone, Debug, Eq, PartialEq)]
struct DataBlock(u64);
impl MaybeEmpty for DataBlock{
    fn empty() -> Self {
        Self(0)
    }

    fn is_empty(&self) -> bool {
        todo!()
    }

    /*fn as_u64_mut(&mut self) -> &mut u64 {
        &mut self.0
    }

    fn restore_empty_u64(&mut self) {
        self.0 = 0;
    }*/
}


#[test]
fn level_depth_test(){
    fn do_test<Levels, DataLevel>(mut array: SparseArray<Levels, DataLevel>, range: Range<usize>)
    where
        DataLevel: ILevel<Block = DataBlock>, 
        Levels: SparseArrayLevels    
    {
        for i in range.clone(){
            *array.get_or_insert(i as usize) = DataBlock(i as u64);
        }
        
        for i in range.clone(){
            let data = unsafe{array.get_unchecked(i)};
            assert_eq!(data, &DataBlock(i as u64));
        }
        
        for i in range.clone(){
            assert!(array.may_contain(i));
        }
        
        /*for (index, data) in CachingBlockIter::new(&array){
            println!("{index}: {:}", data.0);
        }*/
        assert_equal(CachingBlockIter::new(&array).map(|(_, d)|d.0 as usize), range.clone());
        assert_equal(CachingBlockIter::new(&array).map(|(i, _)|i), range.clone());
    }
    
    type Lvl0Block = Block<u64, [u8;64]>;
    type Lvl1Block = Block<u64, [u16;64]>;
    type Lvl2Block = Block<u64, [u32;64]>;
    
    type DataLevel = Level<DataBlock>;
    
    {
        type Array = SparseArray<(SingleBlockLevel<Lvl0Block>, ), DataLevel>;
        do_test(Array::default(), 0..64);
    }
    {
        type Array = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>), DataLevel>;
        do_test(Array::default(), 0..64*64);
    }
    {
        type Array = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>, IntrusiveListLevel<Lvl2Block>), DataLevel>;
        do_test(Array::default(), 0..64*64*64);
    }
}