use std::borrow::Borrow;
use crate::{HibitTree, HibitTreeCursor, HibitTreeCursorTypes, HibitTreeTypes, HierarchyIndex};
use crate::const_utils::{ConstInteger, IsConstTrue};
use crate::utils::Borrowable;

/// Route all cursor data's through data_or_default.
/// 
/// Zero overhead.
pub struct IterateWithDefault<T> {
    tree: T
}

impl<T> Borrowable for IterateWithDefault<T> { type Borrowed = Self; }

impl<'this, T/*: HibitTree*/> HibitTreeTypes<'this> for IterateWithDefault<T>
where
    T: HibitTree<DefaultData: IsConstTrue>
{
    type Data = <T as HibitTreeTypes<'this>>::Data;
    type DataUnchecked = <T as HibitTreeTypes<'this>>::DataUnchecked;
    type DataOrDefault = <T as HibitTreeTypes<'this>>::DataOrDefault;
    type Cursor = UseDefaultCursor<'this, T>;
}

impl<T/*: HibitTree*/> HibitTree for IterateWithDefault<T>
where
    T: HibitTree<DefaultData: IsConstTrue>
{
    const EXACT_HIERARCHY: bool = T::EXACT_HIERARCHY;
    type DefaultData = T::DefaultData;
    
    type LevelCount = T::LevelCount;
    type LevelMask  = T::LevelMask;

    #[inline]
    fn data(&self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>)
        -> Option<<Self as HibitTreeTypes<'_>>::Data> 
    {
        self.tree.data(index)
    }

    #[inline]
    unsafe fn data_unchecked(
        &self, index: &HierarchyIndex<Self::LevelMask, Self::LevelCount>
    ) -> <Self as HibitTreeTypes<'_>>::DataUnchecked {
        self.tree.data_unchecked(index)
    }
}

pub struct UseDefaultCursor<'tree, T: HibitTree>{
    cursor: <T as HibitTreeTypes<'tree>>::Cursor,
}
impl<'cursor, 'tree, T: HibitTree> HibitTreeCursorTypes<'cursor> for UseDefaultCursor<'tree, T> {
    type Data = Self::DataOrDefault;
    type DataUnchecked = Self::DataOrDefault;
    type DataOrDefault = <<T as HibitTreeTypes<'tree>>::Cursor as HibitTreeCursorTypes<'cursor>>::DataOrDefault;
}
impl<'tree, T/*: HibitTree*/> HibitTreeCursor<'tree> for UseDefaultCursor<'tree, T>
where
    T: HibitTree<DefaultData: IsConstTrue>
{
    type Tree = IterateWithDefault<T>;

    #[inline]
    fn new(tree: &'tree Self::Tree) -> Self {
        Self{
            cursor: HibitTreeCursor::new(&tree.tree)
        }
    }

    #[inline]
    unsafe fn select_level_node<N: ConstInteger>(
        &mut self, tree: &'tree Self::Tree, level_n: N, level_index: usize
    ) -> <Self::Tree as HibitTree>::LevelMask {
        self.cursor.select_level_node(&tree.tree, level_n, level_index)
    }

    #[inline]
    unsafe fn select_level_node_unchecked<N: ConstInteger>(
        &mut self, tree: &'tree Self::Tree, level_n: N, level_index: usize
    ) -> <Self::Tree as HibitTree>::LevelMask {
        self.cursor.select_level_node_unchecked(&tree.tree, level_n, level_index)
    }

    #[inline]
    unsafe fn data<'a>(&'a self, tree: &'tree Self::Tree, level_index: usize) 
        -> Option<<Self as HibitTreeCursorTypes<'a>>::Data> 
    {
        Some(self.data_or_default(tree, level_index))
    }

    #[inline]
    unsafe fn data_unchecked<'a>(
        &'a self, tree: &'tree Self::Tree, level_index: usize
    ) -> <Self as HibitTreeCursorTypes<'a>>::DataUnchecked {
        self.data_or_default(tree, level_index)
    }
    
    #[inline]
    unsafe fn data_or_default<'a>(
        &'a self, tree: &'tree Self::Tree, level_index: usize
    ) -> <Self as HibitTreeCursorTypes<'a>>::DataOrDefault {
        self.cursor.data_or_default(&tree.tree, level_index)
    }
}

/// Description in [LazyHibitTree::iterate_with_default]
#[inline]
pub fn iterate_with_default<T>(tree: T) -> IterateWithDefault<T>
where
    T: HibitTree<DefaultData: IsConstTrue>
{
    IterateWithDefault {tree}
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::{tree2, union, HibitTree, ReqDefault};
    use crate::tree2::Config64bit;
    use crate::hibit_tree::RegularHibitTree;
    use crate::ops::iterate_with_default::iterate_with_default;

    #[test]
    fn smoke_test(){
        type Tree = tree2::Tree<usize, Config64bit<3>, ReqDefault>;
        let mut t1 = Tree::new();
        let mut t2 = Tree::new();
        
        t1.insert(10, 10);
        t2.insert(200, 200);
        
        let union = union(t1, t2).map_w_default(|(l, r): (Option<&usize>, Option<&usize>)| l.unwrap() + r.unwrap());
        let union = iterate_with_default(union);
        
        assert_equal(union.iter(), [(10, 10), (200, 200)]);
    }
}