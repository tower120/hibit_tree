use std::marker::PhantomData;
use std::ptr::{null, null_mut, NonNull};
use wide::{u64x2, u64x4};
use crate::{BitBlock, HierarchyIndex, ReqDefault};
use crate::const_utils::{const_loop, ArrayOf, ConstArrayType, ConstInteger, ConstUsize};
use crate::req_default::{DefaultInit, DefaultInitFor, DefaultRequirement, IsReqDefault};
use crate::utils::{Array, RefLt};

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


// TODO: Use special kind of MiniVec instead of Vec.
//       We can use u8 or u16 for len/cap as well.
enum Childs<T, Conf: Config, R>{
    Blocks(Vec<Block<T, Conf, R>>),
    DataBlocks(Vec<T>),
    //HiDataBlocks(Vec<Conf::Data>),
}
impl<T, Conf: Config, R: DefaultRequirement> Childs<T, Conf, R>{
    #[inline]
    unsafe fn get_unchecked_mut<K, const IS_BLOCK: bool>(&mut self) -> &mut Vec<K>{
        let childs: NonNull<u8> = match self {
            Childs::Blocks(childs) => {
                if IS_BLOCK {
                    NonNull::from(childs).cast()
                } else {
                    unsafe{ std::hint::unreachable_unchecked() }    
                }
            },
            Childs::DataBlocks(childs) => {
                if !IS_BLOCK {
                    NonNull::from(childs).cast()
                } else {
                    unsafe{ std::hint::unreachable_unchecked() }    
                }
            },
        };
        childs.cast().as_mut()        
    }
    
    #[inline]
    unsafe fn get_unchecked<K, const IS_BLOCK: bool>(&self) -> &Vec<K>{
        let childs: NonNull<u8> = match self {
            Childs::Blocks(childs) => {
                if IS_BLOCK {
                    NonNull::from(childs).cast()
                } else {
                    unsafe{ std::hint::unreachable_unchecked() }    
                }
            },
            Childs::DataBlocks(childs) => {
                if !IS_BLOCK {
                    NonNull::from(childs).cast()
                } else {
                    unsafe{ std::hint::unreachable_unchecked() }    
                }
            },
        };
        childs.cast().as_ref()
    }
    
    // TODO: this have runtime cost
    #[inline]
    fn as_bytes_ptr(&self) -> *const u8 {
        match self{
            Childs::Blocks(vec) => vec.as_ptr().cast(),
            Childs::DataBlocks(vec) => vec.as_ptr().cast()
        }
    }
}

/// First(0) in child array - is always empty. (conditionally for data)
#[repr(C)]  // We want `childs` to be near `mask` - so they be in one cache line.
struct Block<T, Conf: Config, R>{
    mask: Conf::Mask,
    childs: Childs<T, Conf, R>,
    child_indices: ArrayOf<
        u8,
        <Conf::Mask as BitBlock>::Size
    >,
    pub destruct_childs: bool,  // TODO: This can be done in compile-time fashion instead.
    phantom_data: PhantomData<R>
}
impl<T, Conf: Config, R: DefaultRequirement> Block<T, Conf, R>{
    pub fn new(childs: Childs<T, Conf, R>) -> Self {
        Self{
            mask: BitBlock::zero(),
            childs,
            child_indices: Array::from_fn(|_|0),
            destruct_childs: true,
            phantom_data: Default::default(),
        }
    }
    
    #[inline(always)]
    pub unsafe fn try_get_unchecked<BLOCK_TYPE, const IS_BLOCK: bool>(
        &self, 
        index: usize,
    ) -> Option<&BLOCK_TYPE> {
        let i = *self.child_indices.as_ref().get_unchecked(index);
        if i == 0 {
            return None;
        }

        let childs = self.childs.get_unchecked::<BLOCK_TYPE, IS_BLOCK>();
        Some(childs.get_unchecked(i as usize).into())
    }
    
