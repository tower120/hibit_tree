//! Use compact_sparse_array2 instead. 

use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use thin_vec::ThinVec;
use crate::bit_utils::{get_bit_unchecked, set_bit_unchecked};
use crate::const_utils::{ConstArray, ConstInteger, ConstUsize};
use crate::sparse_array::level_indices;
use crate::SparseHierarchy;
use crate::utils::Primitive;

type Mask = u64;

/// Just a pointer.
union Childs<T>{
    nodes: ManuallyDrop<ThinVec<Node<T>>>, 
    data: ManuallyDrop<ThinVec<T>>
}

struct Node<T>{
    pub mask: Mask,
    /// 0 = standard mode;
    /// MAX = full block;
    switch_mask: u64,
    childs : Childs<T>
}
impl<T> Node<T>{
    #[inline]
    pub fn new_node() -> Self {
        Self{
            mask: 0,
            switch_mask: 0,
            childs: Childs{ nodes: Default::default() },
        }
    }
    
    #[inline]
    pub fn new_terminal_node() -> Self {
        Self{
            mask: 0,
            switch_mask: 0,
            childs: Childs{ data: Default::default() },
        }
    }
    
    #[inline]
    pub unsafe fn drop_as_node(&mut self){
        ManuallyDrop::drop(&mut self.childs.nodes);
    }
    
    #[inline]
    pub unsafe fn drop_as_terminal_node(&mut self){
        ManuallyDrop::drop(&mut self.childs.data);
    }
    
    /*/// 1 - if contains,
    /// 0 - otherwise
    #[inline]
    fn contains_as_mask(&self, index: usize) -> Mask {
        (self.mask >> index) & 1
    }*/

    /// Node must have Node childs
    #[inline]
    pub unsafe fn insert_node(&mut self, index: usize, node: Node<T>){
        set_bit_unchecked::<true, _>(&mut self.mask, index);
        let dense_index = self.get_dense_index(index);
        self.childs.nodes.deref_mut().insert(dense_index, node)
    }
    
    /// Node must have T childs
    #[inline]
    pub unsafe fn insert_data(&mut self, index: usize, data: T){
        set_bit_unchecked::<true, _>(&mut self.mask, index);
        let dense_index = self.get_dense_index(index);
        self.childs.data.deref_mut().insert(dense_index, data)
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
    
    #[inline]
    unsafe fn get_dense_index_or_zero(&self, index: usize) -> usize {
        let is_exists = (self.mask >> index) & 1;   // = 0 or 1
        let masked_block = self.mask & !(u64::MAX << index);
        (masked_block * is_exists).count_ones() as usize
    }

    #[inline]
    pub unsafe fn get_as_node(&self, index: usize) -> &Node<T> {
        let dense_index = self.get_dense_index_or_zero(index);
        self.childs.nodes.deref().get_unchecked(dense_index)
    }
    
    #[inline]
    pub unsafe fn get_as_node_unchecked(&self, index: usize) -> &Node<T> {
        let dense_index = self.get_dense_index(index);
        self.childs.nodes.deref().get_unchecked(dense_index)
    }
    
    #[inline]
    pub unsafe fn get_as_node_mut_unchecked(&mut self, index: usize) -> &mut Node<T> {
        let dense_index = self.get_dense_index(index);
        self.childs.nodes.deref_mut().get_unchecked_mut(dense_index)
    }
    
    #[inline]
    pub unsafe fn get_as_terminal_node(&self, index: usize) -> &T {
        let dense_index = self.get_dense_index_or_zero(index);
        self.childs.data.deref().get_unchecked(dense_index)
    }
    
    #[inline]
    pub unsafe fn get_as_terminal_node_unchecked(&self, index: usize) -> &T {
        let dense_index = self.get_dense_index(index);
        self.childs.data.deref().get_unchecked(dense_index)
    }
    
    #[inline]
    pub unsafe fn get_as_terminal_node_mut_unchecked(&mut self, index: usize) -> &mut T {
        let dense_index = self.get_dense_index(index);
        self.childs.data.deref_mut().get_unchecked_mut(dense_index)
    }  
    
}

pub struct CompactSparseArray<T, const DEPTH: usize>{
    root: Node<u32>,
    
    // TODO: store keys with data?
    data: Vec<T>,
    keys: Vec<usize>,
}

impl<T, const DEPTH: usize> Default for CompactSparseArray<T, DEPTH>{
    #[inline]
    fn default() -> Self {
        Self{
            root: Node::new_node(),
            data: Vec::new(),
            keys: Vec::new(),
        }
    }
}


impl<T, const DEPTH: usize> CompactSparseArray<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger    
{
    #[inline]
    pub fn get_or_insert(&mut self, index: usize) -> &mut T
    where
        T: Default      // TODO: should be empty?
    {
        let indices = level_indices::<u64, ConstUsize<DEPTH>>(index);
        let mut node = &mut self.root;
        // get terminal node pointing to data
        for n in 0..DEPTH-1 {
            let inner_index = indices.as_ref()[n];
            if !node.contains(inner_index) {
                unsafe{ node.insert_node(inner_index, Node::new_node()); }
            }
            node = unsafe{ node.get_as_node_mut_unchecked(inner_index) };
        }
        
        unsafe{
            let inner_index = *indices.as_ref().last().unwrap_unchecked();
            let data_index = if node.contains(inner_index) {
                node.get_as_terminal_node_unchecked(inner_index).as_usize()
            } else {
                let i = self.data.len(); 
                self.data.push(T::default());
                self.keys.push(index);
                node.insert_data(inner_index, i as u32);
                i
            };
            self.data.get_unchecked_mut(data_index)
        }
    }
    
    #[inline]
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        let indices = level_indices::<u64, ConstUsize<DEPTH>>(index);
        let mut node = &self.root;
        for n in 0..DEPTH-1 {
            let inner_index = indices.as_ref()[n];
            node = node.get_as_node_unchecked(inner_index);
        }
        
        unsafe{
            let inner_index = *indices.as_ref().last().unwrap_unchecked();
            let data_index = node.get_as_terminal_node_unchecked(inner_index).as_usize();
            let data_index = if *self.keys.get_unchecked(data_index) != index{
                0
            } else {
                data_index
            };
            self.data.get_unchecked(data_index)
        }        
    }
}

// TODO: Drop

#[cfg(test)]
mod test{
    use super::CompactSparseArray;
    
    #[test]
    fn smoke_test(){
        let mut a: CompactSparseArray<usize, 3> = Default::default();
        *a.get_or_insert(15) = 89;
                
        assert_eq!(*a.get_or_insert(15), 89);
        assert_eq!(*a.get_or_insert(0), 0);
        assert_eq!(*a.get_or_insert(100), 0);
    }
}