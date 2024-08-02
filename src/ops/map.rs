use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{LazySparseHierarchy, SparseHierarchy, SparseHierarchyState};
use crate::const_utils::ConstInteger;
use crate::utils::Borrowable;

pub struct Map<S, F>{
    s: S,
    f: F,
}

impl<'a, S, F, Out> SparseHierarchy<'a> for Map<S, F>
where
    S: Borrowable<Borrowed: SparseHierarchy<'a>> + 'a,
    F: Fn(<S::Borrowed as SparseHierarchy<'a>>::Data) -> Out + 'a,
    Out:'a    
{
    const EXACT_HIERARCHY: bool = <S::Borrowed as SparseHierarchy>::EXACT_HIERARCHY;
    
    type LevelCount = <S::Borrowed as SparseHierarchy<'a>>::LevelCount;
    
    type LevelMaskType = <S::Borrowed as SparseHierarchy<'a>>::LevelMaskType;
    type LevelMask = <S::Borrowed as SparseHierarchy<'a>>::LevelMask;
    
    type DataType = Out;
    type Data = Out;

    #[inline]
    unsafe fn data(&'a self, index: usize, level_indices: &[usize]) -> Option<Self::Data> {
        let data = self.s.borrow().data(index, level_indices);
        if let Some(data) = data {
            Some( (self.f)(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked(&'a self, index: usize, level_indices: &[usize]) -> Self::Data {
        let data = self.s.borrow().data_unchecked(index, level_indices);
        (self.f)(data)
    }
    
    type State = State<'a, S, F>;
}

pub struct State<'a, S, F>(
    <S::Borrowed as SparseHierarchy<'a>>::State,
    PhantomData<F>
) 
where 
    S: Borrowable<Borrowed: SparseHierarchy<'a>>;

impl<'a, S, F, Out> SparseHierarchyState<'a> for State<'a, S, F>
where 
    Out: 'a,
    S: Borrowable<Borrowed: SparseHierarchy<'a>> + 'a,
    F: Fn(<S::Borrowed as SparseHierarchy<'a>>::Data) -> Out + 'a
{
    type This = Map<S, F>;
    
    #[inline]
    fn new(this: &'a Self::This) -> Self {
        Self(
            SparseHierarchyState::new(this.s.borrow()),
            PhantomData
        )
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy<'a>>::LevelMask {
        self.0.select_level_node(this.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy<'a>>::LevelMask {
        self.0.select_level_node_unchecked(this.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn data(&self, this: &'a Self::This, level_index: usize) -> Option<Out> {
        let data = self.0.data(this.s.borrow(), level_index);
        if let Some(data) = data {
            Some( (this.f)(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchy<'a>>::Data
    {
        let data = self.0.data_unchecked(this.s.borrow(), level_index);
        (this.f)(data)
    }
}

impl<'a, S, F> LazySparseHierarchy<'a> for Map<S, F>
where
    Map<S, F>: SparseHierarchy<'a>
{}

impl<S, F> Borrowable for Map<S, F> { type Borrowed = Self; }

#[inline]
pub fn map<'a, S, F, Out>(s: S, f: F) -> Map<S, F>
where
    S: Borrowable<Borrowed: SparseHierarchy<'a>>,
    F: Fn(<S::Borrowed as SparseHierarchy<'a>>::Data) -> Out
{
    Map{ s, f }
} 