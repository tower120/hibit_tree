// Having masks wider, then 64 bit is not viable, 
// since get_dense_index becomes more complex for SIMD registers.
// (no bzhi + popcnt analog for m256)
// Performance drops significantly, and it is cheaper to add more
// depth to a tree, instead.

#[cfg(test)]
mod tests;

mod from;
mod node;

use std::{mem, ptr};
use std::marker::PhantomData;
use crate::{BitBlock, Index, BitmapTreeCursorTypes, BitmapTreeTypes};
use crate::bit_queue::BitQueue;
use crate::const_utils::{const_loop, ConstArray, ConstArrayType, ConstBool, ConstFalse, ConstInteger, ConstTrue, ConstUsize};
use crate::level_indices;
use crate::sparse_hierarchy::{BitmapTree, BitmapTreeCursor};
use crate::utils::{Array, Borrowable, Primitive, Take};

use node::{NodePtr, empty_node};

type Mask = u64;

// TODO: On insert check that capacity does not exceeds DataIndex capacity. 
//       Can be usize as well.
type DataIndex = u32;

/// TODO: Description
/// 
/// Only 64bit wide version available[^64bit_only].
/// 
/// [^64bit_only]: TODO explain why
/// 
/// # Performance
/// 
/// With `bmi2` enabled, access operations almost identical to [SparseArray].
/// Insert and remove operations are slower, since they need to keep child nodes 
/// in order. TODO: swap to non-compressed after certain threshold to amortize this.
///
/// # `target-feature`s
/// 
/// ## x86
/// `popcnt`, `bmi1`, `bmi2`
/// 
/// ## arm
/// `neon`
/// 
/// ## risc-v
/// ???
/// 
/// In addition, to lib's `popcnt` and `bmi1` requirement, on x86 arch
/// CompactSparseArray also benefits from `bmi2`'s [bzhi] instruction.
pub struct CompactSparseArray<T, const DEPTH: usize>
where
    ConstUsize<DEPTH>: ConstInteger
{
    root: NodePtr,
    
    /// First item - is a placeholder for non-existent/default element.
    data: Vec<T>,
    keys: Vec<usize>,
    
    /*// TODO: Make this technique optional? Since it consumes additional mem.
    /// Position in terminal node, that points to `data` with this vec's index.
    ///
    /// Used by remove(). Speedup getting of item that is swapped with.
    /// Without this - we would have to traverse to its terminal node through the tree, 
    /// to get by index.
    terminal_node_positions: Vec<(NodePtr/*terminal_node*/, usize/*in-node index*/)>,*/
}

impl<T, const DEPTH: usize> Default for CompactSparseArray<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{
    #[inline]
    fn default() -> Self {
        // TODO: reuse somehow?
        let mut data: Vec<T> = Vec::with_capacity(1);
        unsafe{ data.set_len(1); }

        Self{
            root: if DEPTH == 1 {
                NodePtr::new::<DataIndex>(node::DEFAULT_CAP, 0)                
            } else {
                NodePtr::new::<NodePtr>(node::DEFAULT_CAP, empty_node(ConstUsize::<1>, ConstUsize::<DEPTH>))
            },
            data,
            keys: vec![usize::MAX]
        }
    }
}

