use crate::BitBlock;
use crate::const_int::{ConstInt, ConstInteger};
use crate::level::ILevel;
use crate::level_block::HiBlock;

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
    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &L, acc: Self::Acc) -> Self::Acc
    where
        L: ILevel,
        L::Block: HiBlock<Mask=Mask>;
}

pub trait FoldMutVisitor<Mask> {
    type Acc;
    fn visit<I: ConstInteger, L>(&mut self, i: I, level: &mut L, acc: Self::Acc) -> Self::Acc
    where
        L: ILevel,
        L::Block: HiBlock<Mask=Mask>;
}


pub trait SparseArrayLevels: Default {
    type LevelCount: ConstInteger;
    type Mask: BitBlock;
    
    fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, visitor: V) -> V::Out;
    fn visit_mut<I: ConstInteger, V: MutVisitor<Self::Mask>>(&mut self, i: I, visitor: V) -> V::Out;
    
    #[inline]
    fn fold<Acc>(&self, acc: Acc, visitor: impl FoldVisitor<Self::Mask, Acc=Acc>) -> Acc{
        self.fold_n(Self::LevelCount::DEFAULT, acc, visitor)
    }
    fn fold_mut<Acc>(&mut self, acc: Acc, visitor: impl FoldMutVisitor<Self::Mask, Acc=Acc>) -> Acc;
    
    /// fold first `n` tuple elements.
    fn fold_n<Acc>(&self, n: impl ConstInteger, acc: Acc, visitor: impl FoldVisitor<Self::Mask, Acc=Acc>) -> Acc;
}

macro_rules! sparse_array_levels_impl {
    ($n:literal: [$($i:tt,)+]; $first_t:tt, $($t:tt,)* ) => {
        impl<$first_t, $($t,)*> SparseArrayLevels for ($first_t, $($t,)*)
        where
            $first_t: ILevel,
            $first_t::Block: HiBlock,
            $(
                $t: ILevel,
                $t::Block: HiBlock<Mask = <$first_t::Block as HiBlock>::Mask>,
            )*
        {
            type LevelCount = ConstInt<$n>;       
            type Mask = <$first_t::Block as HiBlock>::Mask;
    
            #[inline]
            fn visit<I: ConstInteger, V: Visitor<Self::Mask>>(&self, i: I, mut visitor: V) -> V::Out {
                match i.value() {
                    $(
                        $i => visitor.visit(i, &self.$i),
                    )+
                    _ => unreachable!()
                }
            }
            
            #[inline]
            fn visit_mut<I: ConstInteger, V: MutVisitor<Self::Mask>>(&mut self, i: I, mut visitor: V) -> V::Out {
                match i.value() {
                    $(
                        $i => visitor.visit(i, &mut self.$i),
                    )+
                    _ => unreachable!()
                }
            }

            #[inline]
            fn fold_mut<Acc>(&mut self, mut acc: Acc, mut visitor: impl FoldMutVisitor<Self::Mask, Acc = Acc>) -> Acc {
                $(
                    acc = visitor.visit(ConstInt::<$i>, &mut self.$i, acc);
                )+
                acc
            }
            
            #[inline]
            fn fold_n<Acc>(&self, n: impl ConstInteger, mut acc: Acc, mut visitor: impl FoldVisitor<Self::Mask, Acc=Acc>) -> Acc{
                $(
                    if $i == n.value() {
                        return acc;
                    }
                    acc = visitor.visit(ConstInt::<$i>, &self.$i, acc);
                )+
                acc
            }
            
        }
    };
}
sparse_array_levels_impl!(1: [0,]; L0,);
sparse_array_levels_impl!(2: [0,1,]; L0,L1,);
sparse_array_levels_impl!(3: [0,1,2,]; L0,L1,L2,);
sparse_array_levels_impl!(4: [0,1,2,3,]; L0,L1,L2,L3,);
sparse_array_levels_impl!(5: [0,1,2,3,4,]; L0,L1,L2,L3,L4,);
sparse_array_levels_impl!(6: [0,1,2,3,4,5,]; L0,L1,L2,L3,L4,L5,);
sparse_array_levels_impl!(7: [0,1,2,3,4,5,6,]; L0,L1,L2,L3,L4,L5,L6,);
sparse_array_levels_impl!(8: [0,1,2,3,4,5,6,7,]; L0,L1,L2,L3,L4,L5,L6,L7,);