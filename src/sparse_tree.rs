use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::ops::ControlFlow::{Break, Continue};
use std::ptr;
use std::ptr::{NonNull, null};
use crate::bit_block::BitBlock;
use crate::utils::Borrowable;
use crate::level_block::HiBlock;
use crate::level::ILevel;
use crate::const_utils::const_int::{ConstInteger, ConstIntVisitor, ConstUsize};
use crate::const_utils::const_array::{ConstArray, ConstArrayType, ConstCopyArrayType};
use crate::const_utils::{const_loop, ConstBool, ConstFalse, ConstTrue};
use crate::{Empty, Index, HibitTreeCursorTypes, HibitTreeTypes};
use crate::req_default::{DefaultInit, DefaultInitFor, DefaultRequirement, ReqDefault};
use crate::utils::Primitive;
use crate::utils::Array;
use crate::sparse_tree_levels::{FoldMutVisitor, FoldVisitor, MutVisitor, SparseTreeLevels, TypeVisitor, Visitor};
use crate::hibit_tree::{HibitTree, HibitTreeCursor};

/// Uncompressed Hierarchical Bitmap Tree.
///
/// Nodes store children pointers in sparse array. 
/// This means that array size equals to maximum children count (bitmask width).
/// 
/// To save memory, instead of traditional pointers, integer indices
/// are used. Root level use u8 for indices, first level - u16, everything below - u32.
/// 
/// It is slightly faster than [DenseTree], and can have wider nodes with 
/// SIMD-accelerated bitmasks.
///
/// [DenseTree]: crate::DenseTree 
///
/// # `get_or_default`
///
/// Pass [ReqDefault] as `R` argument, to unlock [get_or_default] operation[^1].
/// This will require `Data` to be [Default] to construct container.
///
/// [get_or_default] is as fast as [get_unchecked] and has no
/// performance hit from branching[^2]. 
///
/// [^1]: We can't just treat container with `Data: Default` as `ReqDefault`,
///       due to stable Rust limitations - we need specialization for that.
/// [^2]: All non-existent items just point to the very first default item.
///
/// [get_or_default]: SparseTree::get_or_default 
/// [get_unchecked]: SparseTree::get_unchecked
pub struct SparseTree<Levels, Data, R = ReqDefault<false>>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement
{
    levels: Levels,
    
    // TODO: some kind of multi-vec, to reduce allocation count?
    
    /// First item - is a placeholder for non-existent/default element.
    values: Vec<Data>,
    keys  : Vec<usize>,
    
    // TODO: can be pair of u32's
    // Used only in remove().
    /// Coordinates in last level of pointer to value with this vec index.  
    last_level_block_indices: Vec<(usize/*block_index*/, usize/*in-block index*/)>,
    
    phantom_data: PhantomData<R>
}

impl<Levels, Data, R> Default for
    SparseTree<Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement,
    DefaultInitFor<Data, R>: DefaultInit
{
    #[inline]
    fn default() -> Self {
        let mut values: Vec<Data> = Vec::with_capacity(1);
        unsafe{ values.set_len(1); }
        unsafe{
            <DefaultInitFor<Data, R> as DefaultInit>
            ::init_default(values.as_mut_ptr().cast());
        }
        
        Self{
            levels: Levels::default(),
            
            values, 
            keys  : vec![usize::MAX],
            last_level_block_indices: vec![(0,0)],
            
            phantom_data: PhantomData
        }
    }
}

struct BlockPtr<Levels, LevelN>(NonNull<u8>, PhantomData<*mut (Levels, LevelN)>);

