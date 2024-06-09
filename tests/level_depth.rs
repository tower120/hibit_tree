use std::ops::Range;
use itertools::assert_equal;
use hi_sparse_array::level_block::Block;
use hi_sparse_array::level::{ILevel, IntrusiveListLevel, SingleBlockLevel};
use hi_sparse_array::{Empty, SparseArray, SparseArrayLevels};
use hi_sparse_array::caching_iter::CachingBlockIter;
use hi_sparse_array::SparseHierarchy;

#[derive(Clone, Debug, Eq, PartialEq)]
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
fn level_depth_test(){
    fn do_test<Levels>(mut array: SparseArray<Levels, DataBlock>, range: Range<usize>)
    where
        Levels: SparseArrayLevels
    {
        for i in range.clone(){
            *array.get_mut(i as usize) = DataBlock(i as u64);
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
    
    {
        type Array = SparseArray<(SingleBlockLevel<Lvl0Block>, ), DataBlock>;
        do_test(Array::default(), 0..64);
    }
    {
        type Array = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>), DataBlock>;
        do_test(Array::default(), 0..64*64);
    }
    {
        type Array = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>, IntrusiveListLevel<Lvl2Block>), DataBlock>;
        do_test(Array::default(), 0..64*64*64);
    }
}