//! Use compact_sparse_array2 instead. 

use std::alloc::{alloc, Layout, realloc};
use std::{mem, ptr};
use std::mem::{align_of, ManuallyDrop, MaybeUninit, size_of};
use std::ptr::{addr_of_mut, NonNull};
use crate::bit_utils::{get_bit_unchecked, set_bit_unchecked};
use crate::const_utils::{ConstInteger, ConstUsize};
use crate::sparse_array::level_indices;
use crate::utils::Primitive;

type Mask = u64;
type DataIndex = u32;

const DEFAULT_CAP: u8 = 2;

#[repr(C)]
struct Node{
    mask: Mask,
    
    capacity: u8,
    len: u8,
    
    // TODO: calculate programmatically
    /// NonNull<Node> / u32
    children_placeholder: [*const u8; 0]
}

impl Node{
    #[inline]
    pub unsafe fn get_child<T>(&self, index: usize) -> &T {
        let dense_index = self.get_dense_index(index);
        &*self.children_ptr::<T>().add(dense_index)
    }
    
    #[inline]
    pub unsafe fn get_child_mut<T>(&mut self, index: usize) -> &mut T {
        let dense_index = self.get_dense_index(index);
        &mut*self.children_mut_ptr::<T>().add(dense_index)
    }
    
    unsafe fn children_ptr<T>(&self) -> *const T{
        self.children_placeholder.as_ptr() as *const u8 as _
    }
    
    unsafe fn children_mut_ptr<T>(&mut self) -> *mut T{
        self.children_placeholder.as_mut_ptr() as *mut u8 as _
    }
    
    const fn node_children_addr_offset() -> usize {
        mem::offset_of!(Node, children_placeholder)
    }
    
    #[inline]
    fn layout<T>(cap: u8) -> Layout {
        let array_size = size_of::<T>() * cap as usize;
        let size = Self::node_children_addr_offset() + array_size;
        
        unsafe{
            Layout::from_size_align_unchecked(size, align_of::<Node>())
            .pad_to_align()
        }
    }
    
    #[inline]
    pub fn new<T>(cap: u8) -> NonNull<Self> {
        unsafe {
            let node = alloc(Self::layout::<T>(cap)) as *mut Self;
            
            addr_of_mut!((*node).mask).write(Mask::default());
            addr_of_mut!((*node).capacity).write(cap);
            addr_of_mut!((*node).len).write(0);
            
            NonNull::new_unchecked(node)
        }
    }
    
    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        unsafe{ get_bit_unchecked(self.mask, index) }
    }
    
    /// `index` must be set.
    #[inline]
    unsafe fn get_dense_index(&self, index: usize) -> usize {
        let mask = !(u64::MAX << index);
        let block = (self.mask /*| self.mask_disabler*/) & mask;
        block.count_ones() as usize
        
        /*
        // This cause shift overflow if index == 0
        (self.mask << (u64::BITS as usize - index))
            .count_ones() as usize*/
    }    
    
    // TODO: can use &mut self instead of NonNull<Node>?
    /// Returns a new pointer if relocation happened.
    /// 
    /// # Safety
    /// - `T` must match stored data.
    /// - `this_ptr` must be valid.
    /// - `index` must be in range.
    #[inline]
    pub unsafe fn insert<T>(mut this_ptr: NonNull<Node>, index: usize, value: T)
        // TODO: try Option
        -> (NonNull<T>, /*Option<*/NonNull<Self>/*>*/) 
    {
        // TODO: special case for full-size mode

        /* realloc */ {
            let capacity = this_ptr.as_ref().capacity;
            if this_ptr.as_ref().len == capacity {
                let new_capacity = capacity * 2;
                let new_ptr= realloc(
                    this_ptr.as_ptr() as *mut u8,
                    Self::layout::<T>(capacity),
                    Self::layout::<T>(new_capacity).size(),
                ) as *mut Self;
                (*new_ptr).capacity = new_capacity; 
                this_ptr = NonNull::new_unchecked(new_ptr);
            }
        }

        let this = this_ptr.as_mut();
        set_bit_unchecked::<true, _>(&mut this.mask, index);
        let dense_index = this.get_dense_index(index);
        
        /* move right */ 
        let p: *mut T = this.children_mut_ptr::<T>().add(dense_index);
        // Shift everything over to make space. (Duplicating the
        // `index`th element into two consecutive places.)
        ptr::copy(p, p.offset(1), this.len as usize - dense_index);
        // Write it in, overwriting the first copy of the `index`th
        // element.
        ptr::write(p, value);
        
        this.len += 1;

        (NonNull::new_unchecked(p), this_ptr)
    }
    
    /// Deallocate node
    pub fn drop_node(this: NonNull<Self>){
        todo!()
    } 
}

