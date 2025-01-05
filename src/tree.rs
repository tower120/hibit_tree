use std::alloc::{alloc, dealloc, realloc, Layout};
use std::marker::PhantomData;
use std::{cmp, mem, ptr};
use std::ptr::{addr_of_mut, null, NonNull};
use wide::u64x2;
use crate::{BitBlock, HibitTree, HibitTreeCursor, HibitTreeCursorTypes, HibitTreeTypes, HierarchyIndex, ReqDefault};
use crate::const_utils::{const_loop, max, ArrayOf, ConstBool, ConstFalse, ConstInteger, ConstUsize};
use crate::req_default::{MakeDefault, MakeDefaultFor, DefaultRequirement, IsReqDefault};
use crate::utils::{Array, Borrowable};

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

// We do not implement Config256bit, since we use one index as sentinel.
// That would not fit u8 range (256+1 > u8::MAX).

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
    
    phantom_data: PhantomData<T>
}
impl<T, Conf:Config> BlockHeader<T, Conf>{
    #[inline]
    const fn layout(child_align: usize) -> Layout {
        unsafe {
            Layout::from_size_align_unchecked(
                size_of::<Self>(),
                max!(child_align, align_of::<Self>())
            )
            .pad_to_align()
        }
    }
    
    #[inline]
    const fn children_addr_offset(child_align: usize) -> usize {
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
    const fn layout<Child>(cap: u8) -> Layout {
        let array_size = size_of::<Child>() * cap as usize;
        let header_layout = BlockHeader::<T, Conf>::layout(align_of::<Child>()); 
        let size = header_layout.size() + array_size;
        
        unsafe {
            Layout::from_size_align_unchecked(
                size,
                header_layout.align()
            ).pad_to_align()
        }        
    }
    
    #[inline]
    pub fn new<Child>(cap: u8) -> Self {
        let layout = Self::layout::<Child>(cap);
        unsafe{
            let block = alloc(layout) as *mut BlockHeader<T, Conf>;
            
            addr_of_mut!((*block).mask).write(BitBlock::zero());
            addr_of_mut!((*block).child_indices).write_bytes(0, 1);
            addr_of_mut!((*block).len).write(0);
            addr_of_mut!((*block).cap).write(cap);
            addr_of_mut!((*block).free_child_index).write(FREE_CHILD_INDEX_SENTINEL);
            
            Self(NonNull::new_unchecked(block))
        }
    }
    
    #[inline]
    const fn children_ptr(&self, child_align: usize) -> *mut u8 {
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
    pub fn mask(&self) -> &Conf::Mask {
        unsafe{ &self.0.as_ref().mask }
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
                    Self::layout::<Child>(block.cap),
                    Self::layout::<Child>(new_capacity).size(),
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
    
    // TODO: we no longer need this - we can drop empty_branch_blocks with 
    //      just destruct<T> + n x destruct_empty<BlockPtr>()
    /// Destructs each first child in "empty branch".
    /// 
    /// `Height` - distance to terminal node. 0 - means this IS a terminal node. 
    #[inline]
    pub unsafe fn destruct_empty_branch<Height: ConstInteger, R: DefaultRequirement>(&mut self, height: Height, req: R) {
        if Height::VALUE == 0 {
            // terminal node
            if R::Required::VALUE {
                let child = &mut *self.children_ptr(align_of::<T>()).cast::<T>();
                ptr::drop_in_place(child);
            }
            self.destruct_empty::<T>();
        } else {
            let mut child = &mut *self.children_ptr(align_of::<BlockPtr<T, Conf>>()).cast::<BlockPtr<T, Conf>>();
            child.destruct_empty_branch(height.dec(), req);
            ptr::drop_in_place(child);
            self.destruct_empty::<BlockPtr<T, Conf>>();
        }
    }
    
    /// Destruct block and it's children "recursively".
    /// 
    /// `Height` - distance to terminal node. 0 - means this IS a terminal node. 
    #[inline]
    pub unsafe fn destruct<Height: ConstInteger>(&mut self, height: Height){
        if Height::VALUE == 0 {
            // terminal level
            let mut iter = self.children_iter_mut::<T>();
            for child in iter {
                ptr::drop_in_place(child);
            }
            self.destruct_empty::<T>();
        } else {
            let mut iter = self.children_iter_mut::<BlockPtr<T, Conf>>();
            for child in iter {
                child.destruct(height.dec());
                ptr::drop_in_place(child);
            }
            self.destruct_empty::<BlockPtr<T, Conf>>();
        }
    }
    
    /// Destruct block without children.
    #[inline]
    pub unsafe fn destruct_empty<Child>(&mut self) {
        let block = self.0.as_mut();
        let layout = Self::layout::<Child>(block.cap);
        ptr::drop_in_place(block);
        dealloc(self.0.as_ptr().cast(), layout);
    }
}

#[test]
fn block_free_inidces_test(){
    let mut block: BlockPtr<usize, Config64bit<2>> = BlockPtr::new::<usize>(16);
    unsafe{
        block.push_free_child_index::<usize>(2);
        block.push_free_child_index::<usize>(4);
        block.push_free_child_index::<usize>(8);
        
        assert_eq!(block.pop_free_child_index::<usize>(), Some(8));
        assert_eq!(block.pop_free_child_index::<usize>(), Some(4));
        assert_eq!(block.pop_free_child_index::<usize>(), Some(2));
        assert_eq!(block.pop_free_child_index::<usize>(), None);
        
        block.destruct(ConstUsize::<0>); 
    }
}

#[test]
fn block_free_inidces_test2(){
    let mut block: BlockPtr<usize, Config64bit<2>> = BlockPtr::new::<usize>(16);
    unsafe{
        block.push_free_child_index::<usize>(2);
        block.push_free_child_index::<usize>(4);
        assert_eq!(block.pop_free_child_index::<usize>(), Some(4));
        
        block.push_free_child_index::<usize>(8);
        assert_eq!(block.pop_free_child_index::<usize>(), Some(8));
        assert_eq!(block.pop_free_child_index::<usize>(), Some(2));
        assert_eq!(block.pop_free_child_index::<usize>(), None);
        
        block.destruct(ConstUsize::<0>); 
    }
}


type EmptyBranchBlocks<T, Conf: Config> = ArrayOf<BlockPtr<T, Conf>, /*<*/Conf::LevelCount/* as ConstInteger>::Inc*/>;

pub struct Tree<T, Conf:Config, R: DefaultRequirement = ReqDefault<ConstFalse>>{
    root: BlockPtr<T, Conf>,
    
    // TODO: root level empty block never used - remove?
    /// Sequence of empty blocks with child at pos 0.
    /// This lets us have branchless get().
    empty_branch_blocks: EmptyBranchBlocks<T, Conf>,
    
    phantom_data: PhantomData<R>
}

impl<T, Conf:Config, R: DefaultRequirement> Default for Tree<T, Conf, R>
where
    MakeDefaultFor<T, R>: MakeDefault<T>
{
    #[inline]
    fn default() -> Self {
        Tree::new()
    }
}

impl<T, Conf:Config, R: DefaultRequirement> Tree<T, Conf, R>
{
    #[inline]
    fn get_terminal_block(&self, index: &HierarchyIndex<Conf::Mask, Conf::LevelCount>)
        -> BlockPtr<T, Conf> 
    {
        let mut block = self.root;
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe{ *block.get_unchecked_ptr(child_index) };
        });
        block
    }
    
    #[inline]
    fn get_impl(&self, index: &HierarchyIndex<Conf::Mask, Conf::LevelCount>) -> Option<*mut T> {
        let block = self.get_terminal_block(index);
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        unsafe{
            if block.have_child_unchecked(child_index) {
                Some( block.get_unchecked_ptr(child_index) )
            } else {
                None
            }
        }
    }       
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
            let mut block = BlockPtr::new::<T>(1);
            if R::Required::VALUE {
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
                let mut new_block = BlockPtr::new::<BlockPtr<T, Conf>>(1);
                unsafe{
                    new_block.write_child_at(block, 0);
                    new_block.set_len(1);
                }
                block = new_block;
                empty_branch_blocks.as_mut()[I].write(block);
            }
            unsafe{ Array::assume_init_array(empty_branch_blocks) }
        };
        
