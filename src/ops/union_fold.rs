use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{BitBlock, Empty, fold, Fold, SparseHierarchy};
use crate::const_utils::ConstTrue;
use crate::op::BinaryOp;
use crate::utils::{Borrowable, Take};

pub(crate) struct UnionFoldOp<F, Acc, Data, Mask>{
    f: F,
    phantom_data: PhantomData<(Acc, Data, Mask)>
}
impl<F, Acc, Data, Mask> BinaryOp for UnionFoldOp<F, Acc, Data, Mask>
where
    Acc: Empty,
    F: Fn(Acc, &Data) -> Acc,
    Mask: BitBlock,
{
    const EXACT_HIERARCHY: bool = true;
    type SKIP_EMPTY_HIERARCHIES = ConstTrue;
    type LevelMask = Mask;

    #[inline]
    fn lvl_op(
        &self, 
        left : impl Take<Self::LevelMask>, 
        right: impl Borrow<Self::LevelMask>
    ) -> Self::LevelMask {
        let mut acc = left.take();
        acc |= right.borrow();
        acc
    }

    type Left  = Acc;
    type Right = Data;
    type Out   = Acc;

    #[inline]
    fn data_op(
        &self,
        acc  : impl Take<Self::Left>,
        right: impl Borrow<Self::Right>
    ) -> Self::Out {
        (self.f)(acc.take(), right.borrow())
    }
}

pub type UnionFold<Init, Iter, F> = Fold<
    UnionFoldOp<
        F, 
        <<Init as Borrowable>::Borrowed as SparseHierarchy>::DataType,
        <<<Iter as Iterator>::Item as Borrowable>::Borrowed as SparseHierarchy>::DataType,
        <<Init as Borrowable>::Borrowed as SparseHierarchy>::LevelMaskType
    >,
    Init,
    Iter
>;

/// Union between N [SparseHierarchy]ies in fold-style.
/// 
/// `Init`'s type may differ, but all [SparseHierarchy]ies 
/// must have the same configuration.
#[inline]
pub fn union_fold<Init, Iter, F>(init: Init, iter: Iter, f: F)
     -> UnionFold<Init, Iter, F>
where
    Init: Borrowable<Borrowed: SparseHierarchy>,
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>,

    F: Fn(
        <Init::Borrowed as SparseHierarchy>::DataType, 
        &<<Iter::Item as Borrowable>::Borrowed as SparseHierarchy>::DataType
    ) -> <Init::Borrowed as SparseHierarchy>::DataType
{
    fold(UnionFoldOp { f, phantom_data: PhantomData }, init, iter)
}