impl<T, const DEPTH: usize> CompactSparseArray<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger    
{
    #[inline]
    fn get_or_insert_impl(
        &mut self, 
        index: usize,
        insert: impl ConstBool,
        value_fn: impl FnOnce() -> T
    ) -> &mut T {
        let indices = level_indices::<Mask, ConstUsize<DEPTH>>(index);
        
        // get terminal node pointing to data
        let mut node = &mut self.root;
        const_loop!(N in 0..{DEPTH-1} => {
            let inner_index = indices.as_ref()[N];
            unsafe{
                let mut node_ptr = *node;
                /*child*/ node = if node_ptr.header().contains(inner_index) {
                    node_ptr.get_child_mut(inner_index)
                } else {
                    // TODO: insert node with already inserted ONE element 
                    //       all down below. And immediately exit loop?
                    //       BENCHMARK change.

                    // update a child pointer with a (possibly) new address
                    let (mut inserted_ptr, new_node) =
                        if N == DEPTH-2 /* child node is terminal */ {
                            node_ptr.insert( inner_index, NodePtr::new::<DataIndex>(node::DEFAULT_CAP, 0) )
                        } else {
                            // N + 2, because we point from child, and to it's child
                            let empty_child = empty_node(ConstUsize::<N>.inc().inc(), ConstUsize::<DEPTH>);
                            node_ptr.insert( inner_index, NodePtr::new::<NodePtr>(node::DEFAULT_CAP, empty_child) )
                        };
                    *node = new_node;   // This is actually optional
                    inserted_ptr.as_mut()
                }
            }
        });
     
        // now fetch data
        unsafe{
            let node_ptr = *node;
            let inner_index = *indices.as_ref().last().unwrap_unchecked();
            
            let data_index = if node_ptr.header().contains(inner_index) {
                let data_index = node_ptr.get_child::<DataIndex>(inner_index).as_usize();
                /*const*/ if insert.value(){
                    *self.data.get_unchecked_mut(data_index) = value_fn();                     
                }
                data_index
            } else {
                let i = self.data.len(); 
                self.data.push(value_fn());
                self.keys.push(index);
                let (_, new_node) = node_ptr.insert(inner_index, i as DataIndex);
                *node = new_node;
                i
            };
            self.data.get_unchecked_mut(data_index)
        }
    }    
    
    pub fn get_or_insert(&mut self, index: impl Into<Index<Mask, ConstUsize<DEPTH>>>) -> &mut T
    where
        T: Default
    {
        let index: usize = index.into().into();
        self.get_or_insert_impl(index, ConstFalse, || T::default())
    }
    
    pub fn insert(&mut self, index: impl Into<Index<Mask, ConstUsize<DEPTH>>>, value: T) {
        let index: usize = index.into().into();
        self.get_or_insert_impl(index, ConstTrue, ||value);
    }
    
    /// As long as container not empty - will always point to **SOME** valid
    /// node sequence.
    /// 
    /// First level skipped in return, since it is always a root node.
    #[inline]
    unsafe fn get_branch(&self, level_indices: &impl ConstArray<Item=usize>) 
        -> ConstArrayType<NodePtr, <ConstUsize<DEPTH> as ConstInteger>::Dec> 
    {
        let mut out = ConstArrayType::<NodePtr, <ConstUsize<DEPTH> as ConstInteger>::Dec>
                        ::uninit_array();
        
        let mut node_ptr = self.root;
        for n in 0..DEPTH-1 {
            let inner_index = level_indices.as_ref()[n];
            node_ptr = unsafe{ *node_ptr.get_child(inner_index) };
            out.as_mut()[n].write(node_ptr);
        }
        
        // Should be just `transmute`, but we have "dependent type". 
        // `transmute_unchecked` / `assume_init_array` when available. 
        mem::transmute_copy(&out)
    }

    /// As long as container not empty - will always point to some valid (node, in-node-index).
    #[inline(always)]
    unsafe fn get_terminal_node(&self, indices: &[usize]) -> (NodePtr, usize) {
        //let indices = level_indices::<Mask, ConstUsize<DEPTH>>(index);
        let mut node_ptr = self.root;
        for n in 0..DEPTH-1 {
            let inner_index = *indices.as_ref().get_unchecked(n);
            node_ptr = unsafe{ *node_ptr.get_child(inner_index) };
        }
        let terminal_inner_index = *indices.as_ref().last().unwrap_unchecked();
        (node_ptr, terminal_inner_index)
    }      
    
