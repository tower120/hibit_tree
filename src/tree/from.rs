use std::ops::ControlFlow::Continue;
use arrayvec::ArrayVec;
use crate::{FromHibitTree, RegularHibitTree};
use super::*;

#[inline]
unsafe fn make_terminal_block<'a, Other, T, Conf, R>(
    other: &'a Other,
    other_cursor: &mut <Other as HibitTreeTypes<'a>>::Cursor,
    mask: Conf::Mask,
    len: u8,
    _: R
) -> BlockPtr<T, Conf>
where
    Other: RegularHibitTree<LevelMask = Conf::Mask>,
    Conf: Config,
    R: DefaultRequirement,
    MakeDefaultFor<T, R>: MakeDefault<T> 
{
    // TODO: align capacity to pot?
    let (mut block, mut i) = if const {R::Required::VALUE} {
        let cap = len + 1;
        let mut block = BlockPtr::new::<T>(cap);
        block.write_child_at(
            0,
            <MakeDefaultFor<T, R> as MakeDefault<T>>::make_default(),
        );
        (block, 1)
    } else {
        let cap = len;
        (BlockPtr::new::<T>(cap), 0)    
    }; 
    *block.mask_mut() = mask.clone();        
    mask.traverse_bits(|index| {
        let data = other_cursor.data_unchecked(other, index);
        block.write_child_at(i, data);
        *block.child_indices_mut().get_unchecked_mut(index) = i as u8;
        i += 1;
        Continue(())
    });
    block.set_len(len);
    block    
}

#[inline(always)]
unsafe fn from_tree_impl<'a, T, Conf, Other, N, R>(
    other: &'a Other,
    other_cursor: &mut <Other as HibitTreeTypes<'a>>::Cursor,
    n: N,
    index: usize,
    empty_branch_blocks: &EmptyBranchBlocks<T, Conf>,
    required: R
) -> Option<BlockPtr<T, Conf>>
where
    Conf: Config,
    Other: RegularHibitTree<LevelMask=Conf::Mask>,
    N: ConstInteger,
    R: DefaultRequirement,
    MakeDefaultFor<T, R>: MakeDefault<T>
{
    let mask = other_cursor.select_level_node_unchecked(other, n, index);
    
    if N::VALUE == Other::LevelCount::VALUE - 1 {
        // terminal node with data
        let len = mask.count_ones() as u8;
        return Some(make_terminal_block(other, other_cursor, mask, len, required));
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
        if let Some(child_node) = from_tree_impl(other, other_cursor, n.inc(), index, empty_branch_blocks, required){
            node_mask.set_bit_unchecked::<true>(index);
            childs.push_unchecked(child_node);
        }
        Continue(())
    });
    
    if childs.is_empty(){
        None
    } else {
        let childs_len = childs.len() as u8; 

        // 1. Construct block
        let (mut block, mut i) = if const {R::Required::VALUE} {
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

        // 3. Write child_indices.
        let mut dense_index = i as u8;
        node_mask.traverse_bits(|sparse_index| {
            *block.child_indices_mut().get_unchecked_mut(sparse_index) = dense_index;
            dense_index += 1;
            Continue(())
        });

        // 4. Write children.
        for child in childs{
            block.write_child_at(i, child);
            i+=1;
        }
        block.set_len(childs_len);        
        Some(block)
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
        let empty_branch_blocks = make_empty_branch_blocks::<T, Conf, R>();
        let mut other_cursor = HibitTreeCursor::new(&from);
        let root = unsafe{
            from_tree_impl(&from, &mut other_cursor, ConstUsize::<0>, 0, &empty_branch_blocks, R::default())
            .unwrap()
        };
        Self{
            root,
            empty_branch_blocks,
            phantom_data: Default::default(),
        }
    }
}

#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use super::*;
    use crate::hibit_tree::LazyHibitTree;
    
    #[test]
    fn smoke_test(){
        type Map = Tree<usize, Config64bit<3>, ReqDefault>; 
        let mut tree = Map::new();
        
        for i in 0..1000 {
            tree.insert(i, i);
        }
        
        let copied = tree.map(|i: &usize| *i);
        let tree2: Map = copied.materialize();
        
        /* for (index, i) in tree2.iter(){
            println!("{i}");
        } */
        
        assert_equal(
            tree2.iter().map(|(_, v)| *v), 
            0..1000
        );
    }
}