impl<Levels, LevelN> Clone for BlockPtr<Levels, LevelN>{
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<Levels, LevelN> Copy for BlockPtr<Levels, LevelN>{}

impl<Levels, LevelN> BlockPtr<Levels, LevelN>
where
    Levels: SparseTreeLevels,
    LevelN: ConstInteger,
{
    /// # Safety
    /// 
    /// `ptr` must be valid.
    #[inline]
    pub unsafe fn new_unchecked(ptr: NonNull<u8>) -> Self {
        Self(ptr, PhantomData)
    }
    
    #[inline]
    pub fn as_ptr(self) -> *const u8 {
        self.0.as_ptr() as _
    }
    
    #[inline]
    pub unsafe fn get_mask<'a>(self) -> &'a Levels::Mask {
        struct V(*const u8);
        impl<M> TypeVisitor<M> for V{
            type Out = NonNull<M>;
            
            #[inline(always)]
            fn visit<L>(self, _: PhantomData<L>) -> Self::Out 
            where 
                L: ILevel<Block: HiBlock<Mask=M>> 
            {
                unsafe{
                    let block = self.0 as *const L::Block;
                    NonNull::from((*block).mask())
                }
            }
        }
        let mask_ptr = Levels::visit_type(LevelN::default(), V(self.0.as_ptr()));
        unsafe{ mask_ptr.as_ref() }  
    }

    #[inline(always)]
    pub unsafe fn get_child(self, index: usize) -> usize {
        struct V(*const u8, usize);
        impl<M> TypeVisitor<M> for V{
            type Out = usize;

            #[inline(always)]
            fn visit<L>(self, _: PhantomData<L>) -> Self::Out 
            where 
                L: ILevel<Block: HiBlock> 
            {
                unsafe{
                    let block = self.0 as *const L::Block;
                    (*block).get_or_zero(self.1).as_usize()
                }
            }
        }
        Levels::visit_type(LevelN::default(), V(self.0.as_ptr(), index))
    }
    
    #[inline(always)]
    pub unsafe fn insert_child(self, index: usize, item: usize){
        struct V(*mut u8, usize, usize);
        impl<M> TypeVisitor<M> for V{
            type Out = ();
            
            #[inline(always)]
            fn visit<L>(self, _: PhantomData<L>) -> Self::Out 
            where 
                L: ILevel<Block: HiBlock> 
            {
                unsafe{
                    let block = self.0 as *mut L::Block;
                    (*block).insert(self.1, Primitive::from_usize(self.2));
                }
            }
        }
        Levels::visit_type(LevelN::default(), V(self.0.as_ptr(), index, item))        
    }
}

impl<Levels, Data, R> SparseTree<Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement,
{
    #[inline(always)]
    unsafe fn get_block<LevelN>(&self, level_n: LevelN, level_index: usize) 
        -> BlockPtr<Levels, LevelN>
    where
        LevelN: ConstInteger
    {
        struct V(usize);
        impl<M> Visitor<M> for V{
            type Out = *const u8;
            
            #[inline(always)]
            fn visit<I: ConstInteger, L: ILevel>(self, _: I, level: &L) 
                -> Self::Out 
            {
                unsafe {
                    level.blocks().get_unchecked(self.0) as *const _ as *const u8
                }
            }
        }
        let ptr = self.levels.visit(level_n, V(level_index));
        BlockPtr(NonNull::new_unchecked(ptr as *mut _), PhantomData)
    }
    
    #[inline(always)]
    unsafe fn get_block_mut<LevelN>(&mut self, level_n: LevelN, level_index: usize) 
        -> BlockPtr<Levels, LevelN>
    where
        LevelN: ConstInteger
    {
        struct V(usize);
        impl<M> MutVisitor<M> for V{
            type Out = *mut u8;
            
            #[inline(always)]
            fn visit<I: ConstInteger, L: ILevel>(self, _: I, level: &mut L) 
                -> Self::Out 
            {
                unsafe {
                    level.blocks_mut().get_unchecked_mut(self.0) as *mut _ as *mut u8
                }
            }
        }
        let ptr = self.levels.visit_mut(level_n, V(level_index));
        BlockPtr(NonNull::new_unchecked(ptr), PhantomData)
    }
    
    #[inline]
    unsafe fn fetch_block_indices(&self, level_indices: &[usize])
        -> (ConstCopyArrayType<usize, Levels::LevelCount>, usize)
    {
        debug_assert_eq!(level_indices.len(), Levels::LevelCount::VALUE);
        let mut out = Array::from_fn(|_|0);
        struct V<'a, LevelIndices>{
            level_indices: &'a [usize],
            out: &'a mut LevelIndices
        }
        impl<'a, LevelIndices: ConstArray<Item=usize>, M> FoldVisitor<M> for V<'a, LevelIndices> {
            type Acc = usize;
            