    #[inline]
    pub unsafe fn get_unchecked<BLOCK_TYPE, const IS_BLOCK: bool>(
        &self, 
        index: usize,
    ) -> &BLOCK_TYPE {
        self.try_get_unchecked::<BLOCK_TYPE, IS_BLOCK>(index).unwrap_unchecked()
        /*let i = *self.child_indices.as_ref().get_unchecked(index);
        let childs = self.childs.get_unchecked::<BLOCK_TYPE, IS_BLOCK>();
        childs.get_unchecked(i as usize).into()*/
    }
    
    #[inline]
    pub unsafe fn get_unchecked_mut<BLOCK_TYPE, const IS_BLOCK: bool>(
        &mut self, 
        index: usize,
    ) -> &mut BLOCK_TYPE {
        //self.try_get_unchecked_mut::<BLOCK_TYPE, IS_BLOCK>(index).unwrap_unchecked()
        let i = *self.child_indices.as_ref().get_unchecked(index);
        let childs = self.childs.get_unchecked_mut::<BLOCK_TYPE, IS_BLOCK>();
        childs.get_unchecked_mut(i as usize).into()
    }
    
    #[inline]
    pub unsafe fn have_child_unchecked(&self, index: usize) -> bool {
        self.mask.get_bit_unchecked(index)
    }
    
    /// Get or insert block or data. 
    #[inline]
    pub unsafe fn get_or_insert_unchecked<BLOCK_TYPE, const IS_BLOCK: bool>(
        &mut self, 
        index: usize, 
        child_block: impl FnOnce() -> BLOCK_TYPE
    ) -> &mut BLOCK_TYPE {
        // TODO: try read first
        let have_child = unsafe {
            self.mask.set_bit_unchecked::<true>(index)    
        };
        if have_child {
            return self.get_unchecked_mut::<BLOCK_TYPE, IS_BLOCK>(index);
        }
        
        let childs = self.childs.get_unchecked_mut::<BLOCK_TYPE, IS_BLOCK>();
        let child_index = childs.len(); 
        childs.push(child_block());
        
        *self.child_indices.as_mut().get_unchecked_mut(index) = child_index as u8; 
        
        unsafe{ childs.last_mut().unwrap_unchecked() }
    }
    
    // TODO: used only for Data? 
    #[inline]
    pub unsafe fn insert_unchecked<BLOCK_TYPE, const IS_BLOCK: bool>(
        &mut self, 
        index: usize, 
        value: BLOCK_TYPE
    ) {
        // TODO: try read first
        let have_child = unsafe {
            self.mask.set_bit_unchecked::<true>(index)    
        };
        if have_child {
            *self.get_unchecked_mut::<BLOCK_TYPE, IS_BLOCK>(index) = value;
            return;
        }
        
        let childs = self.childs.get_unchecked_mut::<BLOCK_TYPE, IS_BLOCK>();
        let child_index = childs.len(); 
        childs.push(value);
        *self.child_indices.as_mut().get_unchecked_mut(index) = child_index as u8; 
    }
}

impl<T, Conf: Config, R> Drop for Block<T, Conf, R>{
    #[inline]
    fn drop(&mut self) {
        if !self.destruct_childs {
            match &mut self.childs{
                Childs::Blocks(vec) => {
                    std::mem::take(vec).leak();
                }
                Childs::DataBlocks(vec) => {
                    std::mem::take(vec).leak();
                }
            };
        }
    }
} 

type EmptyBranchBlockChilds<Conf: Config> = ArrayOf<*const u8, /*<*/Conf::LevelCount/* as ConstInteger>::Inc*/>;

pub struct Tree<T, Conf: Config, R: DefaultRequirement = ReqDefault<false>> {
    root: Block<T, Conf, R>,
    
