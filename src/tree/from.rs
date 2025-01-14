use std::mem::ManuallyDrop;
use std::ops::ControlFlow::Continue;
use arrayvec::ArrayVec;
use crate::{FromHibitTree, HibitTreeData, RegularHibitTree};
use crate::utils::UnaryFunction;
use super::*;

#[inline]
unsafe fn make_terminal_block_filtered<'a, Other, T, Conf, F, R>(
    other: &'a Other,
    other_cursor: &mut <Other as HibitTreeTypes<'a>>::Cursor,
    mask: Conf::Mask,
    f: &F,
    req: R
) -> Option<BlockPtr<T, Conf>>
where
    Other: RegularHibitTree<LevelMask = Conf::Mask>,
    Conf: Config,
    for<'any> F: UnaryFunction<&'any HibitTreeData<'a, Other>, Output=bool>,
    R: DefaultRequirement,
    MakeDefaultFor<T, R>: MakeDefault<T> 
{
    let mut node_mask: Conf::Mask = BitBlock::zero();

    const{ assert!(<Other::LevelMask as BitBlock>::Size::VALUE <= 128); }
    let mut childs: ArrayVec<_, 128> = Default::default();        
    
    // Gather children, since they may be filtered out, 
    // and we need to know exact node size first. 
    //
    // As an alternative: We could make a node with `mask.count_ones()` size, 
    // and trim it down if needed - but that will cost additional allocation.  
    mask.traverse_bits(|index| {
        let data = other_cursor.data_unchecked(other, index);
        if !f.exec(&data){
            return Continue(());
        }
        
        node_mask.set_bit_unchecked::<true>(index);
        childs.push_unchecked(data);
        
        Continue(())
    });    
    
    let len = node_mask.count_ones() as u8;
    if len == 0 {
        return None;
    }
    
    let mut block = make_empty_terminal(len, req);
    let index_offset = R::Required::VALUE as u8;
    
    // 0. set mask
    *block.mask_mut() = node_mask.clone();
    
    // 1. memcpy childs to node
    {
        let childs = ManuallyDrop::new(childs);
        ptr::copy_nonoverlapping(
            childs.as_ptr(),
            block.children_ptr::<Other::Data>().add(index_offset as usize),
            childs.len()
        );
    }
    
    // 2. fill child indices
    {
        let mut i = index_offset;
        node_mask.traverse_bits(|index| {
            *block.child_indices_mut().get_unchecked_mut(index) = i;
            i += 1;
            Continue(())
        });
    }
    
    // 3. set len
    block.set_len(len + index_offset);
    
    Some(block)    
}

#[inline]
unsafe fn make_terminal_block<'a, Other, T, Conf, R>(
    other: &'a Other,
    other_cursor: &mut <Other as HibitTreeTypes<'a>>::Cursor,
    mask: Conf::Mask,
    req: R
) -> BlockPtr<T, Conf>
where
    Other: RegularHibitTree<LevelMask = Conf::Mask>,
    Conf: Config,
    R: DefaultRequirement,
    MakeDefaultFor<T, R>: MakeDefault<T> 
{
    let len = mask.count_ones() as u8;
    
    let mut block = make_empty_terminal(len, req);
    let index_offset = R::Required::VALUE as u8;

    *block.mask_mut() = mask.clone();
    
    let mut i = index_offset as usize;
    mask.traverse_bits(|index| {
        let data = other_cursor.data_unchecked(other, index);
        block.write_child_at(i, data);
        *block.child_indices_mut().get_unchecked_mut(index) = i as u8;
        i += 1;
        Continue(())
    });
    
    block.set_len(len + index_offset);
    block    
}

