mod block;

use std::marker::PhantomData;
use std::ptr::{null, null_mut, NonNull};
use wide::{u64x2, u64x4};
use crate::{BitBlock, HierarchyIndex, ReqDefault};
use crate::const_utils::{const_loop, ArrayOf, ConstArrayType, ConstInteger, ConstUsize};
use crate::req_default::{DefaultInit, DefaultInitFor, DefaultRequirement, IsReqDefault};
use crate::utils::{Array, RefLt};
use block::Block;
use block::ChildsType;
use crate::tree::block::childs;

pub trait Config {
    type Mask: BitBlock;
    type LevelCount: ConstInteger;
}

pub struct Config64bit<const LEVELS: usize>;
impl<const LEVELS: usize> Config for Config64bit<LEVELS>
where
    ConstUsize<LEVELS>: ConstInteger,
{
    type Mask = u64;
    type LevelCount = ConstUsize<LEVELS>;
}

pub struct Config128bit<const LEVELS: usize>;
impl<const LEVELS: usize> Config for Config128bit<LEVELS>
where
    ConstUsize<LEVELS>: ConstInteger,
{
    type Mask = u64x2;
    type LevelCount = ConstUsize<LEVELS>;
}

// TODO: 256 is not safe to use now, since we allocate one element as 
//       empty in each block an duse u8 for indexing. 
//       Hence we can hold only 255 elements. 
pub struct Config256bit<const LEVELS: usize>;
impl<const LEVELS: usize> Config for Config256bit<LEVELS>
where
    ConstUsize<LEVELS>: ConstInteger,
{
    type Mask = u64x4;
    type LevelCount = ConstUsize<LEVELS>;
}

type EmptyBranchBlockChilds<Conf: Config> = ArrayOf<*const u8, /*<*/Conf::LevelCount/* as ConstInteger>::Inc*/>;

pub struct Tree<T, Conf: Config, R: DefaultRequirement = ReqDefault<false>> {
    root: Block<T, Conf>,
    
