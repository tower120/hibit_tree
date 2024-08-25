use arrayvec::ArrayVec;
use std::ops::ControlFlow::Continue;

use crate::const_utils::{ConstInteger, ConstUsize};
use crate::{
    BitBlock, FromBitmapTree, BitmapTree, BitmapTreeCursor,
    BitmapTreeCursorTypes, BitmapTreeTypes
};
use crate::utils::Take;

use super::node::{empty_node, NodePtr};
use super::{DenseTree, DataIndex, Mask};

type CursorData<'src, 'state, L> = 
    <<L as BitmapTreeTypes<'src>>
    ::Cursor as BitmapTreeCursorTypes<'state>>
    ::Data;

// TODO: move somewhere up, use in iter
#[inline]
fn block_start<S: BitmapTree, N: ConstInteger>(index: usize) -> usize {
    index << (
        <S as BitmapTree>::LevelMask::SIZE.ilog2() as usize * 
        (S::LevelCount::VALUE - N::VALUE - 1)
    )
}

#[inline]
unsafe fn make_terminal_node<'src, L, F>(
    other: &'src L,
    other_cursor: &mut <L as BitmapTreeTypes<'src>>::Cursor,
    mask: Mask,
    cap: u8,
    key_acc: usize,
    mut push_data: F
) -> NodePtr
where
    L: BitmapTree<LevelMask = Mask>,
    F: for<'a> FnMut(usize, CursorData<'src, 'a, L>) -> DataIndex
{
    let raw_node = NodePtr::raw_new::<DataIndex>(cap, mask);
    mask.traverse_bits(|index| {
        let data = other_cursor.data_unchecked(other, index);
        let key = key_acc + index; 
        let data_index = push_data(key, data);
        NodePtr::raw_push_within_capacity(raw_node, index, data_index);
        
        Continue(())
    });
    return NodePtr::raw_finalize(raw_node, 0);    
}

#[inline(always)]
unsafe fn from_exact_sparse_hierarchy<'src, L, N, F>(
    other: &'src L,
    other_cursor: &mut <L as BitmapTreeTypes<'src>>::Cursor,
    n: N,
    index: usize,
    key_acc: usize,
    push_data: &mut F,
) -> NodePtr
where
    L: BitmapTree<LevelMask = Mask>,
    F: for<'a> FnMut(usize, CursorData<'src, 'a, L>) -> DataIndex,
    N: ConstInteger,
{
    assert!(L::EXACT_HIERARCHY);
    
    let mask = other_cursor.select_level_node_unchecked(other, n, index)
               .take_or_clone();
    let len = mask.count_ones() as u8;
    let cap = len + 1;
    
    if N::VALUE == L::LevelCount::VALUE - 1 {
        // terminal node with data
        return make_terminal_node(other, other_cursor, mask, cap, key_acc, push_data);
    }
    
    let mut raw_node = NodePtr::raw_new::<NodePtr>(cap, mask);
    mask.traverse_bits(|index| {
        // TODO: try calculate key the same in iter. Benchmark.
        // go deeper
        let key_acc = key_acc + block_start::<L, N>(index); 
        let child_node = from_exact_sparse_hierarchy(
            other, other_cursor, n.inc(), index, key_acc, push_data
        );
        
        // connect to current
        NodePtr::raw_push_within_capacity(raw_node, index, child_node);
        
        Continue(())
    });
    
    let empty_child = empty_node(n.inc(), L::LevelCount::default());
    NodePtr::raw_finalize(raw_node, empty_child)
}

#[inline(always)]
unsafe fn from_sparse_hierarchy<'src, L, N, F>(
    other: &'src L,
    other_cursor: &mut <L as BitmapTreeTypes<'src>>::Cursor,
    n: N,
    index: usize,
    key_acc: usize,
    push_data: &mut F,
) -> Option<NodePtr>
where
    L: BitmapTree<LevelMask = Mask>,
    F: for<'a> FnMut(usize, CursorData<'src, 'a, L>) -> DataIndex,
    N: ConstInteger,
{
    let mask = other_cursor.select_level_node_unchecked(other, n, index)
               .take_or_clone();
    
    if N::VALUE == L::LevelCount::VALUE - 1 {
        // terminal node with data
        let len = mask.count_ones() as u8;
        let cap = len + 1;
        return Some(make_terminal_node(other, other_cursor, mask, cap, key_acc, push_data));        
    }
    
    let mut node_mask = Mask::zero();
    let mut childs: ArrayVec<NodePtr, {Mask::SIZE}> = Default::default(); 
    
    mask.traverse_bits(|index| {
        let key_acc = key_acc + block_start::<L, N>(index);
        if let Some(child_node) = from_sparse_hierarchy(other, other_cursor, n.inc(), index, key_acc, push_data){
            node_mask.set_bit::<true>(index);
            childs.push_unchecked(child_node);
        }
        Continue(())
    });
    
    if childs.is_empty(){
        None
    } else {
        let empty_child = empty_node(n.inc(), L::LevelCount::default());
        Some(NodePtr::from_parts(node_mask, childs.as_slice(), empty_child))
    }
}

impl<From, T, const DEPTH: usize> FromBitmapTree<From> for DenseTree<T, DEPTH>
where
    ConstUsize<DEPTH>: ConstInteger,
    From: BitmapTree<
        LevelMask  = Mask,
        LevelCount = ConstUsize<DEPTH>,
    >,
    for<'a> From: BitmapTreeTypes<'a,
        Cursor: for<'b> BitmapTreeCursorTypes<'b, 
            Data = T
        >,
    >,    
{
    fn from_sparse_hierarchy(other: From) -> Self {
        let mut other_cursor = <From as BitmapTreeTypes>::Cursor::new(&other);        
        let mut data: Vec<T> = Vec::with_capacity(1);
        unsafe{ data.set_len(1); }
        
        let mut keys = vec![usize::MAX];
        
        let mut push_fn = |index, value| -> DataIndex {
            let i = data.len(); 
            data.push(value);
            keys.push(index);
            i as DataIndex
        };

        let root = unsafe {
            if <From as BitmapTree>::EXACT_HIERARCHY {
                from_exact_sparse_hierarchy(
                    &other, &mut other_cursor, ConstUsize::<0>, 0, 0, &mut push_fn
                )
            } else {
                from_sparse_hierarchy(
                    &other, &mut other_cursor, ConstUsize::<0>, 0, 0, &mut push_fn
                ).unwrap_unchecked()
            }
        };
        Self{ root, data, keys }
    }
}