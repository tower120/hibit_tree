use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{LazyHibitTree, RegularHibitTree, HibitTree, HibitTreeCursor, HibitTreeCursorTypes, HibitTreeTypes, HierarchyIndex};
use crate::const_utils::{ConstAnd, ConstBool, ConstFalse, ConstInteger, ConstTrue, IsConstTrue};
use crate::utils::{Borrowable, UnaryFunction};

pub struct Map<S, F, D=ConstFalse>{
    s: S,
    f: F,
    phantom: PhantomData<D>,
}

impl<'this, S, F, D> HibitTreeTypes<'this> for Map<S, F, D>
where
    S: Borrowable<Borrowed: HibitTree>,
    for<'a, 'b> F: 
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::Data> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataUnchecked> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataOrDefault> +
    
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::Data > +
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataUnchecked > + 
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataOrDefault >,    
    D: ConstBool
{
    type Data = <F as UnaryFunction<<S::Borrowed as HibitTreeTypes<'this>>::Data>>::Output;
    type DataUnchecked = <F as UnaryFunction<<S::Borrowed as HibitTreeTypes<'this>>::DataUnchecked>>::Output;
    type DataOrDefault = <F as UnaryFunction<<S::Borrowed as HibitTreeTypes<'this>>::DataOrDefault>>::Output;
    type Cursor = Cursor<'this, S, F, D>;
}

impl<S, F, D> HibitTree for Map<S, F, D>
where
    S: Borrowable<Borrowed: HibitTree>,
    for<'a, 'b> F: 
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::Data> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataUnchecked> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataOrDefault> +
    
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::Data > +
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataUnchecked > + 
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataOrDefault >,        
    D: ConstBool
{
    const EXACT_HIERARCHY: bool = <S::Borrowed as HibitTree>::EXACT_HIERARCHY;
    type DefaultData = D;
    
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

    #[inline]
    unsafe fn data_or_default(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>) 
        -> <Self as HibitTreeTypes<'_>>::DataOrDefault 
    {
        let data = self.s.borrow().data_or_default(index);
        self.f.exec(data)
    }
}

impl<'this, 'tree, S, F, D> HibitTreeCursorTypes<'this> for Cursor<'tree, S, F, D>
where 
    S: Borrowable<Borrowed: HibitTree>,
    for<'a, 'b> F: 
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::Data > +
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataUnchecked > + 
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataOrDefault >,
{
    type Data = <F as UnaryFunction<
        <<S::Borrowed as HibitTreeTypes<'tree>>::Cursor as HibitTreeCursorTypes<'this>>::Data
    >>::Output;
    
    type DataUnchecked = <F as UnaryFunction<
        <<S::Borrowed as HibitTreeTypes<'tree>>::Cursor as HibitTreeCursorTypes<'this>>::DataUnchecked
    >>::Output;
    
    type DataOrDefault = <F as UnaryFunction<
        <<S::Borrowed as HibitTreeTypes<'tree>>::Cursor as HibitTreeCursorTypes<'this>>::DataOrDefault
    >>::Output;
} 

pub struct Cursor<'src, S, F, D>(
    <S::Borrowed as HibitTreeTypes<'src>>::Cursor,
    PhantomData<&'src Map<S, F, D>>
) 
where 
    S: Borrowable<Borrowed: HibitTree>;

impl<'tree, S, F, D> HibitTreeCursor<'tree> for Cursor<'tree, S, F, D>
where 
    S: Borrowable<Borrowed: HibitTree>,
    for<'a, 'b> F: 
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::Data> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataUnchecked> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataOrDefault> +
    
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::Data > +
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataUnchecked > + 
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataOrDefault >,    
    D: ConstBool
{
    type Tree = Map<S, F, D>;
    
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
        -> <Self as HibitTreeCursorTypes<'a>>::DataUnchecked
    {
        let data = self.0.data_unchecked(this.s.borrow(), level_index);
        this.f.exec(data)
    }
    
    #[inline]
    unsafe fn data_or_default<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> <Self as HibitTreeCursorTypes<'a>>::DataOrDefault
    {
        let data = self.0.data_or_default(tree.s.borrow(), level_index);
        tree.f.exec(data)
    }
}

impl<S, F, D> LazyHibitTree for Map<S, F, D>
where
    Map<S, F, D>: RegularHibitTree
{}

impl<S, F, D> Borrowable for Map<S, F, D> { type Borrowed = Self; }

/// Maps each [HibitTree] element with [UnaryFunction].
/// 
/// You can make [RegularHibitTree] from [MultiHibitTree] with `map`.
/// 
/// # Note
/// 
/// Due to current Rust limitations, type inference will not work here
/// for lambdas. You'll have to either specify concrete type, or pass fn, 
/// or implement [UnaryFunction] manually or with [fun!].
///
/// ```
/// # use hibit_tree::{map, DenseTree};
/// # use hibit_tree::utils::UnaryFunction;
/// # use hibit_tree::fun;
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
/// 
/// // You can use fun! macro to generate stateless UnaryFunction as well.
/// let m = map(&a, fun!( 
///     [T: Clone] |d: T| -> T { d.clone() } 
/// ));
/// ```   
#[inline]
pub fn map<S, F>(s: S, f: F) -> Map<S, F, ConstFalse>
where
    S: Borrowable<Borrowed: HibitTree>,
    for<'a, 'b> F: 
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::Data> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataUnchecked> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataOrDefault> +
    
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::Data > +
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataUnchecked > + 
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataOrDefault >,    
{
    Map{ s, f, phantom: PhantomData }
} 

