use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ops::ControlFlow;
use crate::BitBlock;
use crate::const_utils::{const_for_rev, ConstIntVisitor};
use crate::const_utils::const_int::{ConstUsize, ConstInteger};
use crate::level::ILevel;
use crate::level_block::HiBlock;

pub trait TypeVisitor<Mask> {
    type Out;
    fn visit<L>(self, _: PhantomData<L>) -> Self::Out
    where
        L: ILevel,
        L::Block: HiBlock<Mask= Mask>;
}

pub trait Visitor<Mask> {
    type Out;
    fn visit<I: ConstInteger, L>(self, i: I, level: &L) -> Self::Out
    where
        L: ILevel,
        L::Block: HiBlock<Mask= Mask>;
}

pub trait MutVisitor<Mask> {
    type Out;
    fn visit<I: ConstInteger, L>(self, i: I, level: &mut L) -> Self::Out
    where
        L: ILevel,
        L::Block: HiBlock<Mask=Mask>;
}

pub trait FoldVisitor<Mask> {
    type Acc;
    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L, acc: Self::Acc) 
        -> ControlFlow<Self::Acc, Self::Acc>
    where
        L: ILevel,
        L::Block: HiBlock<Mask=Mask>;
}

pub trait FoldMutVisitor<Mask> {
    type Acc;
    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &mut L, acc: Self::Acc) 
        -> ControlFlow<Self::Acc, Self::Acc> 
    where
        L: ILevel,
        L::Block: HiBlock<Mask=Mask>;
}


pub trait SparseArrayLevels: Default {
    type LevelCount: ConstInteger;
    type Mask: BitBlock;
    
    fn visit_type<I: ConstInteger, V: TypeVisitor<Self::Mask>>(i: I, visitor: V) -> V::Out;
    
    fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, visitor: V) -> V::Out;
    fn visit_mut<I: ConstInteger, V: MutVisitor<Self::Mask>>(&mut self, i: I, visitor: V) -> V::Out;
    
    // TODO: remove all folds. We can use const_loop! now with visit/visit_type ?

    #[inline]
    fn fold<Acc>(&self, acc: Acc, visitor: impl FoldVisitor<Self::Mask, Acc=Acc>) -> Acc{
        self.fold_n(Self::LevelCount::DEFAULT, acc, visitor)
    }
    
    fn fold_mut<Acc>(&mut self, acc: Acc, visitor: impl FoldMutVisitor<Self::Mask, Acc=Acc>) -> Acc;
    
    fn fold_rev_mut<Acc>(&mut self, acc: Acc, visitor: impl FoldMutVisitor<Self::Mask, Acc=Acc>) -> Acc;
    
    /// fold first `n` tuple elements.
    fn fold_n<Acc>(&self, n: impl ConstInteger, acc: Acc, visitor: impl FoldVisitor<Self::Mask, Acc=Acc>) -> Acc;
}

macro_rules! sparse_array_levels_impl {
    ($n:literal: [$($i:tt,)+] [$($rev_i:tt,)+]; $first_t:tt, $($t:tt,)* ) => {
        impl<$first_t, $($t,)*> SparseArrayLevels for ($first_t, $($t,)*)
        where
            $first_t: ILevel,
            $first_t::Block: HiBlock,
            $(
                $t: ILevel,
                $t::Block: HiBlock<Mask = <$first_t::Block as HiBlock>::Mask>,
            )*
        {
            type LevelCount = ConstUsize<$n>;       
            type Mask = <$first_t::Block as HiBlock>::Mask;
            
            #[inline(always)]
            fn visit_type<I: ConstInteger, V: TypeVisitor<Self::Mask>>(i: I, mut visitor: V) -> V::Out {
                let mut uninit: MaybeUninit<Self> = MaybeUninit::uninit();
                let ptr = uninit.as_mut_ptr();
                
                fn type_of<T>(_: *mut T) -> PhantomData<T> {
                    PhantomData
                }
                
                match i.value() {
                    $(
                        $i => {
                            let p = unsafe { std::ptr::addr_of_mut!((*ptr).$i) };
                            visitor.visit(type_of(p))
                        },
                    )+
                    _ => unreachable!()
                }
            }
    
            #[inline(always)]
            fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, mut visitor: V) -> V::Out {
                match i.value() {
                    $(
                        $i => visitor.visit(i, &self.$i),
                    )+
                    _ => unreachable!()
                }
            }
            
            #[inline(always)]
            fn visit_mut<I: ConstInteger, V: MutVisitor<Self::Mask>>(&mut self, i: I, mut visitor: V) -> V::Out {
                match i.value() {
                    $(
                        $i => visitor.visit(i, &mut self.$i),
                    )+
                    _ => unreachable!()
                }
            }

            #[inline(always)]
            fn fold_mut<Acc>(&mut self, mut acc: Acc, mut visitor: impl FoldMutVisitor<Self::Mask, Acc = Acc>) -> Acc {
                $(
                    match visitor.visit(ConstUsize::<$i>, &mut self.$i, acc) {
                        ControlFlow::Break(v) => return v,
                        ControlFlow::Continue(v) => acc = v,
                    }
                )+
                acc
            }
            
            #[inline(always)]
            fn fold_rev_mut<Acc>(&mut self, mut acc: Acc, mut visitor: impl FoldMutVisitor<Self::Mask, Acc = Acc>) 
                -> Acc
            {
                $(
                    match visitor.visit(ConstUsize::<$rev_i>, &mut self.$rev_i, acc) {
                        ControlFlow::Break(v) => return v,
                        ControlFlow::Continue(v) => acc = v,
                    } 
                )+
                acc
            }
            
            #[inline]
            fn fold_n<Acc>(&self, n: impl ConstInteger, mut acc: Acc, mut visitor: impl FoldVisitor<Self::Mask, Acc=Acc>) 
                -> Acc
            {
                $(
                    /*const*/ if $i == n.value() {
                        return acc;
                    }
                    match visitor.visit(ConstUsize::<$i>, &self.$i, acc) {
                        ControlFlow::Break(v) => return v,
                        ControlFlow::Continue(v) => acc = v,
                    }
                )+
                acc
            }
            
        }
    };
}
sparse_array_levels_impl!(1: [0,] [0,]; L0,);
sparse_array_levels_impl!(2: [0,1,] [1,0,]; L0,L1,);
sparse_array_levels_impl!(3: [0,1,2,] [2,1,0,]; L0,L1,L2,);
sparse_array_levels_impl!(4: [0,1,2,3,] [3,2,1,0,]; L0,L1,L2,L3,);
sparse_array_levels_impl!(5: [0,1,2,3,4,] [4,3,2,1,0,]; L0,L1,L2,L3,L4,);
sparse_array_levels_impl!(6: [0,1,2,3,4,5,] [5,4,3,2,1,0,]; L0,L1,L2,L3,L4,L5,);
sparse_array_levels_impl!(7: [0,1,2,3,4,5,6,] [6,5,4,3,2,1,0,]; L0,L1,L2,L3,L4,L5,L6,);
sparse_array_levels_impl!(8: [0,1,2,3,4,5,6,7,] [7,6,5,4,3,2,1,0,]; L0,L1,L2,L3,L4,L5,L6,L7,);