            #[inline(always)]
            fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L, level_block_index: Self::Acc) 
                -> ControlFlow<usize, usize> 
            where 
                L: ILevel, L::Block: HiBlock
            {
                unsafe{
                    let block = level.blocks().get_unchecked(level_block_index);
                    let in_block_index = self.level_indices.as_ref().get_unchecked(I::VALUE).as_usize();
                    let block_index = block.get_or_zero(in_block_index).as_usize();
                    *self.out.as_mut().get_unchecked_mut(I::VALUE) = block_index; 
                    Continue(block_index)
                }
            }
        }        
        let last_level_block_index = self.levels.fold(0, V{level_indices, out: &mut out});
        (out, last_level_block_index)
    }
    
    #[inline]
    unsafe fn fetch_block_index(&self, level_indices: &[usize]) -> usize {
        self.fetch_block_indices(level_indices).1
    }
    
    /// Returns `Some(item)` if there is an element at `index` in container. `None` otherwise. 
    pub fn remove(&mut self, index: impl Into<Index<Levels::Mask, Levels::LevelCount>>) 
        -> Option<Data> 
    {
        let index: usize = index.into().into();

        let level_indices = crate::level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let (levels_block_indices, data_block_index) = unsafe { 
            self.fetch_block_indices(level_indices.as_ref()) 
        };
        
        if data_block_index == 0 {
            return None;
        }
        
        // 1. Update level masks
        self.levels.fold_rev_mut((), V{level_indices, levels_block_indices});
        struct V<LI, LBI>{
            level_indices: LI,
            levels_block_indices: LBI
        }
        impl<LI, LBI, M> FoldMutVisitor<M> for V<LI, LBI>
        where
            LI : ConstArray<Item=usize>,
            LBI: ConstArray<Item=usize>
        {
            type Acc = ();
            
            #[inline(always)]
            fn visit<I: ConstInteger, L>(&mut self, level_number: I, level: &mut L, acc: Self::Acc)
                 -> ControlFlow<Self::Acc, Self::Acc>  
            where 
                L: ILevel, L::Block: HiBlock<Mask=M> 
            {
                let block_index = if level_number.value() == 0 {
                    0
                } else {
                    self.levels_block_indices.as_ref()[level_number.dec().value()]
                };
                let level_block = unsafe{ level.blocks_mut().get_unchecked_mut(block_index) };
                unsafe{
                    let inner_index = self.level_indices.as_ref()[level_number.value()];
                    level_block.remove_unchecked(inner_index);
                }
                
                if level_block.is_empty() {
                    if level_number.value() != 0 /*if not root level*/ {
                        unsafe{ level.remove_empty_block_unchecked(block_index); }
                    }
                    Continue(())
                } else {
                    Break(())
                }
            }
        }
        
        // 3. Update index in last level block. 
        unsafe{
            // swap_remove(data_block_index)
            let last = self.last_level_block_indices.pop().unwrap_unchecked();
            if self.last_level_block_indices.len() > data_block_index {
                *self.last_level_block_indices.get_unchecked_mut(data_block_index) = last;
                let (level_block_index, inner_index) = last;
                
                self.levels.visit_mut(Levels::LevelCount::default().dec(), V{level_block_index, inner_index, data_block_index});
                struct V {
                    level_block_index: usize,
                    inner_index: usize,
                    data_block_index: usize
                }
                impl<M> MutVisitor<M> for V {
                    type Out = ();
    
                    #[inline(always)]
                    fn visit<I: ConstInteger, L>(self, _: I, level: &mut L) -> Self::Out
                    where
                        L: ILevel<Block: HiBlock>,                
                    {
                        unsafe{
                            let level_block = level.blocks_mut().get_unchecked_mut(self.level_block_index);                            
                            level_block.set_unchecked(self.inner_index, Primitive::from_usize(self.data_block_index));
                        }
                    }
                }                    
            }
        }
        
        // 2. Remove data        
        self.keys.swap_remove(data_block_index);
        let value = self.values.swap_remove(data_block_index);
        
        Some(value)
    }
    
    /// Returns mutable reference to item at `index`, if exists.
    /// Inserts and return [Default], otherwise.
    pub fn get_or_insert(&mut self, index: impl Into<Index<Levels::Mask, Levels::LevelCount>>) -> &mut Data
    where
        Data: Default
    {
        let index: usize = index.into().into();
        self.get_or_insert_impl(index, ConstFalse, ||Data::default())
    }

    /// Inserts `value` at `index`.
    /// If there was a value - it will be replaced.
    ///
    /// Somewhat faster than *[get_or_insert()] = `value`, since it will not insert intermediate
    /// default value [^1], if `index` unoccupied.
    /// 
    /// [get_or_insert()]: Self::get_or_insert()
    ///
    /// [^1]: Thou, if empty constructor is not complex - compiler may be 
    /// able to optimize away intermediate value anyway. But better safe then sorry.
    pub fn insert(&mut self, index: impl Into<Index<Levels::Mask, Levels::LevelCount>>, value: Data) {
        let index: usize = index.into().into();
        self.get_or_insert_impl(index, ConstTrue, ||value);
    }
    
    /// insert = true - will write value.
    #[inline]
    fn get_or_insert_impl(&mut self, index: usize, insert: impl ConstBool, value_fn: impl FnOnce() -> Data)
        -> &mut Data 
    {
        let level_indices = crate::level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let last_level_inner_index = unsafe{ *level_indices.as_ref().last().unwrap_unchecked() };
        
        let mut level_block_index = 0;
        const_loop!(LEVEL_INDEX in 0..{<Levels::LevelCount as ConstInteger>::Dec::VALUE} => {
            let level_index = ConstUsize::<LEVEL_INDEX>;
            let inner_index = level_indices.as_ref()[LEVEL_INDEX];
            let next_level_block_index = unsafe {
                let block = unsafe{ self.get_block(level_index, level_block_index) };
                block.get_child(inner_index) 
            };
            level_block_index = if next_level_block_index.is_zero() {
                // 1. Insert new block in next level
                struct Insert;
                impl<M> MutVisitor<M> for Insert {
                    type Out = usize;
                    
                    #[inline(always)]
                    fn visit<I:ConstInteger, L: ILevel>(self, _: I, level: &mut L) -> usize {
                        level.insert_empty_block()
                    }
                }
                let new_level_block_index = self.levels.visit_mut(level_index.inc(), Insert);
                
                // 2. Insert new block index as a child
                unsafe{
                    // Take **THE SAME** and **UNCHANGED** block again, now as mut, 
                    // just because MIRI does not see that we work with different
                    // level/tuple elements.
                    // Let's hope that compiler does eliminate this inefficiency.
                    let block_ptr = self.get_block_mut(level_index, level_block_index);   
                    block_ptr.insert_child(inner_index, new_level_block_index);
                }
                
                new_level_block_index
            } else {
                next_level_block_index
            };
        });
        
        // 3. Last level
        let data_block_index = unsafe {
            let mut block = self.get_block_mut(Levels::LevelCount::default().dec(), level_block_index);
            let mut data_block_index = block.get_child(last_level_inner_index);
            if data_block_index.is_zero() {
                let i = self.values.len();
                
                self.values.push((value_fn)());
                self.keys.push(index);
                self.last_level_block_indices.push(
                    (level_block_index, last_level_inner_index)
                );
                
                block.insert_child(last_level_inner_index, i);
                i                   
            } else {
                /*const*/ if insert.value() { 
                    *self.values.get_unchecked_mut(data_block_index) = (value_fn)();
                }
                data_block_index
            }
        };

        // 4. Data
        unsafe{ self.values.get_unchecked_mut(data_block_index) }  
    }
    
    /// Returns `Some`, if element with `index` exists in container.
    /// `None` - otherwise.
    #[inline]
    pub fn get_mut(&mut self, index: impl Into<Index<Levels::Mask, Levels::LevelCount>>) 
        -> Option<&mut Data> 
    {
        let index: usize = index.into().into();
        
        let level_indices = crate::level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let data_block_index = unsafe{ self.fetch_block_index(level_indices.as_ref()) };
        
        if data_block_index != 0{
            Some(unsafe{ self.values.get_unchecked_mut(data_block_index) })
        } else {
            None
        }
    }
    
    /// # Safety
    /// 
    /// Element at `index` must exist in container.
    #[inline]
    pub unsafe fn get_mut_unchecked(&mut self, index: usize) -> &mut Data {
        self.get_mut(index).unwrap_unchecked()
        
        /*let level_indices = level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let data_block_index = self.fetch_block_index(level_indices);
        debug_assert!(data_block_index != 0);
        self.values.get_unchecked_mut(data_block_index)*/
    }
    
    // TODO: KeyValues type
    /// Key-values in arbitrary order.
    #[inline]
    pub fn key_values(&self) -> (&[usize], &[Data]) {
        // skip first element
        unsafe{
            (
                self.keys.as_slice().get_unchecked(1..), 
                self.values.as_slice().get_unchecked(1..)
            )
        }
    }

    /// Mutable key-values in arbitrary order.
    #[inline]
    pub fn key_values_mut(&mut self) -> (&[usize], &mut [Data]) {
        // skip first element
        unsafe{
            (
                self.keys.as_slice().get_unchecked(1..), 
                self.values.as_mut_slice().get_unchecked_mut(1..)
            )
        }
    }
    
    #[inline]
    unsafe fn drop_impl(&mut self){
        // Manually drop values, skipping first non-existent element, if necessary.
        let skip_first = !R::REQUIRED; 
        let slice = ptr::slice_from_raw_parts_mut(
            self.values.as_mut_ptr().add(skip_first as usize), 
            self.values.len() - skip_first as usize
        ); 
        ptr::drop_in_place(slice);
        self.values.set_len(0);
    }
}

