use std::alloc::{alloc, dealloc, Layout, realloc};
use std::{mem, ptr};
use std::mem::{align_of, MaybeUninit, size_of};
use std::ops::ControlFlow;
use std::ops::ControlFlow::Continue;
use std::ptr::{addr_of, addr_of_mut, NonNull, null};
use arrayvec::ArrayVec;
use crate::bit_utils::{get_bit_unchecked, set_bit_unchecked};
use crate::BitBlock;
use crate::const_utils::ConstInteger;
use super::DataIndex;
use super::Mask;

pub(super) const DEFAULT_CAP: u8 = 2;

/// Just for safety
pub(super) trait NodeChild: 'static{}
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
pub(super) fn empty_node<N: ConstInteger, DEPTH: ConstInteger>(_: N, _: DEPTH) -> NodePtr {
    /*const*/ let empty_branch = empty_branch();
    let ptr = &empty_branch[(empty_branch.len() - DEPTH::VALUE) + N::VALUE];
    NodePtr(unsafe{ mem::transmute(ptr) })
}

#[repr(C)]
pub(super) struct NodeHeaderN<const N: usize> {
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

pub(super) type EmptyNode = NodeHeaderN<1>;
unsafe impl Sync for EmptyNode{}    // Need this for static EMPTY_NODE

pub(super) type NodeHeader = NodeHeaderN<0>;
impl NodeHeader{
    #[inline]
    pub fn len(&self) -> u8 {
        self.len
    }
    
    #[inline]
    pub fn mask(&self) -> &Mask {
        &self.mask
    }
    
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
    
    #[inline]
    pub fn contains(&self, index: usize) -> bool {
        unsafe{ get_bit_unchecked(self.mask, index) }
    }    
}

/// Pass by value
#[repr(transparent)]
#[derive(Copy, Clone)]
pub(super) struct NodePtr(NonNull<NodeHeader>);

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
    pub unsafe fn from_parts<T: NodeChild>(
        mask: Mask,
        childs: &[T],
        empty_child: T, 
    ) -> Self {
        let len = childs.len();
        let cap = (len + 1) as u8; 
        let node = alloc(Self::layout::<T>(cap)) as *mut NodeHeader;
        
        addr_of_mut!((*node).mask).write(mask);
        addr_of_mut!((*node).capacity).write(cap);
        addr_of_mut!((*node).len).write(len as u8);
        
        let mut this = Self(NonNull::new_unchecked(node));
        
        // copy childs
        ptr::copy_nonoverlapping(
            childs.as_ptr(),
            this.children_mut_ptr::<T>(),
            len
        );
        
        // add empty_child at the end.
        this.children_mut_ptr::<T>().add(len).write(empty_child);
        
        this
    }    
    
    #[inline]
    pub fn raw_new<T: NodeChild>(cap: u8, mask: Mask) -> MaybeUninit<Self> {
        unsafe {
            let node = alloc(Self::layout::<T>(cap)) as *mut NodeHeader;
            
            addr_of_mut!((*node).mask).write(mask);
            addr_of_mut!((*node).capacity).write(cap);
            addr_of_mut!((*node).len).write(0);
            
            MaybeUninit::new(
                Self(NonNull::new_unchecked(node))
            )
        }
    }
    
    /// Mask will not be updated.
    ///
    /// # Safety
    ///
    /// * `index` must point at an element that will be last.
    /// * Must be within capacity.
    #[inline]
    pub unsafe fn raw_push_within_capacity<T: NodeChild>(
        mut this: MaybeUninit<Self>, index: usize, value: T
    ) {
        let this = this.assume_init_mut(); 
        let header = this.header_mut();
        
        //set_bit_unchecked::<true, _>(&mut header.mask, index);
        
        let p: *mut T = this.children_mut_ptr::<T>().add(header.len as usize);
        p.write(value);
        
        header.len += 1;
    }
    
    /// Adds `empty_child` as last element, and makes `this` initialized.
    #[inline]
    pub unsafe fn raw_finalize<T: NodeChild>(
        this: MaybeUninit<Self>, empty_child: T
    ) -> Self {
        let this = this.assume_init();
        let header = this.header_mut();
        
        let p: *mut T = this.children_mut_ptr::<T>().add(header.len as usize);
        p.write(empty_child);
        
        header.len += 1;
        debug_assert!(header.len <= header.capacity);

        this
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