    #[inline]
    pub fn remove(&mut self, index: impl Into<Index<Mask, ConstUsize<DEPTH>>>) -> Option<T>{
        let index: usize = index.into().into();
        
        unsafe{
            let indices = level_indices::<Mask, ConstUsize<DEPTH>>(index);
            let branch = self.get_branch(&indices);

            let terminal_node = branch.as_ref().last().unwrap_unchecked();
            let terminal_inner_index = *indices.as_ref().last().unwrap_unchecked();

            let data_index = terminal_node.get_child::<DataIndex>(terminal_inner_index).as_usize();
            
            if *self.keys.get_unchecked(data_index) == index {
                terminal_node.remove::<DataIndex>(terminal_inner_index);

                // 1. Try remove empty terminal node recursively.
                if terminal_node.header().len() == 1 /* TODO: unlikely */ {
                    terminal_node.drop_node::<DataIndex>();

                    // climb up the tree, and remove empty nodes
                    const_loop!(N in 0..{DEPTH-1} rev => 'out: {
                        let node = if N == 0 {
                            self.root
                        } else {
                            branch.as_ref()[N-1]
                        };
                        
                        node.remove::<NodePtr>(indices.as_ref()[N]);
                        if node.header().len() != 1 {
                            break 'out;
                        } 
                        
                        /*const*/ if N != 0 /*don't touch root*/ {
                            node.drop_node::<NodePtr>();
                        }                        
                    });
                }

                // 2. Swap remove key + update swapped terminal node
                {
                    let last_key = self.keys.pop().unwrap_unchecked();
                    if last_key != index {
                        *self.keys.get_unchecked_mut(data_index) = last_key;

                        let indices = level_indices::<Mask, ConstUsize<DEPTH>>(last_key);
                        let (node, inner_index) = self.get_terminal_node(indices.as_ref());                    
                        *node.get_child_mut::<DataIndex>(inner_index) = data_index as DataIndex; 
                    }    
                }
                
                // 3. Remove data        
                let value = self.data.swap_remove(data_index);                
                Some(value)                
            } else {
                None
            }
        }   
    }

    /// Key-values in arbitrary order.
    #[inline]
    pub fn key_values(&self) -> (&[usize], &[T]) {
        // skip first element
        unsafe{
            (
                self.keys.as_slice().get_unchecked(1..), 
                self.data.as_slice().get_unchecked(1..)
            )
        }        
    }

    /// Key-values in arbitrary order.
    #[inline]
    pub fn key_values_mut(&mut self) -> (&[usize], &mut [T]) {
        // skip first element
        unsafe{
            (
                self.keys.as_slice().get_unchecked(1..), 
                self.data.as_mut_slice().get_unchecked_mut(1..)
            )
        }
    }
    
    #[inline]
    unsafe fn drop_impl(&mut self){
        // drop values
        let slice = ptr::slice_from_raw_parts_mut(
            self.data.as_mut_ptr().add(1), 
            self.data.len() - 1
        ); 
        ptr::drop_in_place(slice);
        self.data.set_len(0);
        
        // drop node hierarchy
        self.root.drop_node_with_childs::<ConstUsize<0>, DEPTH>()         
    }
}

#[cfg(feature = "may_dangle")]
unsafe impl<#[may_dangle] T, const DEPTH: usize> Drop for CompactSparseArray<T, DEPTH> {
    #[inline]
    fn drop(&mut self) {
        unsafe{ self.drop_impl(); }
    }
}

#[cfg(not(feature = "may_dangle"))]
impl<T, const DEPTH: usize> Drop for CompactSparseArray<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{
    #[inline]
    fn drop(&mut self) {
        unsafe{ self.drop_impl(); }
    }
}

impl<'a, T, const DEPTH: usize> BitmapTreeTypes<'a> for CompactSparseArray<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{
    type Data = &'a T;
    type DataUnchecked = &'a T;
    type Cursor = Cursor<'a, T, DEPTH>;    
}

impl<T, const DEPTH: usize> BitmapTree for CompactSparseArray<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{
    const EXACT_HIERARCHY: bool = true;
    
    type LevelCount = ConstUsize<DEPTH>;

