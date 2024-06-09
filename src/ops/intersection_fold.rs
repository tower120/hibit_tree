use std::any::Any;
use std::borrow::Borrow;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitAndAssign, BitOrAssign};
use crate::{BinaryOp, BitBlock, fold, Fold, MaybeEmpty, SparseHierarchy};
use crate::const_utils::ConstFalse;
use crate::level_block::Block;
use crate::utils::{Borrowable, Take};

pub(crate) struct IntersectionFoldOp<F, Acc, Data, Mask>{
    f: F,
    phantom_data: PhantomData<(Acc, Data, Mask)>
}
impl<F, Acc, Data, Mask> BinaryOp for IntersectionFoldOp<F, Acc, Data, Mask>
where
    Acc: MaybeEmpty,
    F: Fn(Acc, &Data) -> Acc,
    Mask: BitBlock,
{
    const EXACT_HIERARCHY: bool = false;
    type SKIP_EMPTY_HIERARCHIES = ConstFalse;
    type LevelMask = Mask;

    fn lvl_op(
        &self, 
        left : impl Take<Self::LevelMask> + Borrow<Self::LevelMask>, 
        right: impl Borrow<Self::LevelMask>
    ) -> Self::LevelMask {
        let mut acc = left.take();
        acc &= right.borrow();
        acc
    }

    type Left  = Acc;
    type Right = Data;
    type Out   = Acc;

    fn data_op(
        &self,
        acc  : impl Take<Self::Left>,
        right: impl Borrow<Self::Right>
    ) -> Self::Out {
        (self.f)(acc.take(), right.borrow())
    }
}

pub type IntersectionFold<Init, Iter, F> = Fold<
    IntersectionFoldOp<
        F, 
        <<Init as Borrowable>::Borrowed as SparseHierarchy>::DataType,
        <<<Iter as Iterator>::Item as Borrowable>::Borrowed as SparseHierarchy>::DataType,
        <<Init as Borrowable>::Borrowed as SparseHierarchy>::LevelMaskType
    >,
    Init,
    Iter
>;

#[inline]
pub fn intersection_fold<Init, Iter, F>(init: Init, iter: Iter, f: F)
    -> IntersectionFold<Init, Iter, F>
where
    Init: Borrowable<Borrowed: SparseHierarchy>,
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy>>,

    F: Fn(
        <Init::Borrowed as SparseHierarchy>::DataType, 
        &<<Iter::Item as Borrowable>::Borrowed as SparseHierarchy>::DataType
    ) -> <Init::Borrowed as SparseHierarchy>::DataType
{
    fold(IntersectionFoldOp { f, phantom_data: PhantomData }, init, iter)
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
        impl MaybeEmpty for DataBlock{
            fn empty() -> Self {
                Self(0)
            }
        
            fn is_empty(&self) -> bool {
                todo!()
            }
        }
        
        fn t3<Init, Item>(
            init: impl Borrowable<Borrowed=Init>,
            iter: impl Iterator<Item:Borrowable<Borrowed=Item>> + Clone
        )
        where
            Init: SparseHierarchy<DataType = DataBlock>,
            Item: SparseHierarchy<
                LevelCount    = Init::LevelCount,
                LevelMaskType = Init::LevelMaskType,
            >,
        {
            let res = intersection_fold(init, iter, |a1, a2| -> DataBlock{/* DataBlock(500)*/ todo!() });
            res.iter();
        }          
        
        type BlockArray = SparseArray<(SingleBlockLevel<Lvl0Block>, IntrusiveListLevel<Lvl1Block>), DataBlock>;
        let mut a1 = BlockArray::default();
        a1.insert(12, DataBlock(100));
        
        let mut a2 = BlockArray::default();
        a2.insert(12, DataBlock(200));
        
        let mut a3 = BlockArray::default();
        a3.insert(12, DataBlock(1000));
        
        let array = [a2, a3];
        
        let res = intersection_fold(&a1, array.iter(),
            |mut acc, d| { 
                acc.0 += d.0; 
                acc 
            }
        );
        assert_eq!(res.get(12).0, 1300);
    }
}