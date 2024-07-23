// Having masks wider, then 64 bit is not viable, 
// since get_dense_index becomes more complex for SIMD registers.
// (no bzhi + popcnt analog for m256)
// Performance drops significantly, and it is cheaper to add more
// depth to a tree, instead.

use std::alloc::{alloc, dealloc, Layout, realloc};
use std::{mem, ptr};
use std::any::TypeId;
use std::marker::PhantomData;
use std::mem::{align_of, MaybeUninit, size_of};
use std::ptr::{addr_of, addr_of_mut, NonNull, null};
use crate::bit_utils::{get_bit_unchecked, set_bit_unchecked};
use crate::{BitBlock, Index};
use crate::const_utils::{const_loop, ConstArray, ConstArrayType, ConstBool, ConstFalse, ConstInteger, ConstTrue, ConstUsize};
use crate::sparse_array::level_indices;
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::utils::{Array, Borrowable, Primitive};

type Mask = u64;

// TODO: On insert check that capacity does not exceeds DataIndex capacity. 
//       Can be usize as well.
type DataIndex = u32;

const DEFAULT_CAP: u8 = 2;

/// Just for safety
trait NodeChild: 'static{}
impl NodeChild for NodePtr{} 
impl NodeChild for DataIndex{}

fn empty_branch() -> &'static [EmptyNode] {
    macro_rules! gen_empty_branch {
        ($name:ident = $len:literal : $($is: literal),*) => {
            static $name: [EmptyNode; $len] = [
                $(
                    EmptyNode {
                        mask: 0,
                        capacity: 1,
                        len: 1,
                        children_placeholder: [&$name[$is] as *const EmptyNode as *const u8],
                    },           
                )*
                EmptyNode {
                    mask: 0,
                    capacity: 1,
                    len: 1,
                    children_placeholder: [unsafe{mem::zeroed()}]
                }                
            ];
        };
    }
    gen_empty_branch!(EMPTY_BRANCH = 9: 1,2,3,4,5,6,7,8);
    &EMPTY_BRANCH
}

#[inline]
fn empty_node<N: ConstInteger, const DEPTH: usize>() -> NodePtr {
    /*const*/ let empty_branch = empty_branch();
    let ptr = &empty_branch[(empty_branch.len() - DEPTH) + N::VALUE];
    NodePtr(unsafe{ mem::transmute(ptr) })
}

#[repr(C)]
struct NodeHeaderN<const N: usize> {
    mask: Mask,
    
    capacity: u8,
    len: u8,
    
    /// NonNull<Node> / DataIndex
    /// 
    /// Always have one element more than specified by mask. 
    /// Last excess element = empty_node/0.
    /// We need that, so get_dense_index(index) point to valid node, even
    /// if `index` points past the mask's bit population (popcnt).
    children_placeholder: [*const u8; N]
}

type EmptyNode = NodeHeaderN<1>;
unsafe impl Sync for EmptyNode{}    // Need this for static EMPTY_NODE

type NodeHeader = NodeHeaderN<0>;
impl NodeHeader{
    /// `index` must be set. Otherwise, return unspecified number that is
    /// less or equal to mask population.
    #[inline]
    unsafe fn get_dense_index(&self, index: usize) -> usize {
        let block = if cfg!(target_feature = "bmi2") {
            core::arch::x86_64::_bzhi_u64(self.mask, index as u32)
        } else {
            let mask = !(u64::MAX << index);
            self.mask & mask
        };
        block.count_ones() as usize
    }        
}

/// Pass by value
#[repr(transparent)]
#[derive(Copy, Clone)]
struct NodePtr(NonNull<NodeHeader>);

impl NodePtr {
    #[inline]
    pub fn header<'a> (self) -> &'a NodeHeader {
        unsafe{ self.0.as_ref() }
    }
    
    #[inline]
    fn header_mut<'a>(mut self) -> &'a mut NodeHeader {
        unsafe{ self.0.as_mut() }
    }
    
    #[inline]
    const fn children_addr_offset() -> usize {
        mem::offset_of!(NodeHeader, children_placeholder)
    }    
 
    #[inline]
    unsafe fn children_ptr<T: NodeChild>(self) -> *const T {
        let ptr: *const u8 = self.0.as_ptr().cast();
        ptr.add(Self::children_addr_offset()) as _
    }    
    
