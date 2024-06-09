use std::borrow::Borrow;
use std::marker::PhantomData;
use crate::{Apply, apply, BitBlock, Empty, SparseHierarchy};
use crate::const_utils::ConstTrue;
use crate::op::BinaryOp;
use crate::utils::{Borrowable, Take};

pub(crate) struct UnionOp<F, L, R, O, M>{
    f: F,
    phantom_data: PhantomData<(L, R, O, M)>
}
impl<F, Left, Right, Out, Mask> BinaryOp for UnionOp<F, Left, Right, Out, Mask>
where
    Out: Empty,
    F: Fn(&Left, &Right) -> Out,
    Mask: BitBlock,
{
    const EXACT_HIERARCHY: bool = true;
    type SKIP_EMPTY_HIERARCHIES = ConstTrue;
    type LevelMask = Mask;

    #[inline]
    fn lvl_op(
        &self, 
        left : impl Take<Self::LevelMask>, 
        right: impl Take<Self::LevelMask>
    ) -> Self::LevelMask {
        left.take_or_clone() | right.take_or_clone()
    }

    type Left  = Left;
    type Right = Right;
    type Out   = Out;

    #[inline]
    fn data_op(
        &self,
        left : impl Borrow<Self::Left>,
        right: impl Borrow<Self::Right>
    ) -> Self::Out {
        (self.f)(left.borrow(), right.borrow())
    }
}

// `Res` should be deducible from `F`, but RUST still
// not dealt with Fn's.
pub type Union<H1, H2, F, Res> = Apply<
    UnionOp<
        F, 
        <<H1 as Borrowable>::Borrowed as SparseHierarchy>::DataType,
        <<H2 as Borrowable>::Borrowed as SparseHierarchy>::DataType,
        Res, 
        <<H1 as Borrowable>::Borrowed as SparseHierarchy>::LevelMaskType
    >, 
    H1, 
    H2
>;

/// Union between two [SparseHierarchy]ies.
/// 
/// Finds a union between two [SparseHierarchy]ies, and applies `f`
/// to each pair of merged items. One item-argument may be in empty state if
/// one [SparseHierarchy] has an item at a certain index, and the other doesn't.
/// 
/// [SparseHierarchy]ies can be of different types, but must have the same configuration.
#[inline]
pub fn union<H1, H2, F, R>(h1: H1, h2: H2, f: F)
   -> Union<H1, H2, F, R>
where
    H1: Borrowable<Borrowed: SparseHierarchy>,
    H2: Borrowable<Borrowed: SparseHierarchy<
        LevelCount    = <H1::Borrowed as SparseHierarchy>::LevelCount,
        LevelMaskType = <H1::Borrowed as SparseHierarchy>::LevelMaskType
    >>,
    F: Fn(&<H1::Borrowed as SparseHierarchy>::DataType, &<H2::Borrowed as SparseHierarchy>::DataType) -> R,
    R: Empty,
{
    apply(UnionOp { f, phantom_data: PhantomData }, h1, h2)
}