impl<Levels, Data> SparseTree<Levels, Data, ReqDefault>
where
    Levels: SparseTreeLevels,
    Data: Default
{
    /// This is **SIGNIFICANTLY** faster than `get(index).unwrap_or(Default::default())`.
    /// 
    /// Completely branchless implementation. 
    /// Performance-wise equivalent of dereferencing `LevelCount` pointers.
    #[inline]
    pub fn get_or_default(&self, index: impl Into<Index<Levels::Mask, Levels::LevelCount>>) 
        -> &Data
    {
        let index: usize = index.into().into();
        let level_indices = crate::level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let data_block_index = unsafe{ self.fetch_block_index(level_indices.as_ref()) };
        unsafe{ self.values.get_unchecked(data_block_index) }
    }
}

#[cfg(feature = "may_dangle")]
unsafe impl<Levels, #[may_dangle] Data, R> Drop for SparseTree<Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement,
{
    #[inline]
    fn drop(&mut self) {
        unsafe{ self.drop_impl(); }
    }
}

#[cfg(not(feature = "may_dangle"))]
impl<Levels, Data, R> Drop for SparseTree<Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement,
{
    #[inline]
    fn drop(&mut self) {
        unsafe{ self.drop_impl(); }
    }
}

impl<Levels, Data, R> Borrowable for SparseTree<Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement
{
    type Borrowed = SparseTree<Levels, Data, R>; 
}