    #[inline]
    unsafe fn children_mut_ptr<T: NodeChild>(mut self) -> *mut T {
        let ptr: *mut u8 = self.0.as_ptr().cast();
        ptr.add(Self::children_addr_offset()) as _
    }     
    
/*    // TODO: remove
    #[inline]
    unsafe fn header_and_children_mut<'a, T: NodeChild>(mut self) 
        -> (&'a mut NodeHeader, *mut T) 
    {
        let ptr: *mut u8 = self.0.as_ptr().cast();
        (
            self.header_mut(),
            ptr.add(Self::children_addr_offset()) as _
        )
    }*/
    
    #[inline]
    pub unsafe fn get_child<'a, T: NodeChild>(self, index: usize) -> &'a T {
        let dense_index = self.header().get_dense_index(index);
        &*self.children_ptr::<T>().add(dense_index)
    }
    
    #[inline]
    pub unsafe fn get_child_mut<'a, T: NodeChild>(self, index: usize) -> &'a mut T {
        let dense_index = self.header().get_dense_index(index);
        &mut*self.children_mut_ptr::<T>().add(dense_index)
    }
    
    #[inline]
    pub unsafe fn children_iter<'a, T: NodeChild + 'a>(self) 
        -> impl Iterator<Item = &'a T>
    {
        self.header().mask.into_bits_iter()
            .map(move |i| self.get_child(i) )
    }
    
    #[inline]
    pub unsafe fn children_mut_iter<'a, T: NodeChild + 'a>(mut self) 
        -> impl Iterator<Item = &'a mut T>
    {
        //let (header, children_ptr) = self.header_and_children_mut::<T>();
        self.header().mask.into_bits_iter()
            .map(move |i| {
                let dense_index = self.header().get_dense_index(i);
                let children_ptr = self.children_mut_ptr::<T>();
                &mut*children_ptr.add(dense_index)
            } )
    }
    
    #[inline]
    fn layout<T: NodeChild>(cap: u8) -> Layout {
        let array_size = size_of::<T>() * cap as usize;
        let size = Self::children_addr_offset() + array_size;
        
        unsafe {
            Layout::from_size_align_unchecked(size, align_of::<NodeHeader>())
            .pad_to_align()
        }
    }
    
    #[inline]
    pub fn new<T: NodeChild>(cap: u8, empty_child: T) -> Self {
        unsafe {
            let node = alloc(Self::layout::<T>(cap)) as *mut NodeHeader;
            
            addr_of_mut!((*node).mask).write(Mask::default());
            debug_assert!(cap>=1);
            addr_of_mut!((*node).capacity).write(cap);
            addr_of_mut!((*node).len).write(1);
            
            // empty_child will always be the last one.
            // Right after real childs.
            let mut this = Self(NonNull::new_unchecked(node));
            this.children_mut_ptr::<T>().write(empty_child);
            this
        }
    }
    
    #[inline]
    pub fn contains(self, index: usize) -> bool {
        unsafe{ get_bit_unchecked(self.header().mask, index) }
    }
    
    /// Returns a new pointer if relocation happened.
    /// 
    /// # Safety
    /// - `T` must match stored data.
    /// - `index` must be in range.
    #[inline]
    pub unsafe fn insert<T: NodeChild>(mut self, index: usize, value: T)
        // TODO: try Option
        -> (NonNull<T>, /*Option<*/Self/*>*/) 
    {
        // TODO: special case for full-size mode
        
        /* realloc */ {         
            let node = self.header();
            let capacity = node.capacity;
            if node.len == capacity {
                let new_capacity = capacity * 2;
                let new_ptr= realloc(
                    self.0.as_ptr() as *mut u8,
                    Self::layout::<T>(capacity),
                    Self::layout::<T>(new_capacity).size(),
                ) as *mut NodeHeader;
                (*new_ptr).capacity = new_capacity; 
                self.0 = NonNull::new_unchecked(new_ptr);
            }
        }
        
        let header = self.header_mut();
        set_bit_unchecked::<true, _>(&mut header.mask, index);
        let dense_index = header.get_dense_index(index);
        
        /* move right */ 
        let p: *mut T = self.children_mut_ptr::<T>().add(dense_index);
        // Shift everything over to make space. (Duplicating the
        // `index`th element into two consecutive places.)
        ptr::copy(p, p.offset(1), header.len as usize - dense_index);
        // Write it in, overwriting the first copy of the `index`th
        // element.
        ptr::write(p, value);
        
        header.len += 1;

        (NonNull::new_unchecked(p), self)
    }
    
