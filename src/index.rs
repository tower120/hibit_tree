use std::marker::PhantomData;
use crate::BitBlock;
use crate::const_utils::{ConstCopyArrayType, ConstInteger, ConstUsize};
use crate::utils::Array;

// TODO: try replace with accumulated key on the fly calculation.
// Compile-time loop inside. Ends up with N ADDs.
#[inline]
pub(crate) fn data_block_index<LevelCount: ConstInteger, LevelMaskType: BitBlock>(
    level_indices: &impl Array<Item=usize>,
    data_index: usize
) -> usize {
    let level_count = LevelCount::VALUE;
    let mut acc = data_index;
    for n in 0..level_count - 1 {
        acc += level_indices.as_ref()[n] << (LevelMaskType::Size::VALUE.ilog2() as usize * (level_count - n - 1));
    }
    acc
}

/// Range checked index, interpreted as a tree path. 
/// 
/// Known to be within `HibitTree<LevelMaskType, LevelCount>::index_range()`.
/// 
/// Whenever you see `impl TryInto<HierarchyIndex<_, _>>` - you can just use your `usize` index
/// as usual.
///  
/// Index range check is very cheap, and is just one assert_eq with constant value.
/// But in tight loops you may want to get rid of that check with [new_unchecked()].
/// Index is also stored in easily digestible form - as an array of in-node indices
/// - that represents tree path. 
/// You can reuse `HierarchyIndex` - this will save a few precious CPU ticks,
/// when you access the same index in different trees.
///
/// ```
/// # use hibit_tree::{HibitTree, HierarchyIndex};
/// # fn example<T: HibitTree>(array: &T, array2: &T){ 
/// // use it just as usize
/// array.get(12);
///
/// // `unsafe` construction without checks.
/// array.get(unsafe{ HierarchyIndex::new_unchecked(12) });
///
/// // safe construct once, then reuse
/// {
///     let i = HierarchyIndex::new(12);
///     array.get(i);
///     array2.get(i);
/// }
/// # }
/// ``` 
#[derive(Clone, Copy)]
pub struct HierarchyIndex<LevelMask: BitBlock, LevelsCount: ConstInteger> {
    pub(crate) index: usize,
    // We could use smaller index size, like u8. 
    // But usize - show best performance so far.
    pub(crate) level_indices: ConstCopyArrayType<usize, LevelsCount>,
    pub(crate) phantom_data : PhantomData<(LevelMask, LevelsCount)>
}

impl<LevelMask: BitBlock, LevelsCount: ConstInteger> 
    HierarchyIndex<LevelMask, LevelsCount>
{
    #[inline]
    pub fn new(index: usize) -> Self {
        Self::try_from(index).unwrap()
    }
    
    #[inline]
    pub unsafe fn new_unchecked(index: usize) -> Self {
        let level_indices = level_indices::<LevelMask, LevelsCount>(index);
        Self{ index, level_indices, phantom_data: PhantomData }
    }
}

impl<LevelMask: BitBlock, LevelsCount: ConstInteger> TryFrom<usize> for
    HierarchyIndex<LevelMask, LevelsCount>
{
    type Error = ();
    
    #[inline]
    fn try_from(index: usize) -> Result<Self, Self::Error> {
        let range_end = LevelMask::Size::VALUE.saturating_pow(LevelsCount::VALUE as _);
        if index >= range_end { 
            return Err(());
        }

        unsafe{ Ok(Self::new_unchecked(index)) }
    }
}

// Compile-time loop inside. Ends up with N (AND + SHR)s.
#[inline]
fn level_indices<LevelMask, LevelsCount>(index: usize)
     -> ConstCopyArrayType<usize, LevelsCount>
where
    LevelMask: BitBlock,
    LevelsCount: ConstInteger,
{
    let mut level_indices = ConstCopyArrayType::<usize, LevelsCount>::from_fn(|_|0);
    
    let mut level_remainder = index;
    let level_count = LevelsCount::VALUE;
    // TODO: const_loop?
    for level in 0..level_count - 1 {
        // LevelMask::SIZE * 2^(level_count - level - 1)
        let level_capacity_exp = LevelMask::Size::VALUE.ilog2() as usize * (level_count - level - 1);
        let level_capacity = 1 << level_capacity_exp;
        
        // level_remainder / level_capacity_exp
        let level_index = level_remainder >> level_capacity_exp;
        
        // level_remainder % level_capacity_exp
        level_remainder = level_remainder & (level_capacity - 1);
        
        level_indices.as_mut()[level] = level_index; 
    }
    
    *level_indices.as_mut().last_mut().unwrap() = level_remainder; 
    
    level_indices
}

#[cfg(test)]
mod test{
    use super::*;
    
    #[test]
    fn test_level_indices_new(){
        {
            let indices = level_indices::<u64, ConstUsize<2>>(65);
            assert_eq!(indices, [1, 1]);
        }
        {
            let lvl0 = 262_144; // Total max capacity
            let lvl1 = 4096;
            let lvl2 = 64;
            let indices = level_indices::<u64, ConstUsize<3>>(lvl1*2 + lvl2*3 + 4);
            assert_eq!(indices, [2, 3, 4]);
        }
        {
            let indices = level_indices::<u64, ConstUsize<3>>(32);
            assert_eq!(indices, [0, 0, 32]);
        }
        {
            let indices = level_indices::<u64, ConstUsize<2>>(32);
            assert_eq!(indices, [0, 32]);
        }    
        {
            let indices = level_indices::<u64, ConstUsize<1>>(32);
            assert_eq!(indices, [32]);
        }
    }
}