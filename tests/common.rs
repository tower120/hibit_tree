/*use hi_sparse_array::level::{IntrusiveListLevel, SingleBlockLevel};
use hi_sparse_array::level_block::{Block, SmallBlock};
use hi_sparse_array::SparseArray;

type Lvl0Block = Block<u64, [u8; 64]>;
type Lvl1Block = Block<u64, [u16; 64]>;
type Lvl2Block = Block<u64, [u32; 64]>;
type Lvls = (SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>, IntrusiveListLevel<Lvl2Block>);

type CompactLvl1Block = SmallBlock<u64, [u8;1], [u16;64], [u16;7]>;
type CompactLvl2Block = SmallBlock<u64, [u8;1], [u32;64], [u32;7]>;
type CompactLvls = (
    SingleBlockLevel<Lvl0Block>, 
    IntrusiveListLevel<CompactLvl1Block>, 
    IntrusiveListLevel<CompactLvl2Block>
);*/

use hi_sparse_array::{config, SparseArray};

// TODO: switch to Compact on flag for CI. 
pub type Array<Data> = SparseArray<config::width_64::depth_3, Data>;
pub const RANGE: usize = 260_000;