    #[inline]
    pub unsafe fn remove<T: NodeChild>(mut self, index: usize){
        let header = self.header_mut();
        let dense_index = header.get_dense_index(index);
        set_bit_unchecked::<false, _>(&mut header.mask, index);        

        /* move left */
        let p: *mut _ = self.children_mut_ptr::<T>().add(dense_index);
        ptr::copy(p.offset(1), p, header.len as usize - dense_index - 1);
        
        header.len -= 1;
    }
    
    /// Deallocate node WITHOUT deallocating child objects.
    #[inline]
    pub unsafe fn drop_node<T: NodeChild>(self){
        let capacity = self.0.as_ref().capacity;
        let layout = Self::layout::<T>(capacity);
        dealloc(self.0.as_ptr().cast(), layout);
    }
    
    // TODO: move out
    // Does compiler unroll this?
    #[inline(always)]
    pub unsafe fn drop_node_with_childs<
        N: ConstInteger,
        const LEVELS_COUNT: usize
    > (mut self)
    {
        /*const*/ if N::VALUE == LEVELS_COUNT - 1 {
            self.drop_node::<DataIndex>();
        } else {
            self.children_mut_iter()
                .for_each(|child: &mut NodePtr|{
                    child.drop_node_with_childs::<N::Inc, LEVELS_COUNT>()
                });
            self.drop_node::<NodePtr>();
        }
    }
}

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
pub struct CompactSparseArray<T, const DEPTH: usize>{
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

impl<T, const DEPTH: usize> Default for CompactSparseArray<T, DEPTH>{
    #[inline]
    fn default() -> Self {
        let mut data: Vec<T> = Vec::with_capacity(1);
        unsafe{ data.set_len(1); }

        Self{
            root: if DEPTH == 1 {
                NodePtr::new::<DataIndex>(DEFAULT_CAP, 0)                
            } else {
                NodePtr::new::<NodePtr>(DEFAULT_CAP, empty_node::<ConstUsize<1>, DEPTH>())
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
                node = if node_ptr.contains(inner_index) {
                    node_ptr.get_child_mut(inner_index)
                } else {
                    // TODO: insert node with already inserted ONE element 
                    //       all down below. And immediately exit loop?
                    //       BENCHMARK change.

                    // update a child pointer with a (possibly) new address
                    let (mut inserted_ptr, new_node) =
                        if N == DEPTH-2 /* terminal node */ {
                            node_ptr.insert( inner_index, NodePtr::new::<DataIndex>(DEFAULT_CAP, 0) )
                        } else {
                            let empty_child = empty_node::<<ConstUsize<N> as ConstInteger>::Inc, DEPTH>();
                            node_ptr.insert( inner_index, NodePtr::new::<NodePtr>(DEFAULT_CAP, empty_child) )
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
            
            let data_index = if node_ptr.contains(inner_index) {
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
    
    /// As long as container not empty - will always point to some valid nodes.
    /// 
    /// First level skipped in return, since it is always root node.
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
                if terminal_node.header().len == 1 /* TODO: unlikely */ {
                    terminal_node.drop_node::<DataIndex>();

                    // climb up the tree, and remove empty nodes
                    const_loop!(N in 0..{DEPTH-1} rev => 'out: {
                        let node = if N == 0 {
                            self.root
                        } else {
                            branch.as_ref()[N-1]
                        };
                        
                        node.remove::<NodePtr>(indices.as_ref()[N]);
                        if node.header().len != 1 {
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
}

impl<T, const DEPTH: usize> Drop for CompactSparseArray<T, DEPTH> {
    #[inline]
    fn drop(&mut self) {
        unsafe{ 
            // drop values
            let slice = ptr::slice_from_raw_parts_mut(
                self.data.as_mut_ptr().add(1), 
                self.data.len() - 1
            ); 
            ptr::drop_in_place(slice);
            self.data.set_len(0);
            
            // drop node hierarchy
            self.root.drop_node_with_childs::<ConstUsize<0>, DEPTH>() 
        };
    }
}

impl<T, const DEPTH: usize> SparseHierarchy for CompactSparseArray<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{
    const EXACT_HIERARCHY: bool = true;
    
    type LevelCount = ConstUsize<DEPTH>;
    
    type LevelMaskType = Mask;
    type LevelMask<'a> = &'a Mask where Self: 'a;
    
    type DataType = T;
    type Data<'a> = &'a T where Self: 'a;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<Self::Data<'_>> 
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
        -> Self::Data<'_> 
    {
        self.data(index, level_indices).unwrap_unchecked()
    }
    
    type State = State<T, DEPTH>;
}

pub struct State<T, const DEPTH: usize>
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
    phantom_data: PhantomData<T>
}

impl<T, const DEPTH: usize> SparseHierarchyState for State<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger
{
    type This = CompactSparseArray<T, DEPTH>;

    #[inline]
    fn new(_: &Self::This) -> Self {
        Self{
            level_nodes: Array::from_fn(|_|None),
            phantom_data: Default::default(),
        }
    }
    
    #[inline]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, 
        this: &'a Self::This, 
        level_n: N, 
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
        if N::VALUE == 0 {
            return &this.root.header().mask;
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
        let contains = prev_node.contains(level_index); 
        let node = *prev_node.get_child::<NodePtr>(level_index);
        // This is not a branch!
        let node = if contains{ node } else { empty_node::<N, DEPTH>() };   
        *self.level_nodes.as_mut().get_unchecked_mut(level_node_index) = Some(node);
        
        &node.header().mask
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self, 
        this: &'a Self::This, 
        level_n: N, 
        level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
        if N::VALUE == 0 {
            return &this.root.header().mask;
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
        
        &node.header().mask
    }
    
    // TODO: data_or_default possible too.
    
    #[inline]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> Option<<Self::This as SparseHierarchy>::Data<'a>> 
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

        if node.contains(level_index) {
            let data_index = node.get_child::<DataIndex>(level_index).as_usize();
            Some(this.data.get_unchecked(data_index))
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::Data<'a> 
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

impl<T, const DEPTH: usize> Borrowable for CompactSparseArray<T, DEPTH>{ type Borrowed = Self; }

#[cfg(test)]
mod test{
    use std::{mem::ManuallyDrop, ptr::NonNull};
    use std::slice;
    use itertools::assert_equal;
    use crate::sparse_hierarchy::SparseHierarchy;
    use crate::utils::Primitive;
    use super::{CompactSparseArray, NodeHeader};
    
    #[test]
    fn test(){
        let mut a: CompactSparseArray<usize, 3> = Default::default();
        assert_eq!(a.get(15), None);

        *a.get_or_insert(15) = 89;
        assert_eq!(a.get(15), Some(&89));

        *a.get_or_insert(16) = 90;
        assert_eq!(a.get(20), None);
                
        assert_eq!(*a.get_or_insert(15), 89);
        assert_eq!(*a.get_or_insert(0), 0);
        assert_eq!(*a.get_or_insert(100), 0);
    }
    
    #[test]
    fn test2(){
        let mut a: CompactSparseArray<usize, 3> = Default::default();

        #[cfg(not(miri))]
        const COUNT: usize = 200_000;
        #[cfg(miri)]
        const COUNT: usize = 200;
        
        for i in 0..COUNT{
            *a.get_or_insert(i) = i;
        }
        
        for i in 0..COUNT{
            let v = a.get(i);
            assert_eq!(v, Some(&i));
        }
        
        for i in 0..COUNT{
            a.remove(i);
        }

        for i in 0..COUNT{
            let v = a.get(i);
            assert_eq!(v, None);
        }
    }
    
    #[test]
    fn test_remove(){
        let mut a: CompactSparseArray<usize, 2> = Default::default();
        *a.get_or_insert(10) = 10;
        *a.get_or_insert(11) = 11;
        //*a.get_or_insert(12) = 12;
        
        a.remove(11);
        a.remove(10);
    }    

    #[test]
    fn test_remove2(){
        let mut a: CompactSparseArray<usize, 2> = Default::default();

        for i in 0..2000{
            *a.get_or_insert(i) = i;
        }

        a.remove(0).unwrap();
        a.remove(1).unwrap();
    }        
}