impl<'this, Levels, Data, R> HibitTreeTypes<'this> for SparseTree<Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement
{
    type Data = &'this Data;
    type DataUnchecked = &'this Data;
    type Cursor = Cursor<'this, Levels, Data, R>;
}

impl<Levels, Data, R> HibitTree for SparseTree<Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement
{
    const EXACT_HIERARCHY: bool = true;
    
    type LevelCount = Levels::LevelCount;
    type LevelMask  = Levels::Mask;
    
    // For terminal_node_mask
    /*#[inline]
    unsafe fn level_mask<I: ConstArray<Item=usize>>(&self, level_indices: I) -> Self::LevelMask<'_> {
        let block_index = self.fetch_block_index(level_indices);
        let block_ptr   = self.get_block_ptr(I::Cap::default(), block_index);
        self.get_block_mask(I::Cap::default(), block_ptr)
    }*/    

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) -> Option<&Data> {
        let data_block_index = self.fetch_block_index(level_indices);
        if data_block_index == 0 {
            None
        } else {
            Some( self.values.get_unchecked(data_block_index) )    
        }
    }

    // This is also data_or_default
    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) -> &Data {
        self.data(index, level_indices).unwrap_unchecked()
        /*let data_block_index = self.fetch_block_index(level_indices);
        self.values.get_unchecked(data_block_index)*/
    }
}

