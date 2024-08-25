use std::marker::PhantomData;
use crate::{LazyBitmapTree, MultiBitmapTree, MultiBitmapTreeTypes, BitmapTree, BitmapTreeCursor, BitmapTreeCursorTypes, BitmapTreeTypes};
use crate::const_utils::ConstInteger;
use crate::utils::{BinaryFunction, Borrowable, NullaryFunction, UnaryFunction};

pub struct MultiMapFold<S, I, F>{
    s: S,
    init: I,
    f: F
}

impl<'this, S, I, F> BitmapTreeTypes<'this> for MultiMapFold<S, I, F>
where
    S: MultiBitmapTree,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<
        I::Output, 
        <S as MultiBitmapTreeTypes<'a>>::IterItem,
        Output = I::Output
    >,
{
    type Data = I::Output;
    type DataUnchecked = Self::Data;
    type Cursor = Cursor<'this, S, I, F>;
}

impl<S, I, F> BitmapTree for MultiMapFold<S, I, F>
where
    S: MultiBitmapTree,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<         
        I::Output, 
        <S as MultiBitmapTreeTypes<'a>>::IterItem,
        Output = I::Output        
    >,
{
    const EXACT_HIERARCHY: bool = S::EXACT_HIERARCHY;
    type LevelCount = S::LevelCount;
    type LevelMask = S::LevelMask;

    #[inline]
    unsafe fn data(&self, index: usize, level_indices: &[usize]) 
        -> Option<<Self as BitmapTreeTypes<'_>>::Data> 
    {
        if let Some(data_iter) = self.s.data(index, level_indices) {
            let init = self.init.exec();
            let out = data_iter.fold(init, |init, data| self.f.exec(init, data) );
            Some(out)
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: usize, level_indices: &[usize]) 
        -> <Self as BitmapTreeTypes<'_>>::DataUnchecked 
    {
        let data_iter = self.s.data_unchecked(index, level_indices);
        let init = self.init.exec();
        let out = data_iter.fold(init, |init, data| self.f.exec(init, data) );
        out
    }
}

pub struct Cursor<'src, S, I, F> (
    <S as BitmapTreeTypes<'src>>::Cursor,
    PhantomData<&'src MultiMapFold<S, I, F>>
) where
    S: BitmapTree;


impl<'this, 'src, S, I, F> BitmapTreeCursorTypes<'this> for Cursor<'src, S, I, F>
where
    S: MultiBitmapTree,
    I: NullaryFunction,
{ 
    type Data = I::Output; 
}
impl<'src, S, I, F> BitmapTreeCursor<'src> for Cursor<'src, S, I, F>
where
    S: MultiBitmapTree,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<
        I::Output, 
        <S as MultiBitmapTreeTypes<'a>>::IterItem,
        Output = I::Output
    >,
{
    type Src = MultiMapFold<S, I, F>;

    #[inline]
    fn new(this: &'src Self::Src) -> Self {
        Self(
            BitmapTreeCursor::new(&this.s),
            PhantomData
        )
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as BitmapTree>::LevelMask {
        self.0.select_level_node(&src.s, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self, src: &'src Self::Src, level_n: N, level_index: usize
    ) -> <Self::Src as BitmapTree>::LevelMask {
        self.0.select_level_node_unchecked(&src.s, level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, src: &'src Self::Src, level_index: usize) 
        -> Option<<Self as BitmapTreeCursorTypes<'a>>::Data> 
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
        -> <Self as BitmapTreeCursorTypes<'a>>::Data 
    {
        let init = src.init.exec();
        let data_iter = self.0.data_unchecked(&src.s, level_index);
        let out = data_iter.fold(init, |init, data| src.f.exec(init, data) );
        out
    }
}

impl<S, I, F> Borrowable for MultiMapFold<S, I, F>{ type Borrowed = Self; }

impl<S, I, F> LazyBitmapTree for MultiMapFold<S, I, F>
where
    MultiMapFold<S, I, F>: BitmapTree,
    /*S: MultiSparseHierarchy,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<
        I::Output, 
        <S as MultiSparseHierarchyTypes<'a>>::IterItem,
        Output = I::Output
    >,*/
{}

#[inline]
pub fn multi_map_fold<S, I, F>(s: S, init: I, f: F) -> MultiMapFold<S, I, F>
where 
    S: MultiBitmapTree,
    I: NullaryFunction,
    F: for<'a> BinaryFunction<
        I::Output, 
        <S as MultiBitmapTreeTypes<'a>>::IterItem,
        Output = I::Output
    >,
{
    MultiMapFold {s, init, f}   
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
        let intersect = multi_map_fold(intersect, ||0, |a, v| a+v );
        assert_eq!(intersect.get(10), None);
        assert_eq!(intersect.get(15), Some(45));
        assert_eq!(intersect.get(30), None);
        assert_eq!(unsafe{ intersect.get_unchecked(15) }, 45);
        assert_equal(intersect.iter(), [(15, 45)]);
    }
}