use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::ops::ControlFlow::{Break, Continue};
use std::ptr;
use std::ptr::{NonNull, null};
use crate::bit_block::BitBlock;
use crate::utils::Borrowable;
use crate::level_block::HiBlock;
use crate::level::{ILevel, IntrusiveListLevel};
use crate::const_utils::const_int::{ConstUsize, ConstInteger, ConstIntVisitor};
use crate::const_utils::const_array::{ConstArray, ConstArrayType, ConstCopyArrayType};
use crate::const_utils::{const_loop, ConstBool, ConstFalse, ConstTrue};
use crate::{Empty, Index};
use crate::utils::primitive::Primitive;
use crate::utils::array::{Array};
use crate::sparse_array_levels::{FoldMutVisitor, FoldVisitor, MutVisitor, SparseArrayLevels, TypeVisitor, Visitor};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};

// TODO: make public
// Compile-time loop inside. Ends up with N (AND + SHR)s.
#[inline]
pub(crate) fn level_indices<LevelMask, LevelsCount>(index: usize)
     -> ConstCopyArrayType<usize, LevelsCount>
where
    LevelMask: BitBlock,
    LevelsCount: ConstInteger,
{
    // TODO: need uninit?
    let mut level_indices = ConstCopyArrayType::<usize, LevelsCount>::from_fn(|_|0);
    
    let mut level_remainder = index;
    let level_count = LevelsCount::VALUE;
    for level in 0..level_count - 1 {
        // LevelMask::SIZE * 2^(level_count - level - 1)
        let level_capacity_exp = LevelMask::SIZE.ilog2() as usize * (level_count - level - 1);
        let level_capacity = 1 << level_capacity_exp;
        
        // level_remainder / level_capacity_exp
        let level_index = level_remainder >> level_capacity_exp;
        
        // level_remainder % level_capacity_exp
        level_remainder = level_remainder & (level_capacity - 1);
        
        level_indices.as_mut()[level] = level_index; 
    }
    
    *level_indices.as_mut().last_mut().unwrap() = level_remainder; 
    
    level_indices
}

#[cfg(test)]
#[test]
fn test_level_indices_new(){
    {
        let indices = level_indices::<u64, ConstUsize<2>>(65);
        assert_eq!(indices, [1, 1]);
    }
    {
        let lvl0 = 262_144; // Total max capacity
        let lvl1 = 4096;
        let lvl2 = 64;
        let indices = level_indices::<u64, ConstUsize<3>>(lvl1*2 + lvl2*3 + 4);
        assert_eq!(indices, [2, 3, 4]);
    }
    {
        let indices = level_indices::<u64, ConstUsize<3>>(32);
        assert_eq!(indices, [0, 0, 32]);
    }
    {
        let indices = level_indices::<u64, ConstUsize<2>>(32);
        assert_eq!(indices, [0, 32]);
    }    
    {
        let indices = level_indices::<u64, ConstUsize<1>>(32);
        assert_eq!(indices, [32]);
    }
}

pub struct SparseArray<Levels, Data> {
    levels: Levels,
    
    // TODO: some kind of multi-vec, to reduce allocation count?
    
    /// First item - is placeholder for a non-existent element.
    values: Vec<Data>,
    keys  : Vec<usize>,
    
    // TODO: can be pair of u32's
    // Used only in remove().
    /// Coordinates in last level of pointer to value with this vec index.  
    last_level_block_indices: Vec<(usize/*block_index*/, usize/*in-block index*/)>, 
}

impl<Levels, Data> Default for
    SparseArray<Levels, Data>