pub struct Cursor<'src, Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement 
{
    /// [*const u8; Levels::LevelCount-1]
    /// 
    /// Level0 skipped - we can get it from self/this.
    level_block_ptrs: ConstArrayType<
        *const u8, 
        <Levels::LevelCount as ConstInteger>::Dec
    >,
    phantom_data: PhantomData<&'src SparseTree<Levels, Data, R>>
}

impl<'this, 'src, Levels, Data, R> HibitTreeCursorTypes<'this> for Cursor<'src, Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement,
{
    type Data = &'src Data;
}

impl<'src, Levels, Data, R> HibitTreeCursor<'src> for Cursor<'src, Levels, Data, R>
where
    Levels: SparseTreeLevels,
    R: DefaultRequirement,
{
    type Src = SparseTree<Levels, Data, R>;

    #[inline]
    fn new(_: &'src Self::Src) -> Self {
        Self{
            level_block_ptrs: Array::from_fn(|_|null()),
            phantom_data: Default::default(),
        }
    }

    #[inline(always)]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as HibitTree>::LevelMask {
        self.select_level_node(src, level_n, level_index)
    }
    
    // Non-existent childs - always point to "empty" node,
    // So we do not need any additional branching here.
    #[inline(always)]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as HibitTree>::LevelMask {
        if N::VALUE == 0 {
            assert_eq!(level_index, 0); // This act as compile-time check
            let block = src.get_block(level_n, 0);
            return block.get_mask().clone();
        }
        
        // We do not store the root level's block.
        let level_block_ptrs_index = level_n.dec().value();
        
        // 1. get level_block_index from prev level. 
        let level_block_index = {
            let prev_level_block: BlockPtr<Levels, N::Dec> = 
                if N::VALUE == 1 {
                    // get from root
                    src.get_block(Default::default(), 0)
                } else {
                    let ptr = *self.level_block_ptrs.as_ref().get_unchecked(level_block_ptrs_index-1); 
                    BlockPtr::new_unchecked(NonNull::new_unchecked(ptr as *mut u8))
                };
            prev_level_block.get_child(level_index)
        };
        
        // 2. get block mask from level.
        let block = src.get_block(level_n, level_block_index);
        *self.level_block_ptrs.as_mut().get_unchecked_mut(level_block_ptrs_index) = block.as_ptr();
        block.get_mask().clone()        
    }

    #[inline(always)]
    unsafe fn data_unchecked<'a>(&'a self, src: &'src Self::Src, level_index: usize)
        -> <Self as HibitTreeCursorTypes<'a>>::Data 
    {
        self.data(src, level_index).unwrap_unchecked()
    }
    
    #[inline(always)]
    unsafe fn data<'a>(&'a self, src: &'src Self::Src, level_index: usize)
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        let level_block: BlockPtr<Levels, <Levels::LevelCount as ConstInteger>::Dec> = 
            if Levels::LevelCount::VALUE == 1{
                // get from root 
                src.get_block(Default::default(), 0)
            } else {
                // We do not store the root level's block.
                let ptr = *self.level_block_ptrs.as_ref().last().unwrap_unchecked();
                    //get_unchecked(Levels::LevelCount::VALUE - 2);
                BlockPtr::new_unchecked(NonNull::new_unchecked(ptr as * mut u8))
            };
        
        let data_block_index = level_block.get_child(level_index);
        if data_block_index == 0 {
            None
        } else {
            Some(src.values.get_unchecked(data_block_index))
        }
    }    
}