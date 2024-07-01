use std::borrow::Borrow;
use crate::{BitBlock, SparseHierarchy};
use crate::const_utils::{ConstArray, ConstInteger};
use crate::iter2::Iter2;
use crate::utils::{Borrowable, Take};

pub trait SparseHierarchy2: Sized + Borrowable<Borrowed=Self> {
    type LevelCount: ConstInteger;
    
    type LevelMaskType: BitBlock;
    type LevelMask<'a>: Borrow<Self::LevelMaskType> + Take<Self::LevelMaskType>
        where Self: 'a;
 
    type DataType;
    type Data<'a>: Borrow<Self::DataType> + Take<Self::DataType>
        where Self: 'a;
 
    // TODO: get() / get_unchecked() instead?
    //       Does level_indices computing actually expensive?
    /// Element may not exists, but `level_indices` must be in range.
    unsafe fn data<I>(&self, level_indices: I) -> Option<Self::Data<'_>>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy;
 
    /// pointed element must exists
    unsafe fn data_unchecked<I>(&self, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy;
    
    type State: SparseHierarchyState2<This = Self>; 
    
    #[inline]
    fn iter(&self) -> Iter2<Self>{
        Iter2::new(self)
    }    
}

pub trait SparseHierarchyState2 {
    type This: SparseHierarchy2;
    
    fn new(this: &Self::This) -> Self;
    
    /// Item at index may not exist. Will return empty mask in such case.
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self,
        this: &'a Self::This,
        level_n: N, 
        level_index: usize,
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a>;
    
    /// Pointed node must exists
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self,
        this: &'a Self::This,
        level_n: N, 
        level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a>;
    
    /// Item at index may not exist.
    unsafe fn data<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> Option<<Self::This as SparseHierarchy2>::Data<'a>>;      
 
    /// Pointed data must exists
    unsafe fn data_unchecked<'a>(
        &self,
        this: &'a Self::This,
        level_index: usize
    ) -> <Self::This as SparseHierarchy2>::Data<'a>;        
}