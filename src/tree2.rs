use std::alloc::{alloc, dealloc, realloc, Layout};
use std::marker::PhantomData;
use std::{cmp, mem, ptr};
use std::ptr::{addr_of_mut, null, NonNull};
use wide::u64x2;
use crate::{BitBlock, HierarchyIndex, ReqDefault};
use crate::const_utils::{const_loop, ArrayOf, ConstInteger, ConstUsize};
use crate::req_default::{MakeDefault, MakeDefaultFor, DefaultRequirement, IsReqDefault};
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

const FREE_CHILD_INDEX_SENTINEL: u8 = u8::MAX;

#[derive(Debug, Copy, Clone)]
pub enum ChildsType{Blocks, DataBlocks}

struct BlockHeader<T, Conf:Config> {
    mask: Conf::Mask,
    child_indices: ArrayOf<
        u8,
        <Conf::Mask as BitBlock>::Size
    >,
    
    len: u8,
    cap: u8,
    /// FREE_CHILD_INDEX_SENTINEL = NONE
    free_child_index: u8,
    // TODO: remove
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
            addr_of_mut!((*block).free_child_index).write(FREE_CHILD_INDEX_SENTINEL);
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
    unsafe fn children_iter_mut<'a, Child: 'a>(&'a mut self) 
        -> impl Iterator<Item = &'a mut Child>
    {
        let block = self.0.as_ref(); 
        block.mask.clone().into_bits_iter()
            .map(move |i| unsafe {
                let i = *block.child_indices.as_ref().get_unchecked(i) as usize;
                &mut *self.children_ptr(align_of::<Child>()).cast::<Child>().add(i)
            })
    }
    
    #[inline]
    pub unsafe fn write_child_at<Child>(&mut self, value: Child, child_index: usize) -> *mut Child {
        let ptr = self.children_ptr(align_of::<Child>()).cast::<Child>()
                 .add(child_index);
        ptr.write(value);
        ptr
    }
    
    #[inline]
    pub unsafe fn set_len(&mut self, len: u8) {
        let mut block = self.0.as_mut();
        block.len = len;
    }

    /// # Safety
    /// 
    /// * `Child` must match.
    /// * `index` must be within range.
    #[inline]
    unsafe fn push_free_child_index<Child>(&mut self, child_index: usize) {
        let block = self.0.as_mut();
        let prev_root_index = block.free_child_index;
        block.free_child_index = child_index as u8;
        
        let free_child_ptr =  
            self.children_ptr(align_of::<Child>()).cast::<Child>().add(child_index)
            .cast::<u8>();
        
        *free_child_ptr = prev_root_index;
    }
    
    /// # Safety
    /// 
    /// `Child` must match.
    #[inline]
    unsafe fn pop_free_child_index<Child>(&mut self) -> Option<usize> {
        let block = self.0.as_mut();
        let index = block.free_child_index as usize;
        if index == FREE_CHILD_INDEX_SENTINEL as usize {
            return None;
        }
        
        let next_free_child_ptr =  
            self.children_ptr(align_of::<Child>()).cast::<Child>().add(index)
            .cast::<u8>();
        
        block.free_child_index = *next_free_child_ptr;
        Some(index)
    }
    
    #[inline]
    pub unsafe fn remove_unchecked<Child>(&mut self, index: usize) {
        // 0. get child data index, and set it to 0.
        let child_element_index = mem::replace(
            self.0.as_mut().child_indices.as_mut().get_unchecked_mut(index),
            0
        ) as usize;         

        // 1. destruct child
        let child = &mut *self.children_ptr(align_of::<Child>()).cast::<Child>().add(child_element_index);
        ptr::drop_in_place(child);
        
        // 2. mark child's slot as free 
        self.push_free_child_index::<Child>(child_element_index);
        
        let mut block = self.0.as_mut();
        block.mask.set_bit_unchecked::<false>(index);
    }
    
    #[inline]
    pub fn is_empty(&self) -> bool {
        unsafe{ self.0.as_ref().mask.is_zero() }
    }
    
    /// Get or insert block or data. 
    #[inline]
    unsafe fn insert_impl<Child, ChildCtr: FnOnce() -> Child>(
        &mut self, 
        index: usize, 
        child: ChildCtr
    ) -> Result<&mut Child, ChildCtr> {
        let block = self.0.as_mut();
        // TODO: try read first
        let have_child = unsafe {
            block.mask.set_bit_unchecked::<true>(index)    
        };
        if have_child {
            return Err(child);
        }
        
        let child_index = if let Some(child_index) = self.pop_free_child_index::<Child>() {
            child_index
        } else {
            let mut block = self.0.as_mut();
            if block.len == block.cap {
                let new_capacity = block.cap * 2;
                let new_ptr = realloc(
                    self.0.as_ptr() as *mut u8,
                    // TODO: childs_type can be compiletime
                    Self::layout(block.cap, block.childs_type),
                    Self::layout(new_capacity, block.childs_type).size(),
                ) as *mut BlockHeader<T, Conf>;
                (*new_ptr).cap = new_capacity; 
                self.0 = NonNull::new_unchecked(new_ptr);
                block = &mut *new_ptr;
            }          
            
            let child_index = block.len;
            block.len += 1;
            child_index as usize
        };
        let child = self.write_child_at(child(), child_index);
        
        let block = self.0.as_mut();
        *block.child_indices.as_mut().get_unchecked_mut(index) = child_index as u8; 
        
        Ok(&mut*child)
    }    
    
    #[inline]
    pub unsafe fn get_or_insert_unchecked<Child, ChildCtr: FnOnce() -> Child>(
        &mut self, 
        index: usize, 
        child: ChildCtr
    ) -> &mut Child {
        // Drop lifetime to fight RUST's not working an early-return lifetime drop.
        let mut this = NonNull::from(self);
        if let Ok(child) = this.as_mut().insert_impl(index, child){
            return child;
        }
        &mut*this.as_mut().get_unchecked_ptr::<Child>(index)
    }
    
    #[inline]
    pub unsafe fn insert_unchecked<Child>(
        &mut self, 
        index: usize, 
        child: Child
    ) {
        if let Err(child) = self.insert_impl(index, ||child){
            *self.get_unchecked_ptr::<Child>(index) = child();
        }
    }
    
    /// For both mut and const scenarios.
    #[inline]
    pub unsafe fn get_unchecked_ptr<Child> (
        &self, 
        index: usize,
    ) -> *mut Child {
        let block = self.0.as_ref();
        let i = *block.child_indices.as_ref().get_unchecked(index);
        self.children_ptr(align_of::<Child>()).cast::<Child>().add(i as usize)
    }

    #[inline]
    pub unsafe fn have_child_unchecked(&self, index: usize) -> bool {
        let block = self.0.as_ref();
        block.mask.get_bit_unchecked(index)
    }
    
    /// Destructs each first child in "empty branch".
    pub unsafe fn destruct_empty_branch<R: DefaultRequirement>(&mut self){
        let block = self.0.as_ref();
        
        // TODO: this switch can be compiletime
        // 1. destruct children
        match block.childs_type{
            ChildsType::Blocks => {
                let mut child = &mut *self.children_ptr(align_of::<BlockPtr<T, Conf>>()).cast::<BlockPtr<T, Conf>>();
                child.destruct_empty_branch::<R>();
                ptr::drop_in_place(child);
            }
            ChildsType::DataBlocks => {
                if R::REQUIRED {
                    let child = &mut *self.children_ptr(align_of::<T>()).cast::<T>();
                    ptr::drop_in_place(child);
                }
            }
        }
        
        // 2. Destruct empty block 
        self.destruct_empty();
    }
    
    /// Destruct block and it's children.
    pub unsafe fn destruct(&mut self){
        let block = self.0.as_mut();
        
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
        
        // 2. Destruct empty block 
        self.destruct_empty();
    }
    
    /// Destruct block without children.
    #[inline]
    pub unsafe fn destruct_empty(&mut self) {
        let block = self.0.as_mut();
        let layout = Self::layout(block.cap, block.childs_type);
        ptr::drop_in_place(block);
        dealloc(self.0.as_ptr().cast(), layout);
    }
}

#[test]
fn block_free_inidces_test(){
    let mut block: BlockPtr<usize, Config64bit<2>> = BlockPtr::new(ChildsType::DataBlocks, 16);
    unsafe{
        block.push_free_child_index::<usize>(2);
        block.push_free_child_index::<usize>(4);
        block.push_free_child_index::<usize>(8);
        
        assert_eq!(block.pop_free_child_index::<usize>(), Some(8));
        assert_eq!(block.pop_free_child_index::<usize>(), Some(4));
        assert_eq!(block.pop_free_child_index::<usize>(), Some(2));
        assert_eq!(block.pop_free_child_index::<usize>(), None);
        
        block.destruct(); 
    }
}

#[test]
fn block_free_inidces_test2(){
    let mut block: BlockPtr<usize, Config64bit<2>> = BlockPtr::new(ChildsType::DataBlocks, 16);
    unsafe{
        block.push_free_child_index::<usize>(2);
        block.push_free_child_index::<usize>(4);
        assert_eq!(block.pop_free_child_index::<usize>(), Some(4));
        
        block.push_free_child_index::<usize>(8);
        assert_eq!(block.pop_free_child_index::<usize>(), Some(8));
        assert_eq!(block.pop_free_child_index::<usize>(), Some(2));
        assert_eq!(block.pop_free_child_index::<usize>(), None);
        
        block.destruct(); 
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
                    block.write_child_at(
                        <MakeDefaultFor<T, R> as MakeDefault<T>>::make_default(),
                        0
                    );
                    block.set_len(1);
                }
            }
            empty_branch_blocks.as_mut()[Conf::LevelCount::VALUE-1].write(block);
            for I in (0..Conf::LevelCount::VALUE-1).rev() {
                let mut new_block = BlockPtr::new(ChildsType::Blocks, 1);
                unsafe{
                    new_block.write_child_at(block, 0);
                    new_block.set_len(1);
                }
                block = new_block;
                empty_branch_blocks.as_mut()[I].write(block);
            }
            unsafe{ Array::assume_init_array(empty_branch_blocks) }
        };
        
        let mut root = BlockPtr::new(ChildsType::Blocks, 2);
        unsafe{
            if <Conf::LevelCount as ConstInteger>::VALUE == 1 {
                if const{R::REQUIRED} {
                    root.write_child_at(
                        <MakeDefaultFor<T, R> as MakeDefault<T>>::make_default(),
                        0
                    );
                    root.set_len(1);
                }                
            } else {            
                root.write_child_at(
                    empty_branch_blocks.as_ref()[1], 
                    0
                );
                root.set_len(1);
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
                                block.write_child_at(
                                    <MakeDefaultFor<T, R> as MakeDefault<T>>::make_default(),
                                    0
                                );
                                block.set_len(1);
                                block
                            } else {
                                BlockPtr::new(ChildsType::DataBlocks, 1)    
                            }
                        } else {
                            let mut block = BlockPtr::new(ChildsType::Blocks, 2);
                            block.write_child_at(
                                self.empty_branch_blocks.as_ref()[I+1],
                                0
                            );
                            block.set_len(1);
                            block
                        }
                    }
                )
            };
        });   
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1]; 
        unsafe{ block.insert_unchecked(child_index, value); }
    }
    
    // TODO: Do not track root block - it always only one.
    fn get_branch(&self, index: &HierarchyIndex<Conf::Mask, Conf::LevelCount>) 
        -> ArrayOf< BlockPtr<T, Conf>, Conf::LevelCount >  
    {
        let mut branch = <ArrayOf< BlockPtr<T, Conf>, Conf::LevelCount> as Array>::uninit_array();
        
        let mut block = self.root;
        branch.as_mut()[0].write(block); 
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe{ *block.get_unchecked_ptr(child_index) };
            branch.as_mut()[I+1].write(block); 
        });
        
        unsafe{ Array::assume_init_array(branch) }        
    }
    
    pub fn remove(
        &mut self,
        index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>,
    ){
        let index = index.into();
        let mut branch = self.get_branch(&index);
        unsafe{
            let terminal_node = branch.as_mut().last_mut().unwrap_unchecked();
            let terminal_child_index = *index.level_indices.as_ref().last().unwrap_unchecked();
            
            if !terminal_node.have_child_unchecked(terminal_child_index) {
                // Have no such item
                return;
            }
            
            terminal_node.remove_unchecked::<T>(terminal_child_index);

            if terminal_node.is_empty() {
                terminal_node.destruct();
                
                // climb up the tree, and remove empty nodes
                const_loop!(I in 0..{Conf::LevelCount::VALUE-1} rev => 'out: {
                //'out: for I in (0..{Conf::LevelCount::VALUE-1}).rev() {
                    let mut node = branch.as_mut()[I];
                    node.remove_unchecked::<BlockPtr<T, Conf>>(
                        index.level_indices.as_ref()[I]
                    );

                    if !node.is_empty() {
                        break 'out;
                    }

                    if I != 0 {
                        node.destruct_empty();
                    }
                //}
                });                
            }
        }
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
            self.empty_branch_blocks.as_mut()[0].destruct_empty_branch::<R>();
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
    
    #[test]
    fn remove_test(){
        let mut tree: Tree<usize, Config64bit<3>> = Tree::new();
        tree.insert(0, 0);
        tree.insert(4000, 4000);
        assert_eq!(tree.get_mut(0), Some(&mut 0));
        assert_eq!(tree.get_mut(4000), Some(&mut 4000));
        
        tree.remove(4000);
        assert_eq!(tree.get_mut(0), Some(&mut 0));
        assert_eq!(tree.get_mut(4000), None);

        tree.remove(0);
        assert_eq!(tree.get_mut(0), None);
        assert_eq!(tree.get_mut(4000), None);
    }
    
}