pub struct CompactSparseArray2<T, const DEPTH: usize>{
    root: NonNull<Node>,
    
    // TODO: store keys with data?
    data: Vec<T>,
    keys: Vec<usize>,
}

impl<T, const DEPTH: usize> Default for CompactSparseArray2<T, DEPTH>{
    #[inline]
    fn default() -> Self {
        Self{
            root: Node::new::<NonNull<Node>>(DEFAULT_CAP),
            data: Vec::new(),
            keys: Vec::new(),
        }
    }
}

impl<T, const DEPTH: usize> CompactSparseArray2<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger    
{
    #[inline]
    pub fn get_or_insert(&mut self, index: usize) -> &mut T
    where
        T: Default      // TODO: should be empty?
    {
        let indices = level_indices::<u64, ConstUsize<DEPTH>>(index);
        
        // get terminal node pointing to data
        let mut node = &mut self.root;
        for n in 0..DEPTH-1 {
            let inner_index = indices.as_ref()[n];
            unsafe{
                let mut node_ptr = *node;
                node = if node_ptr.as_ref().contains(inner_index) {
                    node_ptr.as_mut().get_child_mut(inner_index)
                } else {
                    // TODO: insert node with already inserted ONE element 
                    //       all down below. And immediately exit loop?
                    //       BENCHMARK change.
                    let (mut inserted_ptr, new_node) = Node::insert(node_ptr, inner_index, Node::new::<NonNull<Node>>(DEFAULT_CAP));  // update a child pointer with a (possibly) new address
                    *node = new_node;   // This is actually optional
                    inserted_ptr.as_mut()
                }
            }
        }        
     
        // now fetch data
        unsafe{
            let node_ptr = *node;
            let inner_index = *indices.as_ref().last().unwrap_unchecked();
            
            let data_index = if node_ptr.as_ref().contains(inner_index) {
                node_ptr.as_ref().get_child::<DataIndex>(inner_index).as_usize()
            } else {
                let i = self.data.len(); 
                self.data.push(T::default());
                self.keys.push(index);
                let (_, new_node) = Node::insert(node_ptr, inner_index, i as DataIndex);
                *node = new_node;
                i
            };
            self.data.get_unchecked_mut(data_index)
        }
    }
    
    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        let indices = level_indices::<u64, ConstUsize<DEPTH>>(index);
        let mut node = &self.root;
        for n in 0..DEPTH-1 {
            let inner_index = indices.as_ref()[n];
            node = unsafe{ node.as_ref().get_child(inner_index) };
        }
        
        unsafe{
            let inner_index = *indices.as_ref().last().unwrap_unchecked();
            let data_index = node.as_ref().get_child::<DataIndex>(inner_index).as_usize();
            if *self.keys.get_unchecked(data_index) == index{
                Some(self.data.get_unchecked(data_index))
            } else {
                None
            }
            /*let data_index = if *self.keys.get_unchecked(data_index) != index{
                0
            } else {
                data_index
            };
            self.data.get_unchecked(data_index)*/
        }        
    }
    
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        unsafe{
            self.get(index).unwrap_unchecked()
        }
    }    
    
    /// experimental
    #[inline]
    pub fn get_or_default(&self, index: usize) -> &T {
        let indices = level_indices::<u64, ConstUsize<DEPTH>>(index);
        let mut node = &self.root;
        for n in 0..DEPTH-1 {
            let inner_index = indices.as_ref()[n];
            node = unsafe{ node.as_ref().get_child(inner_index) };
        }
        
        unsafe{
            let inner_index = *indices.as_ref().last().unwrap_unchecked();
            let data_index = node.as_ref().get_child::<DataIndex>(inner_index).as_usize();
            let data_index = if *self.keys.get_unchecked(data_index) == index{
                data_index
            } else {
                0
            };
            self.data.get_unchecked(data_index)
        }        
    }
}

#[cfg(test)]
mod test{
    use super::CompactSparseArray2;
    
    #[test]
    fn test(){
        let mut a: CompactSparseArray2<usize, 3> = Default::default();
        *a.get_or_insert(15) = 89;
                
        assert_eq!(*a.get_or_insert(15), 89);
        assert_eq!(*a.get_or_insert(0), 0);
        assert_eq!(*a.get_or_insert(100), 0);        
    }
}