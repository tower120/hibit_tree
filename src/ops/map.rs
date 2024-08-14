use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{LazySparseHierarchy, MonoSparseHierarchy, SparseHierarchy, SparseHierarchyState, SparseHierarchyStateTypes, SparseHierarchyTypes};
use crate::const_utils::ConstInteger;
use crate::utils::{Borrowable, UnaryFunction};

mod private {
    pub trait Sealed<I> {} // Users in other crates cannot name this trait.
    
    impl<F, I, O> Sealed<I> for F
    where
        F: super::UnaryFunction<I, Output = O>
    {}
}

pub trait MapFunction<'a, I>
    : UnaryFunction<I, Output = <Self as MapFunction<'a, I>>::Output>
    + private::Sealed<I>
{
	type Output;
}

impl<'a, I, F, O> MapFunction<'a, I> for F
where
	F: UnaryFunction<I, Output = O>
{
	type Output = O;
}

pub struct Map<S, F>{
    s: S,
    f: F,
}

impl<'this, S, F> SparseHierarchyTypes<'this> for Map<S, F>
where
    S: Borrowable<Borrowed: MonoSparseHierarchy>,
    F: for<'a> MapFunction<'a, <S::Borrowed as SparseHierarchyTypes<'a>>::Data> 
{
    type Data = <F as MapFunction<'this, <S::Borrowed as SparseHierarchyTypes<'this>>::Data>>::Output;
    type State = State<'this, S, F>;
}

impl<S, F> SparseHierarchy for Map<S, F>
where
    S: Borrowable<Borrowed: MonoSparseHierarchy>,
    F: for<'a> MapFunction<'a, <S::Borrowed as SparseHierarchyTypes<'a>>::Data>
{
    const EXACT_HIERARCHY: bool = <S::Borrowed as SparseHierarchy>::EXACT_HIERARCHY;
    
    type LevelCount = <S::Borrowed as SparseHierarchy>::LevelCount;
    type LevelMask  = <S::Borrowed as SparseHierarchy>::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as SparseHierarchyTypes<'_>>::Data> 
    {
        let data = self.s.borrow().data(index, level_indices);
        if let Some(data) = data {
            Some( self.f.exec(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> <Self as SparseHierarchyTypes<'_>>::Data 
    {
        let data = self.s.borrow().data_unchecked(index, level_indices);
        self.f.exec(data)
    }
}

impl<'this, 'src, S, F> SparseHierarchyStateTypes<'this> for State<'src, S, F>
where 
    S: Borrowable<Borrowed: MonoSparseHierarchy>,
    F: for<'a> MapFunction<'a, <S::Borrowed as SparseHierarchyTypes<'a>>::Data>
{
    // Map can work only with "monolithic" SparseHierarchy. 
    // So it's the same return type everywhere. 
    type Data = <Map<S, F> as SparseHierarchyTypes<'src>>::Data;
} 

pub struct State<'src, S, F>(
    <S::Borrowed as SparseHierarchyTypes<'src>>::State,
    PhantomData<&'src Map<S, F>>
) 
where 
    S: Borrowable<Borrowed: SparseHierarchy>;

impl<'src, S, F> SparseHierarchyState<'src> for State<'src, S, F>
where 
    S: Borrowable<Borrowed: MonoSparseHierarchy>,
    F: for<'a> MapFunction<'a, <S::Borrowed as SparseHierarchyTypes<'a>>::Data>
{
    type Src = Map<S, F>;
    
    #[inline]
    fn new(this: &'src Self::Src) -> Self {
        Self(
            SparseHierarchyState::new(this.s.borrow()),
            PhantomData
        )
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        self.0.select_level_node(src.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        self.0.select_level_node_unchecked(src.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> Option<<Self as SparseHierarchyStateTypes<'a>>::Data> 
    {
        let data = self.0.data(this.s.borrow(), level_index);
        if let Some(data) = data {
            Some( this.f.exec(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, this: &'src Self::Src, level_index: usize) 
        -> <Self as SparseHierarchyStateTypes<'a>>::Data
    {
        let data = self.0.data_unchecked(this.s.borrow(), level_index);
        this.f.exec(data)
    }
}

impl<S, F> LazySparseHierarchy for Map<S, F>
where
    Map<S, F>: MonoSparseHierarchy
{}

impl<S, F> Borrowable for Map<S, F> { type Borrowed = Self; }

#[inline]
pub fn map<S, F>(s: S, f: F) -> Map<S, F>
where
    S: Borrowable<Borrowed: MonoSparseHierarchy>,
    F: for<'a> MapFunction<'a, <S::Borrowed as SparseHierarchyTypes<'a>>::Data>
{
    Map{ s, f }
} 