use crate::const_utils::{ConstArray, ConstInteger};
use crate::sparse_hierarchy::{SparseHierarchy, SparseHierarchyState};
use crate::utils::Borrowable;

/// Wrapper around [SparseHierarchy] that makes it [EXACT_HIERARCHY].  
pub struct ExactHierarchy<T>(T);
impl<T> ExactHierarchy<T>
where 
    T: Borrowable<Borrowed: SparseHierarchy>
{
    /// Construct wrapper, without any checks. 
    /// 
    /// # Safety
    /// 
    /// `hierarchy` must satisfy [EXACT_HIERARCHY].
    #[inline]
    pub unsafe fn new_unchecked(hierarchy: T) -> Self {
        Self(hierarchy)
    }
}

impl<T> SparseHierarchy for ExactHierarchy<T>
where 
    T: Borrowable<Borrowed: SparseHierarchy>
{
    const EXACT_HIERARCHY: bool = true;
    type LevelCount = <T::Borrowed as SparseHierarchy>::LevelCount;
    type LevelMaskType = <T::Borrowed as SparseHierarchy>::LevelMaskType;
    type LevelMask<'a> where Self: 'a = <T::Borrowed as SparseHierarchy>::LevelMask<'a>;

    #[inline]
    unsafe fn level_mask<I>(&self, level_indices: I) -> Self::LevelMask<'_> 
    where 
        I: ConstArray<Item=usize> + Copy 
    {
        self.0.borrow().level_mask(level_indices)
    }

    type DataType = <T::Borrowed as SparseHierarchy>::DataType;
    type Data<'a> where Self: 'a = <T::Borrowed as SparseHierarchy>::Data<'a>;

    #[inline]
    unsafe fn data_block<I>(&self, level_indices: I) -> Self::Data<'_> 
    where 
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy 
    {
        self.0.borrow().data_block(level_indices)
    }

    type State = ExactHierarchyState<T>;
    
    // TODO: forward other functions
}

pub struct ExactHierarchyState<T>(<T::Borrowed as SparseHierarchy>::State)
where 
    T: Borrowable<Borrowed: SparseHierarchy>;

impl<T> SparseHierarchyState for ExactHierarchyState<T>
where
    T: Borrowable<Borrowed: SparseHierarchy>
{
    type This = ExactHierarchy<T>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        Self(SparseHierarchyState::new(this.0.borrow()))
    }

    #[inline]
    unsafe fn select_level_bock<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
        self.select_level_bock(this, level_n, level_index)
    }

    #[inline]
    unsafe fn data_block<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::Data<'a> 
    {
        self.data_block(this, level_index)
    }
}

impl<T> Borrowable for ExactHierarchy<T>{
    type Borrowed = ExactHierarchy<T>; 
}