where
    Levels: SparseArrayLevels,
{
    #[inline]
    fn default() -> Self {
        let mut values = Vec::with_capacity(1);
        unsafe{ values.set_len(1); }
        
        Self{
            levels: Levels::default(),
            
            values, 
            keys  : vec![usize::MAX /*doesn't matter*/],
            last_level_block_indices: vec![(0,0)]
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
    Levels: SparseArrayLevels,
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

impl<Levels, Data> SparseArray<Levels, Data>
where
    Levels: SparseArrayLevels,
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
    unsafe fn fetch_block_indices<I: ConstArray<Item=usize>>(&self, level_indices: I)
        -> (I, usize)
    {
        let mut out = I::from_fn(|_|0);
        struct V<'a, LevelIndices>{
            level_indices: LevelIndices,
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
        let last_level_block_index = self.levels.fold_n(I::Cap::default(), 0, V{level_indices, out: &mut out});
        (out, last_level_block_index)
    }
    
    #[inline]
    unsafe fn fetch_block_index<I: ConstArray<Item=usize>>(&self, level_indices: I)
        -> usize 
    {
        self.fetch_block_indices(level_indices).1
    }
    
    /// Returns `Some(item)` if there is an element at `index` in container. `None` otherwise. 
    pub fn remove(&mut self, index: impl Into<Index<Levels::Mask, Levels::LevelCount>>) 
        -> Option<Data> 
    {
        let index: usize = index.into().into();

        let level_indices = level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let (levels_block_indices, data_block_index) = unsafe { 
            self.fetch_block_indices(level_indices) 
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
                };
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
    /// [^1]: Thou, if empty constructor is not complex - compiler may be 
    /// able to optimize away intermediate value anyway. But better safe then sorry.
    /// 
    /// # Tip
    /// 
    /// Even though this container is ![EXACT_HIERARCHY], try not to insert empty 
    /// `value`, as it will appear in iteration. 
    pub fn insert(&mut self, index: impl Into<Index<Levels::Mask, Levels::LevelCount>>, value: Data) {
        let index: usize = index.into().into();
        self.get_or_insert_impl(index, ConstTrue, ||value);
    }
    
    /// insert = true - will write value.
    #[inline]
    fn get_or_insert_impl(&mut self, index: usize, insert: impl ConstBool, value_fn: impl FnOnce() -> Data)
        -> &mut Data 
    {
        let level_indices = level_indices::<Levels::Mask, Levels::LevelCount>(index);
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
                    // take block again, since it could move on "insert_empty_block".
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
        
        let level_indices = level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let data_block_index = unsafe{ self.fetch_block_index(level_indices) };
        
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
}

impl<Levels, Data> Drop for SparseArray<Levels, Data>{
    #[inline]
    fn drop(&mut self) {
        // Manually drop values, skipping first non-existent element.
        unsafe{
            ptr::drop_in_place(
                ptr::slice_from_raw_parts_mut(
                    self.values.as_mut_ptr().add(1), 
                    self.values.len() - 1
                )
            );
           self.values.set_len(0);
        }
    }
}

impl<Levels, Data> Borrowable for SparseArray<Levels, Data>{
    type Borrowed = SparseArray<Levels, Data>; 
}

impl<Levels, Data> SparseHierarchy2 for SparseArray<Levels, Data>
where
    Levels: SparseArrayLevels
{
    const EXACT_HIERARCHY: bool = true;
    
    type LevelCount = Levels::LevelCount;
    type LevelMaskType = Levels::Mask;
    
    type LevelMask<'a> = &'a Levels::Mask where Self: 'a;
    
    type DataType = Data;
    type Data<'a> = &'a Data where Self: 'a;
    
    // For terminal_node_mask
    /*#[inline]
    unsafe fn level_mask<I: ConstArray<Item=usize>>(&self, level_indices: I) -> Self::LevelMask<'_> {
        let block_index = self.fetch_block_index(level_indices);
        let block_ptr   = self.get_block_ptr(I::Cap::default(), block_index);
        self.get_block_mask(I::Cap::default(), block_ptr)
    }*/    

    #[inline]
    unsafe fn data<I>(&self, index: usize, level_indices: I) -> Option<Self::Data<'_>>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        let data_block_index = self.fetch_block_index(level_indices);
        if data_block_index == 0 {
            None
        } else {
            Some( self.values.get_unchecked(data_block_index) )    
        }
    }

    // This is also data_or_default
    #[inline]
    unsafe fn data_unchecked<I>(&self, index: usize, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        self.data(index, level_indices).unwrap_unchecked()
        /*let data_block_index = self.fetch_block_index(level_indices);
        self.values.get_unchecked(data_block_index)*/
    }

    type State = SparseArrayState<Levels, Data>;
}

pub struct SparseArrayState<Levels, Data>
where
    Levels: SparseArrayLevels
{
    /// [*const u8; Levels::LevelCount-1]
    /// 
    /// Level0 skipped - we can get it from self/this.
    level_block_ptrs: ConstArrayType<
        *const u8, 
        <Levels::LevelCount as ConstInteger>::Dec
    >,
    phantom_data: PhantomData<SparseArray<Levels, Data>>
}

impl<Levels, Data> SparseHierarchyState2 for SparseArrayState<Levels, Data>
where
    Levels: SparseArrayLevels
{
    type This = SparseArray<Levels, Data>;

    #[inline]
    fn new(_: &Self::This) -> Self {
        Self{
            level_block_ptrs: Array::from_fn(|_|null()),
            phantom_data: Default::default(),
        }
    }

    #[inline(always)]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        self.select_level_node(this, level_n, level_index)
    }
    
    #[inline(always)]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        if N::VALUE == 0 {
            assert_eq!(level_index, 0); // This act as compile-time check
            let block = this.get_block(level_n, 0);
            return block.get_mask();
        }
        
        // We do not store the root level's block.
        let level_block_ptrs_index = level_n.dec().value();
        
        // 1. get level_block_index from prev level. 
        let level_block_index = {
            let prev_level_block: BlockPtr<Levels, N::Dec> = 
                if N::VALUE == 1 {
                    // get from root
                    this.get_block(Default::default(), 0)
                } else {
                    let ptr = *self.level_block_ptrs.as_ref().get_unchecked(level_block_ptrs_index-1); 
                    BlockPtr::new_unchecked(NonNull::new_unchecked(ptr as *mut u8))
                };
            prev_level_block.get_child(level_index)
        };
        
        // 2. get block mask from level.
        let block = this.get_block(level_n, level_block_index);
        *self.level_block_ptrs.as_mut().get_unchecked_mut(level_block_ptrs_index) = block.as_ptr();
        block.get_mask()        
    }

    #[inline(always)]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize)
        -> <Self::This as SparseHierarchy2>::Data<'a> 
    {
        self.data(this, level_index).unwrap_unchecked()
    }
    
    #[inline(always)]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize)
        -> Option<<Self::This as SparseHierarchy2>::Data<'a>> 
    {
        let last_level_index = Levels::LevelCount::VALUE - 1;
        
        let level_block: BlockPtr<Levels, <Levels::LevelCount as ConstInteger>::Dec> = 
            if last_level_index == 1{
                // get from root 
                this.get_block(Default::default(), 0)
            } else {
                // We do not store the root level's block.
                let ptr = *self.level_block_ptrs.as_ref().get_unchecked(last_level_index - 1);
                BlockPtr::new_unchecked(NonNull::new_unchecked(ptr as * mut u8))
            };
        
        let data_block_index = level_block.get_child(level_index);
        if data_block_index == 0 {
            None
        } else {
            Some(this.values.get_unchecked(data_block_index))
        }
    }    
}