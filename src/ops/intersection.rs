use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::BitAnd;
use crate::{Apply, apply, BitBlock, Empty, SparseHierarchy};
use crate::const_utils::ConstFalse;
use crate::level_block::Block;
use crate::op::BinaryOp;
use crate::utils::{Borrowable, Take};

pub(crate) struct IntersectionOp<F, L, R, O, M>{
    f: F,
    phantom_data: PhantomData<(L, R, O, M)>
}
impl<F, Left, Right, Out, Mask> BinaryOp for IntersectionOp<F, Left, Right, Out, Mask>
where
    Out: Empty,
    F: Fn(&Left, &Right) -> Out,
    Mask: BitBlock,
{
    const EXACT_HIERARCHY: bool = false;
    type SKIP_EMPTY_HIERARCHIES = ConstFalse;
    type LevelMask = Mask;

    #[inline]
    fn lvl_op(
        &self, 
        left : impl Take<Self::LevelMask>, 
        right: impl Take<Self::LevelMask>
    ) -> Self::LevelMask {
        left.take_or_clone() & right.take_or_clone()
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
pub type Intersection<H1, H2, F, Res> = Apply<
    IntersectionOp<
        F, 
        <<H1 as Borrowable>::Borrowed as SparseHierarchy>::DataType,
        <<H2 as Borrowable>::Borrowed as SparseHierarchy>::DataType,
        Res, 
        <<H1 as Borrowable>::Borrowed as SparseHierarchy>::LevelMaskType
    >, 
    H1, 
    H2
>;

/// Intersection between two [SparseHierarchy]ies.
///
/// Finds an intersection between two [SparseHierarchy]ies, and applies `f`
/// to each pair of intersected items.
/// 
/// [SparseHierarchy]ies can be of different types, but must have the same configuration.
#[inline]
pub fn intersection<H1, H2, F, R>(h1: H1, h2: H2, f: F)
    -> Intersection<H1, H2, F, R>
where
    H1: Borrowable<Borrowed: SparseHierarchy>,
    H2: Borrowable<Borrowed: SparseHierarchy<
        LevelCount    = <H1::Borrowed as SparseHierarchy>::LevelCount,
        LevelMaskType = <H1::Borrowed as SparseHierarchy>::LevelMaskType
    >>,
    F: Fn(&<H1::Borrowed as SparseHierarchy>::DataType, &<H2::Borrowed as SparseHierarchy>::DataType) -> R,
    R: Empty,
{
    apply(IntersectionOp{ f, phantom_data: PhantomData }, h1, h2)
}

#[cfg(test)]
mod test{
    use crate::level::{IntrusiveListLevel, SingleBlockLevel};
    use crate::SparseArray;
    use super::*;
    
    #[test]
    fn test_intersect(){
        type Lvl0Block = Block<u64, [u8;64]>;
        type Lvl1Block = Block<u64, [u16;64]>;
        
        #[derive(Clone)]
        struct DataBlock(u64);
        impl Empty for DataBlock{
            fn empty() -> Self {
                Self(0)
            }
        
            fn is_empty(&self) -> bool {
                todo!()
            }
        }
        
        fn t3<S1, S2>(a1: impl Borrowable<Borrowed=S1>, a2: impl Borrowable<Borrowed=S2>)
            -> impl SparseHierarchy<DataType = DataBlock>
        where
            S1: SparseHierarchy<DataType = DataBlock>,
            S2: SparseHierarchy<
                LevelCount    = S1::LevelCount,
                LevelMaskType = S1::LevelMaskType,
                DataType = DataBlock
            >
        {
            intersection(a1, a2, |a1, a2| DataBlock(a1.0 + a2.0))
        }        
        
        fn t<S1, S2>(a1: S1, a2: S2)
        where
            S1: Borrowable<Borrowed: SparseHierarchy<DataType = DataBlock>>,
            S2: Borrowable<Borrowed: SparseHierarchy<
                LevelCount    = <S1::Borrowed as SparseHierarchy>::LevelCount,
                LevelMaskType = <S1::Borrowed as SparseHierarchy>::LevelMaskType,
                DataType = DataBlock
            >>,
        {
            let res = intersection(a1, a2, |a1: &DataBlock, a2: &DataBlock| -> DataBlock{/* DataBlock(500)*/ todo!() });
            res.iter();
        }
        
        fn t2<'a, S1, S2>(a1: &'a S1, a2: &'a S2)
            //-> impl SparseHierarchy<DataType = DataBlock> + 'a
            -> Intersection<&'a S1, &'a S2, fn (&DataBlock, &DataBlock) -> DataBlock, DataBlock> 
        where
            S1: SparseHierarchy<DataType = DataBlock>,
            S2: SparseHierarchy<
                LevelCount    = S1::LevelCount,
                LevelMaskType = S1::LevelMaskType,
                DataType = DataBlock
            >
        {
            intersection(a1, a2, |a1, a2| DataBlock(a1.0 + a2.0))
        }        
        
        
        type BlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>), DataBlock>;
        let mut a1 = BlockArray::default();
        a1.insert(12, DataBlock(100));
        
        let mut a2 = BlockArray::default();
        a2.insert(12, DataBlock(200));
        
        let res = intersection(&a1, &a2, |a1, a2| DataBlock(a1.0 + a2.0));
        assert_eq!(res.get(12).0, 300);
        
        assert_eq!(t2(&a1, &a2).get(12)/*.take()*/.0, 300);
        
        assert_eq!(t3(&a1, &a2).get(12).borrow().0, 300);
    }
}