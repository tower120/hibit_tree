use std::marker::PhantomData;
use std::ptr::{null_mut, NonNull};
use crate::BitBlock;
use crate::const_utils::ArrayOf;
use crate::tree::Config;
use crate::utils::Array;

pub mod childs{
    use std::alloc::{alloc, dealloc, realloc, Layout};
    use std::{mem, ptr};
    use super::*;
    use crate::tree::Config;
    
    #[inline]
    fn layout<T, Conf: Config>(childs_type: ChildsType, cap: u8) -> Layout {
        match childs_type {
            ChildsType::Blocks => Layout::array::<Block<T, Conf>>(cap as usize),
            ChildsType::DataBlocks => Layout::array::<T>(cap as usize),
        }.unwrap()
    }

    #[inline]
    pub fn init<T, Conf: Config>(
        block: &mut Block<T, Conf>,
        childs_type: ChildsType,
        cap: u8
    ) {
        let layout = layout::<T, Conf>(childs_type, cap);
        unsafe {
            block.childs_ptr = alloc(layout);    
        }
        block.childs_len = 0;
        block.childs_cap = cap;
    }
    
    #[inline]
    pub fn as_mut_ptr<T, Conf: Config>(block: &mut Block<T, Conf>,) -> *mut u8 {
        block.childs_ptr
    }
    
    #[inline]
    pub fn as_ptr<T, Conf: Config>(block: &Block<T, Conf>,) -> *const u8 {
        block.childs_ptr
    }
    
    #[inline]
    pub unsafe fn set_len<T, Conf: Config>(block: &mut Block<T, Conf>, len: u8) {
        block.childs_len = len;
    }
    
    #[inline]
    pub unsafe fn push_within_capacity_unchecked<V, T, Conf: Config>(block: &mut Block<T, Conf>, value: V) -> *mut V {
        debug_assert!(block.childs_len < block.childs_cap);
        let ptr: *mut V = block.childs_ptr.cast::<V>().add(block.childs_len as usize);
        ptr.write(value);
        block.childs_len += 1;
        ptr        
    }
    
    #[inline]
    pub unsafe fn push_unchecked<V, T, Conf: Config>(block: &mut Block<T, Conf>, value: V) -> *mut V {
        if block.childs_len == block.childs_cap {
            let new_cap = block.childs_cap * 2;
            let new_ptr = realloc(
                block.childs_ptr,
                Layout::array::<V>(block.childs_cap as usize).unwrap(),
                Layout::array::<V>(new_cap as usize).unwrap().size()
            );
            block.childs_ptr = new_ptr;
            block.childs_cap = new_cap;
        }
        push_within_capacity_unchecked(block, value)
    }
    
    pub(super) unsafe fn destroy<T, Conf: Config>(block: &mut Block<T, Conf>) {
        if !block.destruct_childs{
            return;
        }
        
        // drop childs
        {
            match block.childs_type {
                ChildsType::Blocks => {
                    for i in 0..block.childs_len as usize {
                        let ptr = block.childs_ptr.cast::<Block<T, Conf>>().add(i);
                        ptr::drop_in_place(ptr);
                    }
                }
                ChildsType::DataBlocks => {
                    if mem::needs_drop::<T>() {
                        for i in 0..block.childs_len as usize {
                            let ptr = block.childs_ptr.cast::<T>().add(i);
                            ptr::drop_in_place(ptr);
                        }            
                    }
                }
            }
        }

        // dealloc
        let layout = layout::<T, Conf>(block.childs_type, block.childs_cap);
        unsafe{
            dealloc(block.childs_ptr, layout);
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ChildsType{Blocks, DataBlocks}

#[repr(C)]  // We want `childs` to be near `mask` - so they be in one cache line.
pub struct Block<T, Conf: Config /* TODO: mask instead? */ >{
    mask: Conf::Mask,
    childs_ptr: *mut u8,
    child_indices: ArrayOf<
        u8,
        <Conf::Mask as BitBlock>::Size
    >,
    childs_len: u8,
    childs_cap: u8,
    childs_type: ChildsType,
    pub destruct_childs: bool,  // TODO: This can be done in compile-time fashion instead.
    phantom_data: PhantomData<T>
}

impl<T, Conf: Config> Block<T, Conf> {
    #[inline]
    pub fn from_parts(
        childs_type: ChildsType, 
        childs_ptr: *mut u8,
        childs_len: u8,
        childs_cap: u8,
    ) -> Self {
        Self{
            mask: BitBlock::zero(),
            childs_ptr,
            child_indices: Array::from_fn(|_|0),
            childs_len,
            childs_cap,
            childs_type,
            destruct_childs: true,
            phantom_data: Default::default(),
        }        
    }
    
    #[inline]
    pub fn with_capacity(childs_type: ChildsType, cap: u8) -> Self {
        debug_assert!(cap > 0);
        let mut this = Self::from_parts(childs_type, null_mut(), 0, 0);
        childs::init(&mut this, childs_type, cap);
        this
    }
    
    #[inline]
    pub fn new(childs_type: ChildsType) -> Self {
        Self::with_capacity(childs_type, 1)
    }    
    
    #[inline]
    pub unsafe fn get_unchecked<V> (
        &self, 
        index: usize,
    ) -> &V {
        let i = *self.child_indices.as_ref().get_unchecked(index);
        &*self.childs_ptr.cast::<V>().add(i as usize)
    }    
    
    #[inline]
    pub unsafe fn get_unchecked_mut<V> (
        &mut self, 
        index: usize,
    ) -> &mut V {
        let i = *self.child_indices.as_ref().get_unchecked(index);
        &mut *self.childs_ptr.cast::<V>().add(i as usize)
    }
    
    #[inline]
    pub unsafe fn have_child_unchecked(&self, index: usize) -> bool {
        self.mask.get_bit_unchecked(index)
    }    
    
    /// Get or insert block or data. 
    #[inline]
    unsafe fn insert_impl<V, Child: FnOnce() -> V>(
        &mut self, 
        index: usize, 
        child: Child
    ) -> Result<&mut V, Child> {
        // TODO: try read first
        let have_child = unsafe {
            self.mask.set_bit_unchecked::<true>(index)    
        };
        if have_child {
            return Err(child);
            //return self.get_unchecked_mut::<V>(index);
        }
        
        let child_index = self.childs_len; 
        let child = childs::push_unchecked(self, child());
        
        *self.child_indices.as_mut().get_unchecked_mut(index) = child_index; 
        
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
    
}

impl<T, Conf: Config> Drop for Block<T, Conf> {
    fn drop(&mut self) {
        unsafe{
            childs::destroy(self);
        }
    }
}