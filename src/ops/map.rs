use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{LazySparseHierarchy, SparseHierarchy, SparseHierarchyState, SparseHierarchyTypes};
use crate::const_utils::ConstInteger;
use crate::utils::Borrowable;

type MapperOutput<'a, F, I> = <F as Mapper<'a, I>>::Output; 

pub trait Mapper<'a, I>: Fn(I) -> <Self as Mapper<'a, I>>::Output {
	type Output;
}

impl<'a, I, F, O> Mapper<'a, I> for F
where
	F: Fn(I) -> O,
{
	type Output = O;
}



pub struct Map<S, F>{
    s: S,
    f: F,
}

impl<'this, S, F> SparseHierarchyTypes<'this> for Map<S, F>
where
    S: Borrowable<Borrowed: SparseHierarchy>,
    F: for<'a> Mapper<'a, <S::Borrowed as SparseHierarchyTypes<'a>>::Data> 
        //Fn(<S::Borrowed as SparseHierarchyTypes<'a>>::Data) -> Out,
{
    type LevelMaskType = <S::Borrowed as SparseHierarchyTypes<'this>>::LevelMaskType;
    type LevelMask = <S::Borrowed as SparseHierarchyTypes<'this>>::LevelMask;
    
    type DataType = //MapperOutput<'this, F, <S::Borrowed as SparseHierarchyTypes<'this>>::Data>;
                    <F as Mapper<'this, <S::Borrowed as SparseHierarchyTypes<'this>>::Data>>::Output;
    type Data = Self::DataType;
}

impl<S, F> SparseHierarchy for Map<S, F>
where
    S: Borrowable<Borrowed: SparseHierarchy>,
    F: for<'a> Mapper<'a, <S::Borrowed as SparseHierarchyTypes<'a>>::Data>
        //for<'a> Fn(<S::Borrowed as SparseHierarchyTypes<'a>>::Data) -> Out,
{
    const EXACT_HIERARCHY: bool = <S::Borrowed as SparseHierarchy>::EXACT_HIERARCHY;
    
    type LevelCount = <S::Borrowed as SparseHierarchy>::LevelCount;
    
    /*type LevelMaskType = <S::Borrowed as SparseHierarchy<'a>>::LevelMaskType;
    type LevelMask = <S::Borrowed as SparseHierarchy<'a>>::LevelMask;
    
    type DataType = Out;
    type Data = Out;*/

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as SparseHierarchyTypes<'_>>::Data> 
    {
        let data = self.s.borrow().data(index, level_indices);
        if let Some(data) = data {
            Some( (self.f)(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> <Self as SparseHierarchyTypes<'_>>::Data 
    {
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

impl<S, F> SparseHierarchyState for State<S, F>
where 
    S: Borrowable<Borrowed: SparseHierarchy>,
    F: for<'a> Mapper<'a, <S::Borrowed as SparseHierarchyTypes<'a>>::Data>
        //for<'a> Fn(<S::Borrowed as SparseHierarchyTypes<'a>>::Data) -> Out
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
    ) -> <Self::This as SparseHierarchyTypes<'a>>::LevelMask {
        self.0.select_level_node(this.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchyTypes<'a>>::LevelMask {
        self.0.select_level_node_unchecked(this.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> Option<<Self::This as SparseHierarchyTypes<'a>>::Data> 
    {
        let data = self.0.data(this.s.borrow(), level_index);
        if let Some(data) = data {
            Some( (this.f)(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> <Self::This as SparseHierarchyTypes<'a>>::Data
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
pub fn map<S, F>(s: S, f: F) -> Map<S, F>
where
    S: Borrowable<Borrowed: SparseHierarchy>,
    F: for<'any> Mapper<'any, <S::Borrowed as SparseHierarchyTypes<'any>>::Data>
       //for<'a> Fn(<S::Borrowed as SparseHierarchyTypes<'a>>::Data) -> Out
{
    Map{ s, f }
} 