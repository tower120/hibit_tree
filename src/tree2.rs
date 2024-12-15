use std::alloc::{alloc, dealloc, realloc, Layout};
use std::marker::PhantomData;
use std::{cmp, mem, ptr};
use std::ptr::{addr_of_mut, NonNull};
use crate::{BitBlock, HierarchyIndex};
use crate::const_utils::{const_loop, ArrayOf, ConstInteger, ConstUsize};

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

/// Owning ptr
// TODO: rename to just Block?
struct BlockPtr<T, Conf:Config>(NonNull<BlockHeader<T, Conf>>);
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
            addr_of_mut!((*block).len).write(0);
            addr_of_mut!((*block).cap).write(cap);
            addr_of_mut!((*block).childs_type).write(childs_type);
            
            Self(NonNull::new_unchecked(block))
        }
    }
    
    #[inline]
    /*const*/ fn children_ptr_mut(&mut self, child_align: usize) -> *mut u8 {
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
                &mut *self.children_ptr_mut(align_of::<V>()).cast::<V>().add(i)
            })
    }    
    
    #[inline]
    unsafe fn push_within_capacity_unchecked<V>(&mut self, value: V) -> *mut V {
        let block = self.0.as_mut();
        debug_assert!(block.len < block.cap);
        let ptr: *mut V = self.children_ptr_mut(align_of::<V>()).cast::<V>()
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
        this.as_mut().get_unchecked_mut::<V>(index)
    }
    
    #[inline]
    pub unsafe fn insert_unchecked<V>(
        &mut self, 
        index: usize, 
        child: V
    ) {
        if let Err(child) = self.insert_impl(index, ||child){
            *self.get_unchecked_mut::<V>(index) = child();
        }
    }
    
    #[inline]
    pub unsafe fn get_unchecked_mut<V> (
        &mut self, 
        index: usize,
    ) -> &mut V {
        let block = self.0.as_ref();
        let i = *block.child_indices.as_ref().get_unchecked(index);
        &mut *self.children_ptr_mut(align_of::<V>()).cast::<V>().add(i as usize)
    }     

    #[inline]
    pub unsafe fn have_child_unchecked(&self, index: usize) -> bool {
        let block = self.0.as_ref();
        block.mask.get_bit_unchecked(index)
    }  
}
impl<T, Conf:Config> Drop for BlockPtr<T, Conf>{
    fn drop(&mut self) {
        unsafe{
            let block = self.0.as_ref();
            
            // TODO: this switch can be compiletime
            // 1. destruct children
            match block.childs_type{
                ChildsType::Blocks => {
                    for child in self.children_iter_mut::<BlockPtr<T, Conf>>(){
                        ptr::drop_in_place(child);
                    }
                }
                ChildsType::DataBlocks => {
                    for child in self.children_iter_mut::<T>(){
                        ptr::drop_in_place(child);
                    }
                }
            }
            
            // 2. Deallocate self
            let layout = Self::layout(block.cap, block.childs_type);
            dealloc(self.0.as_ptr().cast(), layout);
        }
    }
}

pub struct Tree<T, Conf:Config>{
    root: BlockPtr<T, Conf>
}

impl<T, Conf:Config> Tree<T, Conf>{
    pub fn new() -> Self{
        Self{
            root: BlockPtr::new(ChildsType::Blocks, 1 ),
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
                            /*if R::REQUIRED {
                                let mut block = Block::with_capacity(ChildsType::DataBlocks, 2);
                                <DefaultInitFor<T, R> as DefaultInit>::init_default(childs::as_mut_ptr(&mut block).cast());
                                childs::set_len(&mut block, 1);
                                block
                            } else {
                                Block::with_capacity(ChildsType::DataBlocks, 1)
                            }*/
                            BlockPtr::new(ChildsType::DataBlocks, 1)
                        } else {
                            /*let empty_child = Self::make_empty_block::<{I+1}>(&self.empty_branch_block_childs);
                            let mut block = Block::with_capacity(ChildsType::Blocks, 2);
                            childs::push_within_capacity_unchecked(&mut block, empty_child);
                            block*/
                            BlockPtr::new(ChildsType::Blocks, 1)
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
                if !block.have_child_unchecked(child_index) {
                    return None;
                }
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
        assert_eq!(tree.get_mut(1000), None);
        
    }
}