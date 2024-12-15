use std::alloc::{alloc, dealloc, realloc, Layout};
use std::marker::PhantomData;
use std::{cmp, mem, ptr};
use std::ptr::{addr_of_mut, null, NonNull};
use crate::{BitBlock, HierarchyIndex, ReqDefault};
use crate::const_utils::{const_loop, ArrayOf, ConstInteger, ConstUsize};
use crate::req_default::{MakeDefault, DefaultInitFor, MakeDefaultFor, DefaultRequirement, IsReqDefault};
use crate::utils::Array;

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


#[derive(Debug, Copy, Clone)]
pub enum ChildsType{Blocks, DataBlocks}

struct BlockHeader<T, Conf:Config> {
    mask: Conf::Mask,
    child_indices: ArrayOf<
        u8,
        <Conf::Mask as BitBlock>::Size
    >,
    
    // We can move this into BlockMeta and store in BlockPtr
    len: u8,
    cap: u8,
    childs_type: ChildsType,
    
    phantom_data: PhantomData<T>
}
impl<T, Conf:Config> BlockHeader<T, Conf>{
    #[inline]
    /*const*/ fn layout(child_align: usize) -> Layout {
        unsafe {
            Layout::from_size_align_unchecked(
                size_of::<Self>(),
                cmp::max(child_align, align_of::<Self>())
            )
            .pad_to_align()
        }
    }
    
    #[inline]
    /*const*/ fn children_addr_offset(child_align: usize) -> usize {
        Self::layout(child_align).size()
    }
}

// TODO: Try everything with self instead of &self
struct BlockPtr<T, Conf:Config>(NonNull<BlockHeader<T, Conf>>);

impl<T, Conf:Config> Clone for BlockPtr<T, Conf>{
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}
impl<T, Conf:Config> Copy for BlockPtr<T, Conf>{}

impl<T, Conf:Config> BlockPtr<T, Conf>{
    #[inline]
    fn layout(cap: u8, childs_type: ChildsType) -> Layout {
        let (child_size, child_align) = match childs_type {
            ChildsType::Blocks => (
                size_of::<BlockPtr<T, Conf>>(),
                align_of::<BlockPtr<T, Conf>>()
            ),
            ChildsType::DataBlocks => (
                size_of::<T>(),
                align_of::<T>()
            )
        };
        
        let array_size = child_size * cap as usize;
        let header_layout = BlockHeader::<T, Conf>::layout(child_align); 
        let size = header_layout.size() + array_size;
        
        unsafe {
            Layout::from_size_align_unchecked(
                size,
                header_layout.align()
            ).pad_to_align()
        }        
    }
    
    #[inline]
    pub fn new(childs_type: ChildsType, cap: u8) -> Self {
        let layout = Self::layout(cap, childs_type);
        unsafe{
            let block = alloc(layout) as *mut BlockHeader<T, Conf>;
            
            addr_of_mut!((*block).mask).write(BitBlock::zero());
            addr_of_mut!((*block).child_indices).write_bytes(0, 1);
            addr_of_mut!((*block).len).write(0);
            addr_of_mut!((*block).cap).write(cap);
            addr_of_mut!((*block).childs_type).write(childs_type);
            
            Self(NonNull::new_unchecked(block))
        }
    }
    
    #[inline]
    /*const*/ fn children_ptr(&self, child_align: usize) -> *mut u8 {
        let ptr = self.0.as_ptr() as *mut u8;
        unsafe{
            ptr.add(BlockHeader::<T, Conf>::children_addr_offset(child_align))
        }
    }    
    