#[inline(always)]
unsafe fn from_tree_impl<'a, T, Conf, Other, N, F, R>(
    other: &'a Other,
    other_cursor: &mut <Other as HibitTreeTypes<'a>>::Cursor,
    n: N,
    index: usize,
    empty_branch_blocks: &EmptyBranchBlocks<T, Conf>,
    f: Option<&F>,
    required: R
) -> Option<BlockPtr<T, Conf>>
where
    Conf: Config,
    Other: RegularHibitTree<LevelMask=Conf::Mask>,
    N: ConstInteger,
    for<'any> F: UnaryFunction<&'any HibitTreeData<'a, Other>, Output=bool>,
    R: DefaultRequirement,
    MakeDefaultFor<T, R>: MakeDefault<T>
{
    let mask = other_cursor.select_level_node_unchecked(other, n, index);
    
    if N::VALUE == Other::LevelCount::VALUE - 1 {
        // terminal node with data
        return match f {
            None => Some(make_terminal_block(other, other_cursor, mask, required)),
            Some(f) => make_terminal_block_filtered(other, other_cursor, mask, f, required),
        }
    }
    
    let mut node_mask: Conf::Mask = BitBlock::zero();
    
    const{ assert!(<Other::LevelMask as BitBlock>::Size::VALUE <= 128); }
    let mut childs: ArrayVec<
        BlockPtr<T, Conf>, 
        128
    > = Default::default();    
    
    // Gather children.
    // Alternatively, we could construct block with `mask.count_bits()` capacity 
    // and write directly to it. TODO: try and benchmark that.
    mask.traverse_bits(|index| {
        if let Some(child_node) = from_tree_impl(other, other_cursor, n.inc(), index, empty_branch_blocks, f, required){
            node_mask.set_bit_unchecked::<true>(index);
            childs.push_unchecked(child_node);
        }
        Continue(())
    });
    
    let childs_len = childs.len() as u8;
    if childs_len == 0 {
        return None;
    }

    // 1. Construct block.
    let (mut block, start_index) = if const {R::Required::VALUE} {
        let cap = childs_len + 1;
        let mut block = BlockPtr::new::<T>(cap);
        block.write_child_at(
            0,
            empty_branch_blocks.as_ref()[N::VALUE+1],
        );
        (block, 1)
    } else {
        let cap = childs_len;
        (BlockPtr::new::<T>(cap), 0)    
    };         

    // 2. Set mask.
    *block.mask_mut() = node_mask.clone();
    
    // 3. memcpy childs to node
    {
        let childs = ManuallyDrop::new(childs);
        ptr::copy_nonoverlapping(
            childs.as_ptr(), 
            block.children_ptr::<BlockPtr<T, Conf>>().add(start_index),
            childs.len()
        );
    }    

    // 4. Write child_indices.
    let mut dense_index = start_index as u8;
    node_mask.traverse_bits(|sparse_index| {
        *block.child_indices_mut().get_unchecked_mut(sparse_index) = dense_index;
        dense_index += 1;
        Continue(())
    });
    
    // 5. Set len.
    block.set_len(childs_len);    
    
    Some(block)
}

fn from_tree<T, Conf, R, F, Other>(from: Other, f: Option<&F>) -> Tree<T, Conf, R>
where
    for<'a, 'b> F: UnaryFunction<&'a HibitTreeData<'b, Other>, Output=bool>,
    Other: RegularHibitTree<
        LevelMask  = Conf::Mask,
        LevelCount = Conf::LevelCount,
    >,
    Conf:Config,
    R: DefaultRequirement, 
    MakeDefaultFor<T, R>: MakeDefault<T>,    
{
    let empty_branch_blocks = make_empty_branch_blocks::<T, Conf, R>();
    let mut other_cursor = HibitTreeCursor::new(&from);
    let root = unsafe{
        from_tree_impl(&from, &mut other_cursor, ConstUsize::<0>, 0, &empty_branch_blocks, f, R::default())
        .unwrap_or_else(||make_empty_root::<T, Conf, R>(&empty_branch_blocks))
    };
    Tree{
        root,
        empty_branch_blocks,
        phantom_data: Default::default(),
    }    
} 

impl<T, Conf, R, Other> FromHibitTree<Other> for Tree<T, Conf, R>
where
    Other: RegularHibitTree<
        LevelMask  = Conf::Mask,
        LevelCount = Conf::LevelCount,
    >,
    Conf:Config,
    R: DefaultRequirement, 
    MakeDefaultFor<T, R>: MakeDefault<T>,
{
    fn from_tree(from: Other) -> Self {
        // TODO: special optimized case for EXACT_HIERARCHY.
        
        struct UnreachableBool;
        impl<T> UnaryFunction<T> for UnreachableBool{
            type Output = bool;
        
            #[inline]
            fn exec(&self, _: T) -> Self::Output {
                unreachable!()
            }
        }            
        from_tree(from, None::<&UnreachableBool>)
    }

    fn from_filtered_tree<F>(from: Other, f: F) -> Self
    where
        for<'a, 'b> F: UnaryFunction<&'a HibitTreeData<'b, Other>, Output=bool>
    {
        from_tree(from, Some(&f))
    }
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use super::*;
    use crate::hibit_tree::LazyHibitTree;
    
    #[test]
    fn from_test(){
        type Map = Tree<usize, Config64bit<3>, ReqDefault>; 
        let mut tree = Map::new();
        
        for i in 0..1000 {
            tree.insert(i, i);
        }
        
        let copied = tree.map(|i: &usize| *i);
        let tree2: Map = copied.materialize();
        
        assert_equal(
            tree2.iter().map(|(_, v)| *v), 
            0..1000
        );
    }
    
    #[test]
    fn from_filtered_test(){
        type Map = Tree<usize, Config64bit<3>, ReqDefault>; 
        let mut tree = Map::new();
        
        for i in 0..1000 {
            tree.insert(i, i);
        }
        
        let copied = tree.map(|i: &usize| *i);
        let tree2: Map = copied.materialize_filtered(|&i: &usize| i != 0);
        
        assert_equal(
            tree2.iter().map(|(_, v)| *v), 
            1..1000
        );
    }    
}