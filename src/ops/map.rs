use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{LazyHibitTree, RegularHibitTree, HibitTree, HibitTreeCursor, HibitTreeCursorTypes, HibitTreeTypes, HierarchyIndex};
use crate::const_utils::{ConstFalse, ConstInteger};
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

impl<'this, S, F> HibitTreeTypes<'this> for Map<S, F>
where
    S: Borrowable<Borrowed: RegularHibitTree>,
    F: for<'a> MapFunction<'a, <S::Borrowed as HibitTreeTypes<'a>>::Data> 
{
    type Data = <F as MapFunction<'this, <S::Borrowed as HibitTreeTypes<'this>>::Data>>::Output;
    type DataUnchecked = Self::Data;
    type Cursor = Cursor<'this, S, F>;
}

impl<S, F> HibitTree for Map<S, F>
where
    S: Borrowable<Borrowed: RegularHibitTree>,
    F: for<'a> MapFunction<'a, <S::Borrowed as HibitTreeTypes<'a>>::Data>
{
    const EXACT_HIERARCHY: bool = <S::Borrowed as HibitTree>::EXACT_HIERARCHY;
    type DefaultData = ConstFalse;

    type LevelCount = <S::Borrowed as HibitTree>::LevelCount;
    type LevelMask  = <S::Borrowed as HibitTree>::LevelMask;

    #[inline]
    fn data(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
    {
        let data = self.s.borrow().data(index);
        if let Some(data) = data {
            Some( self.f.exec(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> <Self as HibitTreeTypes<'_>>::DataUnchecked 
    {
        let data = self.s.borrow().data_unchecked(index);
        self.f.exec(data)
    }
}

impl<'this, 'src, S, F> HibitTreeCursorTypes<'this> for Cursor<'src, S, F>
where 
    S: Borrowable<Borrowed: RegularHibitTree>,
    F: for<'a> MapFunction<'a, <S::Borrowed as HibitTreeTypes<'a>>::Data>
{
    // Map can work only with "monolithic" SparseHierarchy. 
    // So it's the same return type everywhere. 
    type Data = <Map<S, F> as HibitTreeTypes<'src>>::Data;
} 

pub struct Cursor<'src, S, F>(
    <S::Borrowed as HibitTreeTypes<'src>>::Cursor,
    PhantomData<&'src Map<S, F>>
) 
where 
    S: Borrowable<Borrowed: HibitTree>;

impl<'tree, S, F> HibitTreeCursor<'tree> for Cursor<'tree, S, F>
where 
    S: Borrowable<Borrowed: RegularHibitTree>,
    F: for<'a> MapFunction<'a, <S::Borrowed as HibitTreeTypes<'a>>::Data>
{
    type Tree = Map<S, F>;
    
    #[inline]
    fn new(this: &'tree Self::Tree) -> Self {
        Self(
            HibitTreeCursor::new(this.s.borrow()),
            PhantomData
        )
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, src: &'tree Self::Tree, level_n: N, level_index: usize
    ) -> <Self::Tree as HibitTree>::LevelMask {
        self.0.select_level_node(src.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self, src: &'tree Self::Tree, level_n: N, level_index: usize
    ) -> <Self::Tree as HibitTree>::LevelMask {
        self.0.select_level_node_unchecked(src.s.borrow(), level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, this: &'tree Self::Tree, level_index: usize) 
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        let data = self.0.data(this.s.borrow(), level_index);
        if let Some(data) = data {
            Some( this.f.exec(data) )
        } else {
            None
        }
    }

    #[inline]
    unsafe fn data_unchecked<'a>(&'a self, this: &'tree Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::Data
    {
        let data = self.0.data_unchecked(this.s.borrow(), level_index);
        this.f.exec(data)
    }
    
    #[inline]
    unsafe fn data_or_default<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::Data
    {
        let data = self.0.data_or_default(tree.s.borrow(), level_index);
        tree.f.exec(data)
    }
}

impl<S, F> LazyHibitTree for Map<S, F>
where
    Map<S, F>: RegularHibitTree
{}

impl<S, F> Borrowable for Map<S, F> { type Borrowed = Self; }

/// Maps each [RegularHibitTree] element with `f: Fn(Item) -> Out`.
/// 
/// # Note
/// 
/// Due to current Rust limitations, type inference will not work here
/// for lambdas. You'll have to either specify concrete type, or pass fn, 
/// or you can implement [UnaryFunction] with generics.
///
/// ```
/// # use hibit_tree::{map, DenseTree};
/// # use hibit_tree::utils::UnaryFunction;
/// let a: DenseTree<usize, 4> = Default::default();
///
/// // This will fail to compile:
/// // let m = map(&a, |d| { d.clone() } );
///
/// // Lambdas with concrete input type will work.    
/// let m = map(&a, |d: &usize| { d.clone() } );
///
/// // Function pointers with concrete type argument will work too.
/// fn bypass(d: &usize) -> &usize { d }
/// let m = map(&a, bypass );
///
/// // As well as generic UnaryFunction implementation.
/// struct Cloned;
/// impl<T: Clone> UnaryFunction<T> for Cloned{
///     type Output = T;
///
///     fn exec(&self, arg: T) -> Self::Output {
///         arg.clone()
///     }
/// }
/// let m = map(&a, Cloned );
/// ```   
#[inline]
pub fn map<S, F>(s: S, f: F) -> Map<S, F>
where
    S: Borrowable<Borrowed: RegularHibitTree>,
    F: for<'a> MapFunction<'a, <S::Borrowed as HibitTreeTypes<'a>>::Data>
{
    Map{ s, f }
} 