    #[inline]
    unsafe fn children_iter_mut<'a, V: 'a>(&'a mut self) 
        -> impl Iterator<Item = &'a mut V>
    {
        let block = self.0.as_ref(); 
        block.mask.clone().into_bits_iter()
            .map(move |i| unsafe {
                let i = *block.child_indices.as_ref().get_unchecked(i) as usize;
                &mut *self.children_ptr(align_of::<V>()).cast::<V>().add(i)
            })
    }    
    
    #[inline]
    unsafe fn push_within_capacity_unchecked<V>(&mut self, value: V) -> *mut V {
        let block = self.0.as_mut();
        debug_assert!(block.len < block.cap);
        let ptr: *mut V = self.children_ptr(align_of::<V>()).cast::<V>()
                         .add(block.len as usize);
        ptr.write(value);
        block.len += 1;
        ptr        
    }    
    
    #[inline]
    unsafe fn push_unchecked<V>(&mut self, value: V) -> *mut V {
        let block = self.0.as_ref();
        if block.len == block.cap {
            let new_capacity = block.cap * 2;
            let new_ptr= realloc(
                self.0.as_ptr() as *mut u8,
                // TODO: childs_type can be compiletime
                Self::layout(block.cap, block.childs_type),
                Self::layout(new_capacity, block.childs_type).size(),
            ) as *mut BlockHeader<T, Conf>;
            (*new_ptr).cap = new_capacity; 
            self.0 = NonNull::new_unchecked(new_ptr);
        }
        self.push_within_capacity_unchecked(value)
    }
    
    /// Get or insert block or data. 
    #[inline]
    unsafe fn insert_impl<V, Child: FnOnce() -> V>(
        &mut self, 
        index: usize, 
        child: Child
    ) -> Result<&mut V, Child> {
        let block = self.0.as_mut();
        // TODO: try read first
        let have_child = unsafe {
            block.mask.set_bit_unchecked::<true>(index)    
        };
        if have_child {
            return Err(child);
        }
        
        let child_index = block.len;
        
        let child = self.push_unchecked(child());
        
        let block = self.0.as_mut();
        *block.child_indices.as_mut().get_unchecked_mut(index) = child_index; 
        
        Ok(&mut*child)
    }    
    
    #[inline]
    pub unsafe fn get_or_insert_unchecked<V, Child: FnOnce() -> V>(
        &mut self, 
        index: usize, 
        child: Child
    ) -> &mut V {
        // Drop lifetime to fight RUST's not working an early-return lifetime drop.
        let mut this = NonNull::from(self);
        if let Ok(child) = this.as_mut().insert_impl(index, child){
            return child;
        }
        &mut*this.as_mut().get_unchecked_ptr::<V>(index)
    }
    
    #[inline]
    pub unsafe fn insert_unchecked<V>(
        &mut self, 
        index: usize, 
        child: V
    ) {
        if let Err(child) = self.insert_impl(index, ||child){
            *self.get_unchecked_ptr::<V>(index) = child();
        }
    }
    
    /// For both mut and const scenarios.
    #[inline]
    pub unsafe fn get_unchecked_ptr<V> (
        &self, 
        index: usize,
    ) -> *mut V {
        let block = self.0.as_ref();
        let i = *block.child_indices.as_ref().get_unchecked(index);
        self.children_ptr(align_of::<V>()).cast::<V>().add(i as usize)
    }

    #[inline]
    pub unsafe fn have_child_unchecked(&self, index: usize) -> bool {
        let block = self.0.as_ref();
        block.mask.get_bit_unchecked(index)
    }
    
    /// Destructs each first child in "empty branch".
    pub unsafe fn destruct_empty_block<R: DefaultRequirement>(&mut self){
        let block = self.0.as_ref();
        
        // TODO: this switch can be compiletime
        // 1. destruct children
        match block.childs_type{
            ChildsType::Blocks => {
                let mut child = &mut *self.children_ptr(align_of::<BlockPtr<T, Conf>>()).cast::<BlockPtr<T, Conf>>();
                child.destruct_empty_block::<R>();
                ptr::drop_in_place(child);
            }
            ChildsType::DataBlocks => {
                if R::REQUIRED {
                    let child = &mut *self.children_ptr(align_of::<T>()).cast::<T>();
                    ptr::drop_in_place(child);
                }
            }
        }
        
        // 2. Deallocate self
        let layout = Self::layout(block.cap, block.childs_type);
        dealloc(self.0.as_ptr().cast(), layout);
    }
    
    pub unsafe fn destruct(&mut self){
        let block = self.0.as_ref();
        
        // TODO: this switch can be compiletime
        // 1. destruct children
        match block.childs_type{
            ChildsType::Blocks => {
                let mut iter = self.children_iter_mut::<BlockPtr<T, Conf>>();
                for child in iter {
                    child.destruct();
                    ptr::drop_in_place(child);
                }
            }
            ChildsType::DataBlocks => {
                let mut iter = self.children_iter_mut::<T>();
                for child in iter {
                    ptr::drop_in_place(child);
                }
            }
        }
        
        // 2. Deallocate self
        let layout = Self::layout(block.cap, block.childs_type);
        dealloc(self.0.as_ptr().cast(), layout);         
    }
}

type EmptyBranchBlocks<T, Conf: Config> = ArrayOf<BlockPtr<T, Conf>, /*<*/Conf::LevelCount/* as ConstInteger>::Inc*/>;


pub struct Tree<T, Conf:Config, R: DefaultRequirement = ReqDefault<false>>{
    root: BlockPtr<T, Conf>,
    
