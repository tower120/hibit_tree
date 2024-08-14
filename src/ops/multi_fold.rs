use std::marker::PhantomData;
use crate::{MultiSparseHierarchy, MultiSparseHierarchyTypes, SparseHierarchy, SparseHierarchyState, SparseHierarchyStateTypes, SparseHierarchyTypes};
use crate::const_utils::ConstInteger;
use crate::utils::{BinaryFunction, Borrowable, NullaryFunction, UnaryFunction};

pub struct MultiFold<S, I, F>{
    s: S,
    init: I,
    f: F
}

impl<'this, S, I, F> SparseHierarchyTypes<'this> for MultiFold<S, I, F>
where
    S: MultiSparseHierarchy,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<
        I::Output, 
        <S as MultiSparseHierarchyTypes<'a>>::IterItem,
        Output = I::Output
    >,
{
    type Data = I::Output;
    type State = State<'this, S, I, F>;
}

impl<S, I, F> SparseHierarchy for MultiFold<S, I, F>
where
    S: MultiSparseHierarchy,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<         
        I::Output, 
        <S as MultiSparseHierarchyTypes<'a>>::IterItem,
        Output = I::Output        
    >,
{
    const EXACT_HIERARCHY: bool = S::EXACT_HIERARCHY;
    type LevelCount = S::LevelCount;
    type LevelMask = S::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as SparseHierarchyTypes<'_>>::Data> 
    {
        if let Some(data_iter) = self.s.data(index, level_indices) {
            let init = self.init.exec();
            let out = data_iter.fold(init, |init, data| self.f.exec(init, data) );
            Some(out)
        } else {
            None
        }
    }

    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> <Self as SparseHierarchyTypes<'_>>::Data 
    {
        todo!()
    }
}

pub struct State<'src, S, I, F> (
    <S as SparseHierarchyTypes<'src>>::State,
    PhantomData<&'src MultiFold<S, I, F>>
) where
    S: SparseHierarchy;


impl<'this, 'src, S, I, F> SparseHierarchyStateTypes<'this> for State<'src, S, I, F>
where
    S: MultiSparseHierarchy,
    I: NullaryFunction,
{ 
    type Data = I::Output; 
}
impl<'src, S, I, F> SparseHierarchyState<'src> for State<'src, S, I, F>
where
    S: MultiSparseHierarchy,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<
        I::Output, 
        <S as MultiSparseHierarchyTypes<'a>>::IterItem,
        Output = I::Output
    >,
{
    type Src = MultiFold<S, I, F>;

    #[inline]
    fn new(this: &'src Self::Src) -> Self {
        Self(
            SparseHierarchyState::new(&this.s), 
            PhantomData
        )
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        self.0.select_level_node(&src.s, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as SparseHierarchy>::LevelMask {
        self.0.select_level_node_unchecked(&src.s, level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, src: &'src Self::Src, level_index: usize) 
        -> Option<<Self as SparseHierarchyStateTypes<'a>>::Data> 
    {
        if let Some(data_iter) = self.0.data(&src.s, level_index){
            let init = src.init.exec();
            let out = data_iter.fold(init, |init, data| src.f.exec(init, data) );
            Some(out)
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, src: &'src Self::Src, level_index: usize) 
        -> <Self as SparseHierarchyStateTypes<'a>>::Data 
    {
        let init = src.init.exec();
        let data_iter = self.0.data_unchecked(&src.s, level_index);
        let out = data_iter.fold(init, |init, data| src.f.exec(init, data) );
        out
    }
}

impl<S, I, F> Borrowable for MultiFold<S, I, F>{ type Borrowed = Self; }

#[inline]
pub fn multi_fold<S, I, F>(s: S, init: I, f: F) -> MultiFold<S, I, F>
where 
    S: MultiSparseHierarchy,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<
        I::Output, 
        <S as MultiSparseHierarchyTypes<'a>>::IterItem,
        Output = I::Output
    >,
{
    MultiFold{s, init, f}   
}

#[cfg(test)]
mod tests{
    use itertools::assert_equal;
    use super::*;
    use crate::{multi_intersection, CompactSparseArray};
    
    #[test]
    fn smoke_test() {
        type Array = CompactSparseArray<usize, 4>; 
        let mut a1: Array = Default::default();
        let mut a2: Array = Default::default();
        let mut a3: Array = Default::default();
        
        a1.insert(10, 10);
        a1.insert(15, 15);
        a1.insert(30, 30);
        
        a2.insert(15, 15);
        a2.insert(20, 20);
        
        a3.insert(15, 15);
        a3.insert(30, 30);
        
        let arrays = vec![a1, a2, a3];
        let intersect = multi_intersection(arrays.iter());
        let intersect = multi_fold(intersect, ||0, |a, v| a+v );
        assert_eq!(intersect.get(10), None);
        assert_eq!(intersect.get(15), Some(45));
        assert_eq!(intersect.get(30), None);
        assert_equal(intersect.iter(), [(15, 45)]);
    }
}