    /// Sequence of empty blocks with child at pos 0.
    /// This lets us have branchless get().
    empty_branch: Block<T, Conf, R>,
    /// Pointers to empty_branch Block's childs.
    empty_branch_block_childs: EmptyBranchBlockChilds<Conf>,
    phantom_data: PhantomData<R>,
}
impl<T, Conf: Config, R: DefaultRequirement> Tree<T, Conf, R>
where
    DefaultInitFor<T, R>: DefaultInit
{
    #[inline]
    pub fn make_empty_block<const I: usize>(empty_branch_block_childs: &EmptyBranchBlockChilds<Conf>) -> Block<T, Conf, R> {
        let ptr = empty_branch_block_childs.as_ref()[I];
        let mut block = 
            if I == Conf::LevelCount::VALUE-1{
                let vec = unsafe{Vec::from_raw_parts(ptr as *mut _, 1, 1)};
                Block::new(Childs::DataBlocks(vec))    
            } else {
                let vec = unsafe{Vec::from_raw_parts(ptr as *mut _, 1, 1)};
                Block::new(Childs::Blocks(vec))
            };
        block.destruct_childs = false;
        block
    }
    
    pub fn new() -> Self {
        let mut empty_branch_block_childs: EmptyBranchBlockChilds<Conf> = Array::from_fn(|_|null());
        // construct empty branch
        let empty_branch = {
            // in reverse order - from terminal node to the root.
            let mut vec: Vec<T> = Vec::with_capacity(1);
            if R::REQUIRED {
                unsafe {
                    <DefaultInitFor<T, R> as DefaultInit>::init_default(vec.as_mut_ptr().cast());
                    vec.set_len(1);    
                }
            }
            let mut block = Block::new(Childs::DataBlocks(vec));
            empty_branch_block_childs.as_mut()[Conf::LevelCount::VALUE-1] = block.childs.as_bytes_ptr();
            for I in (0..Conf::LevelCount::VALUE-1).rev() {
                block = Block::new(Childs::Blocks(vec![block]));
                empty_branch_block_childs.as_mut()[I] = block.childs.as_bytes_ptr();
            }
            block
        };
        
        let mut this = Self {
            root: Block::new(Childs::Blocks(Vec::with_capacity(2))),
            empty_branch,
            empty_branch_block_childs,
            phantom_data: PhantomData,
        };
        
        let empty_child = Self::make_empty_block::<1>(&this.empty_branch_block_childs);
        match &mut this.root.childs{
            Childs::Blocks(vec) => vec.push(empty_child), 
            _ => unsafe{ std::hint::unreachable_unchecked() }
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
                block.get_or_insert_unchecked::<_, true>(
                    child_index, 
                    ||{
                        if I == Conf::LevelCount::VALUE-2 {
                            let vec = if R::REQUIRED {
                                unsafe {
                                    let mut vec: Vec<T> = Vec::with_capacity(2);
                                    <DefaultInitFor<T, R> as DefaultInit>::init_default(vec.as_mut_ptr().cast());
                                    vec.set_len(1);    
                                    vec
                                }
                            } else {
                                Vec::with_capacity(1)
                            };
                            Block::new(Childs::DataBlocks(vec))
                        } else {
                            let empty_child = Self::make_empty_block::<{I+1}>(&self.empty_branch_block_childs);
                            Block::new(Childs::Blocks(vec![empty_child]))
                        }
                    }
                )
            };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1]; 
        unsafe{ block.insert_unchecked::<T, false>(child_index, value); }
    }
    
    #[inline]
    pub fn get_mut(&mut self, index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>) -> Option<&mut T> {
        let index = index.into();
        let mut block = &mut self.root;
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe {
                block.get_unchecked_mut::<_, true>(child_index as _)
            };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        unsafe{
            if block.have_child_unchecked(child_index as _) {
                Some(block.get_unchecked_mut::<_, false>(child_index as _))
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
                block.get_unchecked::<_, true>(child_index as _)
            };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        unsafe{
            if block.have_child_unchecked(child_index) {
                Some(block.get_unchecked::<_, false>(child_index))
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
    use crate::tree::{Config64bit, Tree};

    #[test]
    fn smoke_test(){
        let mut tree: Tree<usize, Config64bit<3>> = Tree::new();
        tree.insert(1, 1);
        tree.insert(1, 1);
        assert_eq!(tree.get_mut(1), Some(&mut 1));
        assert_eq!(tree.get_mut(0), None);
        assert_eq!(tree.get_mut(4000), None);
    }
}