    // TODO: root level empty block never used - remove?
    /// Sequence of empty blocks with child at pos 0.
    /// This lets us have branchless get().
    empty_branch_blocks: EmptyBranchBlocks<T, Conf>,
    
    phantom_data: PhantomData<R>
}

impl<T, Conf:Config, R: DefaultRequirement> Tree<T, Conf, R>
where
    MakeDefaultFor<T, R>: MakeDefault<T>
{
    pub fn new() -> Self{
        // construct empty branch
        let empty_branch_blocks: EmptyBranchBlocks<T, Conf> = {
            let mut empty_branch_blocks = EmptyBranchBlocks::<T, Conf>::uninit_array();
            // in reverse order - from terminal node to the root.
            let mut block = BlockPtr::new(ChildsType::DataBlocks, 1);
            if R::REQUIRED {
                unsafe {
                    block.push_within_capacity_unchecked(
                        <MakeDefaultFor<T, R> as MakeDefault<T>>::make_default()
                    );
                }
            }
            empty_branch_blocks.as_mut()[Conf::LevelCount::VALUE-1].write(block);
            for I in (0..Conf::LevelCount::VALUE-1).rev() {
                let mut new_block = BlockPtr::new(ChildsType::Blocks, 1);
                unsafe{
                    new_block.push_within_capacity_unchecked(block);
                }
                block = new_block;
                empty_branch_blocks.as_mut()[I].write(block);
            }
            unsafe{ Array::assume_init_array(empty_branch_blocks) }
        };
        
        let mut root = BlockPtr::new(ChildsType::Blocks, 2 );
        unsafe{
            if <Conf::LevelCount as ConstInteger>::VALUE == 1 {
                if R::REQUIRED {
                    root.push_within_capacity_unchecked(
                        <MakeDefaultFor<T, R> as MakeDefault<T>>::make_default()
                    );
                }                
            } else {            
                root.push_within_capacity_unchecked(empty_branch_blocks.as_ref()[1]);
            }
        }
        
        Self{
            root,
            empty_branch_blocks,
            phantom_data: PhantomData,
        }
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
                                let mut block = BlockPtr::new(ChildsType::DataBlocks, 2);
                                unsafe {
                                    block.push_within_capacity_unchecked(
                                        <MakeDefaultFor<T, R> as MakeDefault<T>>::make_default()
                                    );
                                }
                                block
                            } else {
                                BlockPtr::new(ChildsType::DataBlocks, 1)    
                            }
                        } else {
                            let mut block = BlockPtr::new(ChildsType::Blocks, 2);
                            block.push_within_capacity_unchecked(self.empty_branch_blocks.as_ref()[I+1]);
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
    fn get_impl(&self, index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>) -> Option<*mut T> {
        let index = index.into();
        let mut block = self.root;
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe{ *block.get_unchecked_ptr(child_index) };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        unsafe{
            if block.have_child_unchecked(child_index) {
                Some( block.get_unchecked_ptr(child_index) )
            } else {
                None
            }
        }
    }    
    
    #[inline]
    pub fn get_mut(&mut self, index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>) -> Option<&mut T> {
        self.get_impl(index).map(|v| unsafe{ &mut *v })
    }
    
    #[inline]
    pub fn get(&self, index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>) -> Option<&T> {
        self.get_impl(index).map(|v| unsafe{ &*v })
    }    
    
    #[inline]
    pub fn get_or_default(
        &self, 
        index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>
    ) -> &T
    where
        R: IsReqDefault
    {
        unsafe{ self.get(index).unwrap_unchecked() }
    }    
    
}
impl<T, Conf: Config, R: DefaultRequirement> Drop for Tree<T, Conf, R> {
    fn drop(&mut self) {
        unsafe{
            self.root.destruct();
            self.empty_branch_blocks.as_mut()[0].destruct_empty_block::<R>();
        }
    }
}

#[cfg(test)]
mod test{
    use super::*;
    
    #[test]
    fn smoke_test(){
        let mut tree: Tree<usize, Config64bit<2>> = Tree::new();
        tree.insert(0, 0);
        tree.insert(0, 0);
        assert_eq!(tree.get_mut(0), Some(&mut 0));
        assert_eq!(tree.get_mut(4000), None);
        
    }
}