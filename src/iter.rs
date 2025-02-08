use crate::hibit_tree::{HibitTree, HibitTreeCursor};
use crate::{BitBlock, data_block_index, RegularHibitTree, HibitTreeCursorTypes, HibitTreeTypes};
use crate::bit_queue::BitQueue;
use crate::const_utils::{ConstInteger, ConstUsize, const_loop};
use crate::const_utils::ConstArrayType;
use crate::utils::LendingIterator;
use crate::utils::Array;

/// [usize; T::LevelCount::N - 1]
type LevelIndices<T> =
    ConstArrayType<
        usize,
        <<T as HibitTree>::LevelCount as ConstInteger>::Dec   
    >;

/// Each hierarchy level has its own bitblock iterator.
/// 
/// [T::LevelMaskType::BitsIter; T::LevelCount]
type LevelIterators<T> =
    ConstArrayType<
        <<T as HibitTree>::LevelMask as BitBlock>::BitsIter,
        <T as HibitTree>::LevelCount
    >;

/// [HibitTree] iterator.
///  
/// This is [LendingIterator], that also [Iterator] for [RegularHibitTree]. 
pub struct Iter<'a, T>
where
    T: HibitTree,
{
    container: &'a T,
    
    /// [T::LevelMaskType::BitsIter; T::LevelCount]
    level_iters: LevelIterators<T>,
    
    /// [usize; T::LevelCount - 1]
    level_indices: LevelIndices<T>,

    cursor: <T as HibitTreeTypes<'a>>::Cursor,
}

impl<'a, T> Iter<'a, T>
where
    T: HibitTree,
{
    #[inline]
    pub fn new(container: &'a T) -> Self {
        let mut level_iters: LevelIterators<T> = Array::from_fn(|_| BitQueue::empty());
        
        let mut cursor = T::Cursor::new(container);
        
        let root_mask = unsafe{
            cursor.select_level_node_unchecked(container, ConstUsize::<0>, 0)
        };
        let level0_iter = root_mask.into_bits_iter();
        
        level_iters.as_mut()[0] = level0_iter; 
        
        Self{
            container,
            level_iters,
            
            // TODO: refactor this
            // usize::MAX - is marker, that we're in "intial state".
            // Which means that only level0_iter initialized, and in original state.
            level_indices: Array::from_fn(|_| usize::MAX),

            cursor,
        }
    }
}

impl<'a, T> LendingIterator for Iter<'a, T>
where
    T: HibitTree,
{
    type Item<'this>= (
        usize/*index*/, 
        <<T as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'this>>::DataUnchecked
    ) where Self:'this;

    #[inline]
    fn next(&mut self) -> Option<Self::Item<'_>> {
        let level_index =
        'out: loop {
            // We're driven by the terminal-level iterator.
            let last_level_iter = self.level_iters.as_mut().last_mut().unwrap();
            if let Some(index) = last_level_iter.next() {
                break index;
            } else {
                // climb up the tree and search for the first iterable node.
                const_loop!(I in 0..{T::LevelCount::VALUE-1} rev => {
                    let level_iter = unsafe{
                        self.level_iters.as_mut().get_unchecked_mut(I)
                    };
                    if let Some(index) = level_iter.next(){
                        // 1. update level_index
                        *unsafe{
                            self.level_indices.as_mut().get_unchecked_mut(I)  
                        } = index;
                        
                        // 2. update level_iter from mask
                        const DEPTH: usize = I + 1;
                        unsafe{
                            let level_mask = self.cursor.select_level_node_unchecked(
                                &self.container,
                                ConstUsize::<DEPTH>,
                                index
                            );
                            
                            *self.level_iters.as_mut().get_unchecked_mut(DEPTH) 
                                = level_mask.into_bits_iter();                            
                        }

                        // 3. Go down to the terminal level.
                        const_loop!(N in {I+1}..{T::LevelCount::VALUE-1} => {
                            let level_iter = unsafe{
                                self.level_iters.as_mut().get_unchecked_mut(N)
                            };
                            if let Some(index) = level_iter.next(){
                                // 1. update level_index
                                *unsafe{
                                    self.level_indices.as_mut().get_unchecked_mut(N)  
                                } = index;
                                
                                // 2. update level_iter from mask
                                const DEPTH: usize = N + 1;
                                unsafe{
                                    let level_mask = self.cursor.select_level_node_unchecked(
                                        &self.container,
                                        ConstUsize::<DEPTH>,
                                        index
                                    );
                                    
                                    *self.level_iters.as_mut().get_unchecked_mut(DEPTH) 
                                        = level_mask.into_bits_iter();
                                }
                            } else {
                                if T::EXACT_HIERARCHY {
                                    unsafe{ std::hint::unreachable_unchecked(); }
                                } else {
                                    // This branch turns to be empty - repeat search again.
                                    // TODO: do better.
                                    continue 'out;
                                }
                            }
                        });
                        
                        continue 'out;
                    }
                });
                
                // We traversed through whole hierarchy and 
                // root iter have nothing more. 
                return None;
            }
        };

        let data_block = unsafe {
            self.cursor.data_unchecked(&self.container, level_index)
        };
        let block_index = data_block_index::<T::LevelCount, T::LevelMask>(&self.level_indices, level_index);
        Some((block_index, data_block))
    }    
}


impl<'a, T> Iterator for Iter<'a, T>
where
    T: RegularHibitTree,
{
    type Item = (usize, <T as HibitTreeTypes<'a>>::Data);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        LendingIterator::next(self)
    }
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::config::_64bit;
    use crate::{HibitTree, Tree};

    #[test]
    fn deep_tree_iter_test(){
        let mut tree = Tree::<usize, _64bit<6>>::new();
        
        tree.insert(0,0);
        tree.insert(1,1);
        tree.insert(200_000,200_000);
        
        /* let mut iter = tree.iter();
        let v = iter.next();
        println!("{:?}", v);
        return; */
        
        assert_equal(
            tree.iter(),
            [(0, &0), (1, &1), (200_000, &200_000)]
        );
    }    
}