        let mut root = BlockPtr::new::<BlockPtr<T, Conf>>(2);
        unsafe{
            if <Conf::LevelCount as ConstInteger>::VALUE == 1 {
                if const{R::Required::VALUE} {
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
                            if const {R::Required::VALUE} {
                                let mut block = BlockPtr::new::<T>(2);
                                block.write_child_at(
                                    <MakeDefaultFor<T, R> as MakeDefault<T>>::make_default(),
                                    0
                                );
                                block.set_len(1);
                                block
                            } else {
                                BlockPtr::new::<T>(1)    
                            }
                        } else {
                            let mut block = BlockPtr::new::<BlockPtr<T, Conf>>(2);
                            block.write_child_at(
                                // I+2, because we point from child, and to it's child
                                self.empty_branch_blocks.as_ref()[I+2],
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
    
    // TODO: Do not track root block - it is always only one.
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
                terminal_node.destruct_empty::<T>();
                
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
                        node.destruct_empty::<BlockPtr<T, Conf>>();
                    }
                //}
                });                
            }
        }
    }
    
    #[inline]
    pub fn get_mut(&mut self, index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>) -> Option<&mut T> {
        self.get_impl(&index.into()).map(|v| unsafe{ &mut *v })
    }
    
/*    #[inline]
    pub fn get(&self, index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>) -> Option<&T> {
        self.get_impl(&index.into()).map(|v| unsafe{ &*v })
    }    
    
    #[inline]
    pub fn get_or_default(
        &self, 
        index: impl Into<HierarchyIndex<Conf::Mask, Conf::LevelCount>>
    ) -> &T
    where
        R: IsReqDefault
    {
        
        let index = index.into();
        let mut block = self.root;
        const_loop!(I in 0..{Conf::LevelCount::VALUE-1} => {
            let child_index = index.level_indices.as_ref()[I];
            block = unsafe{ *block.get_unchecked_ptr(child_index) };
        });
        
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        unsafe{ &*block.get_unchecked_ptr(child_index) }
        
        //unsafe{ self.get(index).unwrap_unchecked() }
    } */   
    
}
impl<T, Conf: Config, R: DefaultRequirement> Drop for Tree<T, Conf, R> {
    fn drop(&mut self) {
        unsafe{
            let height = Conf::LevelCount::DEFAULT.dec();
            self.root.destruct(height);
            self.empty_branch_blocks.as_mut()[0].destruct_empty_branch(height, R::default());
        }
    }
}