/// Same as [map], but allows access to `data_or_default` methods and [iterate_w_default].
///
/// # Map function 
/// 
/// Your function MUST preserve the default value when it appears as input.
/// You're allowed to change its type, but it must be semantically default value.
/// Like `&0 -> 0`, `(&0, &0) -> 0`, `0 -> None`, etc. 
#[inline]
pub fn map_w_default<S, F>(s: S, f: F) -> Map<S, F, ConstTrue>
where
    S: Borrowable<Borrowed: HibitTree<DefaultData: IsConstTrue>>,
    for<'a, 'b> F: 
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::Data> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataUnchecked> +
        UnaryFunction<<S::Borrowed as HibitTreeTypes<'a>>::DataOrDefault> +
    
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::Data > +
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataUnchecked > + 
        UnaryFunction< <<S::Borrowed as HibitTreeTypes<'a>>::Cursor as HibitTreeCursorTypes<'b>>::DataOrDefault >,
{
    Map{ s, f, phantom: PhantomData }
}


/// Generates stateless generic UnaryFunction. Ala generic closure.
/// 
/// Made for [map].
/// 
/// # Syntax
/// 
/// ```text
/// fun!([generics list] |argument: type| -> type { .. }
/// fun!([generics list] |argument: type| where [where bounds] -> type { .. }
/// ```
/// 
/// # Examples
/// 
/// ```
/// # use hibit_tree::fun;
/// # use hibit_tree::utils::Primitive;
/// fun!(['a, T: Primitive, I: Iterator<Item=&'a T>] |xs: I| -> T {
///     xs.fold(T::ZERO, |acc, v| acc + *v) 
/// });
/// ```
/// 
/// With `where` bounds:
/// 
/// ```
/// # use hibit_tree::fun;
/// # use hibit_tree::utils::Primitive;
/// fun!(['a, T, I] |xs: I| -> T where [T: Primitive, I: Iterator<Item=&'a T>] {
///     xs.fold(T::ZERO, |acc, v| acc + *v) 
/// });
/// ```
#[macro_export]
macro_rules! fun {
    // --- UnaryFunction ---

    ([$($generics:tt)*] |$arg:ident: $arg_type:ty| -> $output:ty $body:block) => {
        fun!([$($generics)*] |$arg: $arg_type| -> $output where [] $body)
    };
    
    ([$($generics:tt)*] |$arg:ident: $arg_type:ty| -> $output:ty where [$($bounds:tt)*] $body:block) => {
        {
            struct F;
            impl<$($generics)*> $crate::utils::UnaryFunction<$arg_type> for F
            where
                $($bounds)*
            {
                type Output = $output;
    
                #[inline]
                fn exec(&self, mut $arg: $arg_type) -> Self::Output {
                    $body
                }
            }
            F
        }
    };
}

#[cfg(test)]
mod tests {
    use itertools::assert_equal;
    use super::*;    
    use crate::{multi_intersection, tree2, ReqDefault};
    use crate::tree2::Config64bit;
    use crate::utils::Primitive;

    fn smoke_test(){
        type Tree = tree2::Tree<usize, Config64bit<3>, ReqDefault>;
        let mut t1 = Tree::new();
        let mut t2 = Tree::new();
        
        t1.insert(10, 10);
        t2.insert(200, 200);
        
        fn bypass(d: &usize) -> &usize { d }
        map(
            &t1, bypass 
        );
        
        struct Bypass;
        impl<'a, T> UnaryFunction<&'a T> for Bypass {
            type Output = &'a T;
            fn exec(&self, arg: &'a T) -> Self::Output {
                arg
            }
        }
        let m = map(&t1, /*Bypass*/|i: &usize| -> usize {*i});        
    }
    
    #[test]
    fn multi_map_test() {
        type Tree = tree2::Tree<usize, Config64bit<3>, ReqDefault>;
        let mut a1: Tree = Default::default();
        let mut a2: Tree = Default::default();
        let mut a3: Tree = Default::default();
        
        a1.insert(10, 10);
        a1.insert(15, 15);
        a1.insert(30, 30);
        
        a2.insert(15, 15);
        a2.insert(20, 20);
        
        a3.insert(15, 15);
        a3.insert(30, 30);
        
        let arrays = vec![a1, a2, a3];
        let intersect = multi_intersection(arrays.iter());
        
        {
            struct Sum;
            impl<'a, I> UnaryFunction<I> for Sum
            where
                I: Iterator<Item=&'a usize>
            {
                type Output = usize;
                fn exec(&self, arg: I) -> Self::Output {
                    arg.sum()
                }
            }
            let intersect = map(&intersect, Sum);
        }
        
        // test most generic case with fun!
        let intersect = map(&intersect, fun!(
            ['a, T: Primitive, I: Iterator<Item=&'a T>] |xs: I| -> T {
                xs.fold(T::ZERO, |acc, v| acc + *v) 
            }
        ));
        assert_eq!(intersect.get(10), None);
        assert_eq!(intersect.get(15), Some(45));
        assert_eq!(intersect.get(30), None);
        assert_eq!(unsafe{ intersect.get_unchecked(15) }, 45);
        assert_equal(intersect.iter(), [(15, 45)]);
    }    
}