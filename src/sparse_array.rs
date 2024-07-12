use std::marker::PhantomData;
use std::ops::ControlFlow;
use std::ops::ControlFlow::{Break, Continue};
use std::ptr::{NonNull, null};
use crate::bit_block::BitBlock;
use crate::utils::Borrowable;
use crate::level_block::HiBlock;
use crate::level::{ILevel, IntrusiveListLevel};
use crate::const_utils::const_int::{ConstUsize, ConstInteger, ConstIntVisitor};
use crate::const_utils::const_array::{ConstArray, ConstArrayType, ConstCopyArrayType};
use crate::const_utils::{ConstBool, ConstFalse, ConstTrue};
use crate::Empty;
use crate::utils::primitive::Primitive;
use crate::utils::array::{Array};
use crate::sparse_array_levels::{FoldMutVisitor, FoldVisitor, MutVisitor, SparseArrayLevels, Visitor};
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
    for level in 0..level_count - 1{
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
    Data: Default,
{
    #[inline]
    fn default() -> Self {        
        Self{
            levels: Levels::default(),
            
            values: vec![Data::default()], 
            keys  : vec![usize::MAX /*doesn't matter*/],
            last_level_block_indices: vec![(0,0)]
        }
    }
}

impl<Levels, Data> SparseArray<Levels, Data>
where
    Levels: SparseArrayLevels,
    Data: Default,
{
    #[inline(always)]
    fn check_index_range(index: usize) {
        assert!(index <= <Self as SparseHierarchy2>::max_range(), "index out of range!");
    }
    
    #[inline(always)]
    unsafe fn get_block_ptr(&self, level_n: impl ConstInteger, level_index: usize) -> *const u8{
        struct V(usize);
        impl<M> Visitor<M> for V{
            type Out = *const u8;
            #[inline(always)]
            fn visit<I: ConstInteger, L>(self, _: I, level: &L) -> Self::Out 
            where 
                L: ILevel 
            {
                unsafe {
                    level.blocks().get_unchecked(self.0) as *const _ as *const u8
                }
            }
        }
        self.levels.visit(level_n, V(level_index))
    }
    
    #[inline(always)]
    unsafe fn get_block_mask(
        &self, 
        level_n: impl ConstInteger, 
        level_block_ptr: *const u8,
    ) -> &Levels::Mask {
        struct V(*const u8);
        impl<M> Visitor<M> for V{
            type Out = NonNull<M>;
            #[inline(always)]
            fn visit<I: ConstInteger, L>(self, _: I, _: &L) -> Self::Out 
            where 
                L: ILevel<Block: HiBlock<Mask=M>> 
            {
                unsafe{
                    let block = self.0 as *const L::Block;
                    NonNull::from((*block).mask())
                }
            }
        }
        self.levels.visit(level_n, V(level_block_ptr)).as_ref()        
    }    

    #[inline(always)]
    unsafe fn get_block_index(
        &self, 
        level_n: impl ConstInteger, 
        level_block_ptr: *const u8, 
        index: usize
    ) -> usize {
        struct V(*const u8, usize);
        impl<M> Visitor<M> for V{
            type Out = usize;
            #[inline(always)]
            fn visit<I: ConstInteger, L>(self, _: I, _: &L) -> Self::Out 
            where 
                L: ILevel<Block: HiBlock> 
            {
                unsafe{
                    let block = self.0 as *const L::Block;
                    (*block).get_or_zero(self.1).as_usize()
                }
            }
        }
        self.levels.visit(level_n, V(level_block_ptr, index))
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
    
    /// Returns `Some(item)` if there is an element at `index` in container.
    /// `None` otherwise. 
    /// 
    /// # Panics
    /// 
    /// Will panic if `index` is outside [max_range()].
    pub fn remove(&mut self, index: usize) -> Option<Data> {
        Self::check_index_range(index);

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
    
    /*/// Returns mutable reference to item at `index`, if exists.
    /// Inserts and return [empty] level_block, otherwise.
    /// 
    /// # Panics
    ///
    /// Will panic if `index` is outside [max_range()].
    ///
    /// # Tip
    /// 
    /// Even though this container is ![EXACT_HIERARCHY], if you end up 
    /// with a value in empty state - consider calling [remove()].
    pub fn get_mut(&mut self, index: usize) -> &mut Data {
        self.get_or_insert(index, ConstFalse, ||Data::empty())
    }*/

    /// Inserts `value` at `index`.
    /// If there was a value - it will be replaced.
    ///
    /// Somewhat faster than *[get_mut()] = `value`, since it will not insert intermediate
    /// [empty] value [^1], if `index` unoccupied.
    ///
    /// [^1]: Thou, if empty constructor is not complex - compiler may be 
    /// able to optimize away intermediate value anyway. But better safe then sorry.
    /// 
    /// # Panics
    ///
    /// Will panic if `index` is outside [max_range()].
    ///
    /// # Tip
    /// 
    /// Even though this container is ![EXACT_HIERARCHY], try not to insert empty 
    /// `value`, as it will appear in iteration. 
    pub fn insert(&mut self, index: usize, value: Data) {
        self.get_or_insert(index, ConstTrue, ||value);
    }
    
    /// insert:
    /// true  - for insert
    /// false - for get_mut
    #[inline]
    fn get_or_insert(&mut self, index: usize, insert: impl ConstBool, value_fn: impl FnOnce() -> Data)
        -> &mut Data 
    {
        Self::check_index_range(index);

        let level_indices = level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let last_level_inner_index = unsafe{ *level_indices.as_ref().last().unwrap_unchecked() }; 
        
        let this = NonNull::new(self).unwrap();
        let last_level_block_index = self.levels.fold_mut(0, V{this, level_indices, index});
        struct V<Levels, Data, LevelIndices> {
            this: NonNull<SparseArray<Levels, Data>>,
            level_indices: LevelIndices,
            index: usize
        }
        impl<Levels, Data, LevelIndices, M> FoldMutVisitor<M> for V<Levels, Data, LevelIndices>
        where
            Levels: SparseArrayLevels,
            Data: Default,
            LevelIndices: Array<Item=usize>
        {
            type Acc = usize;
            
            #[inline(always)]
            fn visit<I: ConstInteger, L: ILevel>(&mut self, i: I, level: &mut L, level_block_index: usize) 
                -> ControlFlow<usize, usize>
            where
                L::Block: HiBlock
            {
            unsafe{
                /*const*/ if I::VALUE == Levels::LevelCount::VALUE - 1 {
                    // Skip last level, will process outside of the loop.
                    return Continue(level_block_index);
                }
                
                let block = level.blocks_mut().get_unchecked_mut(level_block_index);
                let inner_index = self.level_indices.as_ref()[I::VALUE];
                let (block_index, _) = block.get_or_insert(inner_index, ||{
                    struct Insert;
                    impl<M> MutVisitor<M> for Insert {
                        type Out = usize;
                        #[inline(always)]
                        fn visit<I:ConstInteger, L: ILevel>(self, _: I, level: &mut L) -> usize {
                            level.insert_empty_block()
                        }
                    }
                    let block_index = self.this.as_mut().levels.visit_mut(i.inc(), Insert);
                    Primitive::from_usize(block_index)
                });
                Continue(block_index.as_usize())
            }
            }
        }
        
        // 3. Last level
        let data_block_index = self.levels.visit_mut(
            Levels::LevelCount::default().dec(), 
            LastLevelVisitor{
                this,
                level_block_index: last_level_block_index,
                block_inner_index: last_level_inner_index,
                insert, value_fn, index
            }
        );
        struct LastLevelVisitor<Levels, Data, Insert, ValueFn>{
            this: NonNull<SparseArray<Levels, Data>>,
            level_block_index: usize,
            block_inner_index: usize,
            insert: Insert,
            value_fn: ValueFn,
            index: usize
        }
        impl<Levels, Data, Insert, ValueFn, M> MutVisitor<M> for LastLevelVisitor<Levels, Data, Insert, ValueFn>
        where
            Insert: ConstBool,
            ValueFn: FnOnce() -> Data
        {
            type Out = usize;

            #[inline(always)]
            fn visit<I: ConstInteger, L>(mut self, _: I, level: &mut L) -> Self::Out 
            where 
                L: ILevel, L::Block: HiBlock<Mask=M> 
            {
            unsafe{ 
                let block = level.blocks_mut().get_unchecked_mut(self.level_block_index);
                
                let this = self.this.as_mut();
                let (block_index, inserted) = block.get_or_insert(self.block_inner_index, ||{
                    let i = this.values.len();
                    // Make RUST happy, push value latter
                    //this.values.push(value);
                    this.keys.push(self.index);
                    this.last_level_block_indices.push(
                        (self.level_block_index, self.block_inner_index)
                    );
                    Primitive::from_usize(i)                        
                });
                let block_index = block_index.as_usize();
                
                // Compiler should be able to eliminate this branch,
                // and put it's contains into the inner get_or_insert branch.
                // If not - split get_or_insert into try_get + insert_unchecked.  
                if inserted {        
                    this.values.push((self.value_fn)());
                } else {
                    if Insert::VALUE {
                        *this.values.get_unchecked_mut(block_index) = (self.value_fn)();
                    } 
                }
                
                block_index
            }
            }
        }

        // 4. Data
        unsafe{
            self.values.get_unchecked_mut(data_block_index)
        }  
    }
    
/*    /// Returns `Some`, if an element with `index` exists in container.
    /// `None` - otherwise.
    /// 
    /// # Panics
    ///
    /// Will panic if `index` is outside [max_range()].
    #[inline]
    pub fn try_get(&self, index: usize) -> Option<&Data> {
        Self::check_index_range(index);
        let level_indices = level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let data_block_index = unsafe{ self.fetch_block_index(level_indices) };
        
        if data_block_index != 0 {
            Some(unsafe{ self.values.get_unchecked(data_block_index) })
        } else {
            None
        }
    }*/    
    
    /// Returns `Some`, if element with `index` exists in container.
    /// `None` - otherwise.
    /// 
    /// # Panics
    ///
    /// Will panic if `index` is outside [max_range()].
    #[inline]
    pub fn try_get_mut(&mut self, index: usize) -> Option<&mut Data> {
        Self::check_index_range(index);
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
    /// Element at `index` must exist in container [^1].
    /// 
    /// [^1]: Pay attention that this is a stricter requirement than
    /// [SparseHierarchy::get_unchecked]'s.
    #[inline]
    pub unsafe fn get_mut_unchecked(&mut self, index: usize) -> &mut Data {
        let level_indices = level_indices::<Levels::Mask, Levels::LevelCount>(index);
        let data_block_index = self.fetch_block_index(level_indices);
        debug_assert!(data_block_index != 0);
        self.values.get_unchecked_mut(data_block_index)
    }
    
    /* // TODO: mut version
    // TODO: concrete type in return
    /// Return keys and values as contiguous array iterator. 
    #[inline]
    pub fn unordered_iter(&self) -> impl ExactSizeIterator<Item = (usize, &Data)>{
        self.keys[1..].iter().copied().zip(
            self.values[1..].iter()
        )
    } */


    // TODO: index_values?
    // TODO: KeyValues type
    /// Returns (keys_slice, values_slice). Key-values in arbitrary order.
    #[inline]
    pub fn key_values(&self) -> (&[usize], &[Data]) {
        // TODO: use raw
        (&self.keys[1..], &self.values[1..])
    }

    // Key-values in arbitrary order.
    #[inline]
    pub fn key_values_mut(&mut self) -> (&[usize], &mut [Data]) {
        // TODO: use raw
        (&self.keys[1..], &mut self.values[1..])
    }


/*     
    /// Keys in arbitrary order.
    #[inline]
    pub fn keys(&self) -> &[usize] {
        &self.keys[1..]
    }
    
    /// Values in arbitrary order, but same as [keys()].
    #[inline]
    pub fn values(&self) -> &[Data] {
        &self.values[1..]
    }
    
    /// Mutable values in arbitrary order, but same as [keys()].
    #[inline]
    pub fn values_mut(&mut self) -> &mut [Data] {
        &mut self.values[1..]
    } */
}


/*impl<Levels, Data> SparseHierarchy for SparseArray<Levels, Data>
where
    Levels: SparseArrayLevels,
    Data: Empty
{
    const EXACT_HIERARCHY: bool = false;
    
    type LevelCount = Levels::LevelCount;
    type LevelMaskType = Levels::Mask;
    type LevelMask<'a> where Self: 'a = &'a Self::LevelMaskType;

    #[inline]
    unsafe fn level_mask<I: ConstArray<Item=usize>>(&self, level_indices: I) -> Self::LevelMask<'_> {
        let block_index = self.fetch_block_index(level_indices);
        let block_ptr   = self.get_block_ptr(I::Cap::default(), block_index);
        self.get_block_mask(I::Cap::default(), block_ptr)
    }
    
    type DataType = Data;
    type Data<'a> where Self: 'a = &'a Self::DataType;
    
    #[inline]
    unsafe fn data_block<I: ConstArray<Item=usize, Cap=Self::LevelCount>>(&self, level_indices: I) -> Self::Data<'_> {
        let data_block_index = self.fetch_block_index(level_indices);
        self.values.get_unchecked(data_block_index)
    }

    type State = SparseArrayState<Levels, Data>;
}*/

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

/*impl<Levels, Data> SparseHierarchyState for SparseArrayState<Levels, Data>
where
    Levels: SparseArrayLevels,
    Data: Empty,
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
    unsafe fn select_level_bock<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    )
        -> <Self::This as SparseHierarchy>::LevelMask<'a> 
    {
        if N::VALUE == 0{
            assert_eq!(level_index, 0); // This act as compile-time check
            let block_ptr = this.get_block_ptr(level_n, 0);
            return this.get_block_mask(level_n, block_ptr);
        }
        
        // We do not store the root level's block.
        let level_block_ptrs_index = level_n.dec().value();
        
        // 1. get level_block_index from prev level. 
        let level_block_index = {
            let prev_level_block_ptr = 
                if N::VALUE == 1 {
                    // get directly from root
                    this.get_block_ptr(ConstUsize::<0>, 0)
                } else {
                    *self.level_block_ptrs.as_ref().get_unchecked(level_block_ptrs_index-1)
                };
            this.get_block_index(level_n.dec(), prev_level_block_ptr, level_index)
        };
        
        // 2. get block mask from level.
        let block_ptr = this.get_block_ptr(level_n, level_block_index);
        *self.level_block_ptrs.as_mut().get_unchecked_mut(level_block_ptrs_index) = block_ptr;
        this.get_block_mask(level_n, block_ptr)
    }

    #[inline(always)]
    unsafe fn data_block<'a>(&self, this: &'a Self::This, level_index: usize)
        -> <Self::This as SparseHierarchy>::Data<'a> 
    {
        let last_level_index = Levels::LevelCount::default().dec();
        
        let level_block_ptr = 
            if Levels::LevelCount::VALUE == 1{
                this.get_block_ptr(ConstUsize::<0>, 0)
            } else {
                // We do not store the root level's block.
                let level_block_ptrs_index = last_level_index.dec();
                let level_block_ptr = *self.level_block_ptrs.as_ref()
                                      .get_unchecked(level_block_ptrs_index.value());
                level_block_ptr
            };
        
        let data_block_index = this.get_block_index(last_level_index, level_block_ptr, level_index);
        this.values.get_unchecked(data_block_index)
    }
}*/

impl<Levels, Data> Borrowable for SparseArray<Levels, Data>{
    type Borrowed = SparseArray<Levels, Data>; 
}


// Experimental
impl<Levels, Data> SparseHierarchy2 for SparseArray<Levels, Data>
where
    Levels: SparseArrayLevels,
    Data: Default
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
        let data_block_index = self.fetch_block_index(level_indices);
        self.values.get_unchecked(data_block_index)
    }

    type State = SparseArrayState<Levels, Data>;
}

impl<Levels, Data> SparseHierarchyState2 for SparseArrayState<Levels, Data>
where
    Levels: SparseArrayLevels,
    Data: Default,
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
    )
        -> <Self::This as SparseHierarchy2>::LevelMask<'a> 
    {
        // TODO: identical to "select_level_node"
        
        if N::VALUE == 0{
            assert_eq!(level_index, 0); // This act as compile-time check
            let block_ptr = this.get_block_ptr(level_n, 0);
            return this.get_block_mask(level_n, block_ptr);
        }
        
        // We do not store the root level's block.
        let level_block_ptrs_index = level_n.dec().value();
        
        // 1. get level_block_index from prev level. 
        let level_block_index = {
            let prev_level_block_ptr = 
                if N::VALUE == 1 {
                    // get directly from root
                    this.get_block_ptr(ConstUsize::<0>, 0)
                } else {
                    *self.level_block_ptrs.as_ref().get_unchecked(level_block_ptrs_index-1)
                };
            this.get_block_index(level_n.dec(), prev_level_block_ptr, level_index)
        };
        
        // 2. get block mask from level.
        let block_ptr = this.get_block_ptr(level_n, level_block_index);
        *self.level_block_ptrs.as_mut().get_unchecked_mut(level_block_ptrs_index) = block_ptr;
        this.get_block_mask(level_n, block_ptr)
    }
    
    #[inline(always)]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    )
        -> <Self::This as SparseHierarchy2>::LevelMask<'a> 
    {
        if N::VALUE == 0{
            assert_eq!(level_index, 0); // This act as compile-time check
            let block_ptr = this.get_block_ptr(level_n, 0);
            return this.get_block_mask(level_n, block_ptr);
        }
        
        // We do not store the root level's block.
        let level_block_ptrs_index = level_n.dec().value();
        
        // 1. get level_block_index from prev level. 
        let level_block_index = {
            let prev_level_block_ptr = 
                if N::VALUE == 1 {
                    // get directly from root
                    this.get_block_ptr(ConstUsize::<0>, 0)
                } else {
                    *self.level_block_ptrs.as_ref().get_unchecked(level_block_ptrs_index-1)
                };
            this.get_block_index(level_n.dec(), prev_level_block_ptr, level_index)
        };
        
        // 2. get block mask from level.
        let block_ptr = this.get_block_ptr(level_n, level_block_index);
        *self.level_block_ptrs.as_mut().get_unchecked_mut(level_block_ptrs_index) = block_ptr;
        this.get_block_mask(level_n, block_ptr)
    }
    

    #[inline(always)]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize)
        -> <Self::This as SparseHierarchy2>::Data<'a> 
    {
        let last_level_index = Levels::LevelCount::default().dec();
        
        let level_block_ptr = 
            if Levels::LevelCount::VALUE == 1{
                this.get_block_ptr(ConstUsize::<0>, 0)
            } else {
                // We do not store the root level's block.
                let level_block_ptrs_index = last_level_index.dec();
                let level_block_ptr = *self.level_block_ptrs.as_ref()
                                      .get_unchecked(level_block_ptrs_index.value());
                level_block_ptr
            };
        
        let data_block_index = this.get_block_index(last_level_index, level_block_ptr, level_index);
        this.values.get_unchecked(data_block_index)
    }
    
    #[inline(always)]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize)
        -> Option<<Self::This as SparseHierarchy2>::Data<'a>> 
    {
        let last_level_index = Levels::LevelCount::default().dec();
        
        let level_block_ptr = 
            if Levels::LevelCount::VALUE == 1{
                this.get_block_ptr(ConstUsize::<0>, 0)
            } else {
                // We do not store the root level's block.
                let level_block_ptrs_index = last_level_index.dec();
                let level_block_ptr = *self.level_block_ptrs.as_ref()
                                      .get_unchecked(level_block_ptrs_index.value());
                level_block_ptr
            };
        
        let data_block_index = this.get_block_index(last_level_index, level_block_ptr, level_index);
        /*Some(this.values.get_unchecked(data_block_index))*/
        if data_block_index == 0{
            None
        } else {
            Some(this.values.get_unchecked(data_block_index))
        }
    }    
}