    /// Sequence of empty blocks with child at pos 0.
    /// This lets us have branchless get().
    empty_branch: Block<T, Conf>,
    /// Pointers to empty_branch Block's childs.
    empty_branch_block_childs: EmptyBranchBlockChilds<Conf>,
    phantom_data: PhantomData<R>,
}
impl<T, Conf: Config, R: DefaultRequirement> Tree<T, Conf, R>
where
    DefaultInitFor<T, R>: DefaultInit
{
    #[inline]
    pub fn make_empty_block<const I: usize>(empty_branch_block_childs: &EmptyBranchBlockChilds<Conf>) -> Block<T, Conf> {
        let childs_type = if I == Conf::LevelCount::VALUE-1{
            ChildsType::DataBlocks
        } else {
            ChildsType::Blocks
        };
        let ptr = empty_branch_block_childs.as_ref()[I];
        let mut block = Block::from_parts(childs_type, ptr as *mut _, 1, 1);
        block.destruct_childs = false;
        block
    }
    
    pub fn new() -> Self {
        let mut empty_branch_block_childs: EmptyBranchBlockChilds<Conf> = Array::from_fn(|_|null());
        // construct empty branch
        let empty_branch = {
            // in reverse order - from terminal node to the root.
            let mut block = Block::with_capacity(ChildsType::DataBlocks, 1);
            if R::REQUIRED {
                unsafe {
                    <DefaultInitFor<T, R> as DefaultInit>::init_default(childs::as_mut_ptr(&mut block));
                    childs::set_len(&mut block, 1);
                }
            }
            empty_branch_block_childs.as_mut()[Conf::LevelCount::VALUE-1] = childs::as_ptr(&block);
            for I in (0..Conf::LevelCount::VALUE-1).rev() {
                let mut new_block = Block::with_capacity(ChildsType::Blocks, 1);
                unsafe{
                    childs::push_within_capacity_unchecked(&mut new_block, block);
                }
                block = new_block;
                empty_branch_block_childs.as_mut()[I] = childs::as_ptr(&block);
            }
            block
        };
        
        let mut this = Self {
            root: Block::with_capacity(ChildsType::Blocks, 2),
            empty_branch,
            empty_branch_block_childs,
            phantom_data: PhantomData,
        };
        
        let empty_child = Self::make_empty_block::<1>(&this.empty_branch_block_childs);
        unsafe{
            childs::push_within_capacity_unchecked(&mut this.root, empty_child);    
        }
        this
    }
    
    #[inline]
    pub fn insert(
        &mut self,
        index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>,
        value: T
    ) {
        let index = index.into();
        let mut block = &mut self.root;
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe {
                block.get_or_insert_unchecked(
                    child_index, 
                    ||{
                        if I == Conf::LevelCount::VALUE-2 {
                            if R::REQUIRED {
                                let mut block = Block::with_capacity(ChildsType::DataBlocks, 2);
                                <DefaultInitFor<T, R> as DefaultInit>::init_default(childs::as_mut_ptr(&mut block).cast());
                                childs::set_len(&mut block, 1);
                                block
                            } else {
                                Block::with_capacity(ChildsType::DataBlocks, 1)
                            }
                        } else {
                            let empty_child = Self::make_empty_block::<{I+1}>(&self.empty_branch_block_childs);
                            let mut block = Block::with_capacity(ChildsType::Blocks, 2);
                            childs::push_within_capacity_unchecked(&mut block, empty_child);
                            block
                        }
                    }
                )
            };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1]; 
        unsafe{ block.insert_unchecked(child_index, value); }
    }
    
    #[inline]
    pub fn get_mut(&mut self, index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>) -> Option<&mut T> {
        let index = index.into();
        let mut block = &mut self.root;
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe {
                block.get_unchecked_mut(child_index)
            };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        unsafe{
            if block.have_child_unchecked(child_index) {
                Some(block.get_unchecked_mut(child_index))
            } else {
                None
            }
        }
    }
    
    #[inline]
    pub fn get(&self, index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>) -> Option<&T> {
        let index = index.into();
        let mut block = &self.root;
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe {
                block.get_unchecked(child_index)
            };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        unsafe{
            if block.have_child_unchecked(child_index) {
                Some(block.get_unchecked(child_index))
            } else {
                None
            }
        }
    }    
    
    #[inline]
    pub unsafe fn get_unchecked(
        &self, 
        index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>
    ) -> &T {
        unsafe{ self.get(index).unwrap_unchecked() }
        /*let index = index.into();
        let mut block = &self.root;
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe {
                block.get_unchecked::<_, true>(child_index as _)
            };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        unsafe{
            block.get_unchecked::<_, false>(child_index as _)
        }        */
    }
    
    #[inline]
    pub fn get_or_default(
        &self, 
        index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>
    ) -> &T
    where
        R: IsReqDefault
    {
        unsafe{ self.get_unchecked(index) }
    }
}

#[cfg(test)]
mod test{
    use ahash::{HashMap, HashSet};
    use rand::{Rng, SeedableRng};
    use crate::tree::{Config64bit, Tree};

    #[test]
    fn smoke_test(){
        let mut tree: Tree<usize, Config64bit<3>> = Tree::new();
        tree.insert(1, 1);
        tree.insert(1, 1);
        
        assert_eq!(tree.get/*_mut*/(1), Some(&/*mut*/ 1));
        assert_eq!(tree.get/*_mut*/(0), None);
        assert_eq!(tree.get/*_mut*/(4000), None);
    }
    
    #[test]
    fn fuzzy_read_test(){
        const REPEATS: usize = 100;
        const RANGE  : usize = 10000;
        const MAX_INSERTS: usize = 10000;
        const MAX_READS  : usize = 10000;
        
        let mut rng = rand::rngs::StdRng::seed_from_u64(0xe15bb9db3dee3a0f);
        
        for _ in 0..REPEATS {
            let mut array: Tree<usize, Config64bit<3>> = Tree::new();
            let mut set  : HashMap<usize, usize> = Default::default();
            for _ in 0..rng.gen_range(0..MAX_INSERTS){
                let v = rng.gen_range(0..RANGE);
                array.insert(v, v);
                set.insert(v, v);
            }
            
            // random read
            for _ in 0..rng.gen_range(0..MAX_READS){
                let i = rng.gen_range(0..RANGE);
                let a = array.get(i); 
                let s = set.get(&i);
                assert_eq!(a,s);
            }
            
            // read existent
            for (i, v) in set {
                let a = array.get(i);
                assert_eq!(a, Some(&v));
            }
        }
    }
}