    type LevelMask = Mask;
    
    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<&T> 
    {
        let (node, inner_index) = self.get_terminal_node(level_indices);
        // point to SOME valid item
        let data_index = node.get_child::<DataIndex>(inner_index).as_usize();

        // The element may be wrong thou, so we check its index.
        if *self.keys.get_unchecked(data_index) == index {
            Some(self.data.get_unchecked(data_index))
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> &T 
    {
        self.data(index, level_indices).unwrap_unchecked()
    }
}

pub struct Cursor<'src, T, const DEPTH: usize>
where
    ConstUsize<DEPTH>: ConstInteger
{
    /// [*const Node; Levels::LevelCount-1]
    /// 
    /// Level0 skipped - we can get it from self/this.
    level_nodes: ConstArrayType<
        Option<NodePtr>, 
        <ConstUsize<DEPTH> as ConstInteger>::Dec
    >,     
    phantom_data: PhantomData<&'src T>
}

impl<'this, 'src, T, const DEPTH: usize> BitmapTreeCursorTypes<'this> for Cursor<'src, T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{
    type Data = &'src T;
}

impl<'src, T, const DEPTH: usize> BitmapTreeCursor<'src> for Cursor<'src, T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{
    type Src = CompactSparseArray<T, DEPTH>;

    #[inline]
    fn new(_: &'src Self::Src) -> Self {
        Self{
            level_nodes: Array::from_fn(|_|None),
            phantom_data: Default::default(),
        }
    }
    
    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self,
        src: &'src Self::Src,
        level_n: N,
        level_index: usize
    ) -> <Self::Src as BitmapTree>::LevelMask {
        if N::VALUE == 0 {
            return *src.root.header().mask();
        }
        
        // We do not store the root level's node.
        let level_node_index = level_n.dec().value();
        
        // 1. get &Node from prev level.
        let prev_node = if N::VALUE == 1 {
            src.root
        } else {
            self.level_nodes.as_ref().get_unchecked(level_node_index - 1).unwrap_unchecked()
        };
        
        // 2. store *node in state cache
        let contains = prev_node.header().contains(level_index); 
        let node = *prev_node.get_child::<NodePtr>(level_index);
        // This is not a branch!
        let node = if contains{ node } else { empty_node(level_n, ConstUsize::<DEPTH>) };   
        *self.level_nodes.as_mut().get_unchecked_mut(level_node_index) = Some(node);
        
        *node.header().mask()
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self,
        this: &'src Self::Src,
        level_n: N,
        level_index: usize
    ) -> <Self::Src as BitmapTree>::LevelMask {
        if N::VALUE == 0 {
            return *this.root.header().mask();
        }
        
        // We do not store the root level's node.
        let level_node_index = level_n.dec().value();
        
        // 1. get &Node from prev level.
        let prev_node = if N::VALUE == 1 {
            this.root
        } else {
            self.level_nodes.as_ref().get_unchecked(level_node_index - 1).unwrap_unchecked()
        };
        
        // 2. store *node in state cache
        let node = *prev_node.get_child::<NodePtr>(level_index);
        *self.level_nodes.as_mut().get_unchecked_mut(level_node_index) = Some(node);
        
        *node.header().mask()
    }
    
    // TODO: data_or_default possible too.
    
    #[inline]
    unsafe fn data<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> Option<<Self as BitmapTreeCursorTypes<'a>>::Data> 
    {
        let node = if DEPTH == 1{
            // We do not store the root level's block.
            this.root
        } else {
            self.level_nodes.as_ref().last().unwrap_unchecked().unwrap_unchecked()
        };
        
        /*// default
        let data_index = node.get_child::<DataIndex>(level_index).as_usize() * node.contains(level_index) as usize;
        Some(this.data.get_unchecked(data_index))*/

        if node.header().contains(level_index) {
            let data_index = node.get_child::<DataIndex>(level_index).as_usize();
            Some(this.data.get_unchecked(data_index))
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> <Self as BitmapTreeCursorTypes<'a>>::Data
    {
        let node = if DEPTH == 1{
            // We do not store the root level's block.
            this.root
        } else {
            self.level_nodes.as_ref().last().unwrap_unchecked().unwrap_unchecked()
        };
            
        let data_index = node.get_child::<DataIndex>(level_index).as_usize();
        this.data.get_unchecked(data_index)
    }
}

impl<T, const DEPTH: usize> Borrowable for CompactSparseArray<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{ type Borrowed = Self; }