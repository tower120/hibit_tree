use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{LazySparseHierarchy, SparseHierarchy, SparseHierarchyState};
use crate::const_utils::ConstInteger;
use crate::utils::Borrowable;

pub struct Map<S, F>{
    s: S,
    f: F
}

impl<S, F, Out> SparseHierarchy for Map<S, F>
where
    S: Borrowable<Borrowed: SparseHierarchy>,
    for<'a> F: Fn(<S::Borrowed as SparseHierarchy>::Data<'a>) -> Out
{
    const EXACT_HIERARCHY: bool = <S::Borrowed as SparseHierarchy>::EXACT_HIERARCHY;
    
    type LevelCount = <S::Borrowed as SparseHierarchy>::LevelCount;
    
    type LevelMaskType = <S::Borrowed as SparseHierarchy>::LevelMaskType;
    type LevelMask<'a> = <S::Borrowed as SparseHierarchy>::LevelMask<'a> where Self: 'a;
    
    type DataType = Out;
    type Data<'a> = Out where Self: 'a;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) -> Option<Self::Data<'_>> {
        let data = self.s.borrow().data(index, level_indices);
        if let Some(data) = data {
            Some( (self.f)(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) -> Self::Data<'_> {
        let data = self.s.borrow().data_unchecked(index, level_indices);
        (self.f)(data)
    }
    
    type State = State<S, F>;
}

pub struct State<S, F>(
    <S::Borrowed as SparseHierarchy>::State,
    PhantomData<F>
) 
where 
    S: Borrowable<Borrowed: SparseHierarchy>;

impl<S, F, Out> SparseHierarchyState for State<S, F>
where 
    S: Borrowable<Borrowed: SparseHierarchy>,
    for<'a> F: Fn(<S::Borrowed as SparseHierarchy>::Data<'a>) -> Out
{
    type This = Map<S, F>;
    
    #[inline]
    fn new(this: &Self::This) -> Self {
        Self(
            SparseHierarchyState::new(this.s.borrow()),
            PhantomData
        )
    }

    #[inline]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
        self.0.select_level_node(this.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy>::LevelMask<'a> {
        self.0.select_level_node_unchecked(this.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) -> Option<Out> {
        let data = self.0.data(this.s.borrow(), level_index);
        if let Some(data) = data {
            Some( (this.f)(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy>::Data<'a> 
    {
        let data = self.0.data_unchecked(this.s.borrow(), level_index);
        (this.f)(data)
    }
}

impl<S, F> LazySparseHierarchy for Map<S, F>
where
    Map<S, F>: SparseHierarchy
{}

impl<S, F> Borrowable for Map<S, F> { type Borrowed = Self; }

#[inline]
pub fn map<S, F, Out>(s: S, f: F) -> Map<S, F>
where
    S: Borrowable<Borrowed: SparseHierarchy>,
    for<'a> F: Fn(<S::Borrowed as SparseHierarchy>::Data<'a>) -> Out
{
    Map{ s, f }
} 