impl<T, Conf: Config, R: DefaultRequirement> Borrowable for Tree<T, Conf, R> {
    type Borrowed = Self;
}

impl<'a, T, Conf: Config, R: DefaultRequirement> HibitTreeTypes<'a> for Tree<T, Conf, R> {
    type Data = &'a T;
    type DataUnchecked = &'a T;
    type DataOrDefault = &'a T;
    type Cursor = TreeCursor<'a, T, Conf, R>;
}

impl<T, Conf: Config, R: DefaultRequirement> HibitTree for Tree<T, Conf, R> {
    const EXACT_HIERARCHY: bool = true;
    type DefaultData = R::Required;
    
    type LevelCount = Conf::LevelCount;
    type LevelMask  = Conf::Mask;

    #[inline]
    fn data(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>) 
        -> Option<&T> 
    {
        self.get_impl(index).map(|v| unsafe{ &*v })
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>) 
        -> &T 
    {
        let block = self.get_terminal_block(index);
        let child_index = index.level_indices.as_ref()[Conf::LevelCount::VALUE-1];
        &*block.get_unchecked_ptr(child_index)
    }
    
    #[inline]
    unsafe fn data_or_default(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>) 
        -> &T 
    {
        self.data_unchecked(index)
    }
}

type CursorBranch<T, Conf: Config> = ArrayOf<
    Option<BlockPtr<T, Conf>>, 
    Conf::LevelCount
>; 
pub struct TreeCursor<'tree, T, Conf: Config, R: DefaultRequirement>{
    branch: CursorBranch<T, Conf>, 
    phantom_data: PhantomData<&'tree Tree<T, Conf, R>>
}
impl<'a, 'tree, T, Conf: Config, R: DefaultRequirement> HibitTreeCursorTypes<'a> for TreeCursor<'tree, T, Conf, R> {
    type Data = &'tree T;
    type DataUnchecked = Self::Data;
    type DataOrDefault = Self::Data;
}
impl<'tree, T, Conf: Config, R: DefaultRequirement> HibitTreeCursor<'tree> for TreeCursor<'tree, T, Conf, R> {
    type Tree = Tree<T, Conf, R>;

    #[inline]
    fn new(src: &'tree Self::Tree) -> Self {
        let mut branch: CursorBranch<T, Conf> = Array::from_fn(|_|None);
        branch.as_mut()[0] = Some(src.root);
        
        Self{
            branch,
            phantom_data: Default::default(),
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(&mut self, _: &'tree Self::Tree, level_n: N, level_index: usize) 
        -> <Self::Tree as HibitTree>::LevelMask 
    {
        if N::VALUE == 0 {
            return self.branch.as_ref()[0].unwrap_unchecked().mask().clone();
        }      
        
        let parrent_node = self.branch.as_ref().get_unchecked(level_n.value() - 1).unwrap_unchecked();
        let node = *parrent_node.get_unchecked_ptr::<BlockPtr<T, Conf>>(level_index);
        
        *self.branch.as_mut().get_unchecked_mut(level_n.value()) = Some(node);
        
        node.mask().clone()
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(&mut self, tree: &'tree Self::Tree, level_n: N, level_index: usize) 
        -> <Self::Tree as HibitTree>::LevelMask 
    {
        self.select_level_node(tree, level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> Option<&'tree T> 
    {
        let terminal_node = self.branch.as_ref().last().unwrap_unchecked().unwrap_unchecked();
        if terminal_node.have_child_unchecked(level_index) {
            Some(&*terminal_node.get_unchecked_ptr::<T>(level_index))
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> &'tree T
    {
        let terminal_node = self.branch.as_ref().last().unwrap_unchecked().unwrap_unchecked();
        &*terminal_node.get_unchecked_ptr::<T>(level_index)
    }
    
    #[inline]
    unsafe fn data_or_default<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> &'tree T
    {
        self.data_unchecked(tree, level_index)
    }
}

#[cfg(test)]
mod test{
    use std::collections::HashMap;
    use itertools::assert_equal;
    use rand::{Rng, SeedableRng};
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
    fn get_default_test(){
        let mut tree: Tree<usize, Config64bit<2>, ReqDefault> = Tree::new();
        tree.insert(200, 200);
        assert_eq!(tree.get_or_default(10), &0);
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
    
    #[test]
    fn iter_test(){
        let mut tree: Tree<usize, Config64bit<3>> = Tree::new();
        assert!(tree.iter().next().is_none());
        
        tree.insert(0, 0);
        tree.insert(4000, 4000);
        tree.insert(100, 100);
        tree.insert(18000, 18000);
        
        assert_equal(tree.iter(), [
            (0, &0),
            (100, &100),
            (4000, &4000),
            (18000, &18000),
        ])
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