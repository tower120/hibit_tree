use std::iter;
use hibit_tree::{DenseTree, HibitTree, multi_intersection};
use hibit_tree::utils::LendingIterator;

fn main(){
    /// [store_id; good_amount]
    type Goods = DenseTree<usize, 4>;
    
    let mut apples : Goods = Default::default();
    apples.insert(0, 12);
    apples.insert(3, 40);
    
    let mut oranges: Goods = Default::default();
    oranges.insert(0, 4);
    oranges.insert(1, 15);
    oranges.insert(3, 40);     
    
    let mut carrots: Goods = Default::default();
    carrots.insert(1, 5);
    carrots.insert(3, 100);
    
    // We want 5 apples, 20 oranges, 7 carrots - from the SAME store.
    let goods            = [&apples, &oranges, &carrots];
    let min_goods_amount = [5      , 20      , 7       ];
    
    let intersection = multi_intersection(goods.iter().copied());
    let mut iter = intersection.iter();
    while let Some((store_id, goods_amount /*: impl Iterator<usize> */)) = 
        LendingIterator::next(&mut iter)
    {
        // `goods_amount` iterator has the same order as `goods`.
        let contains = 
            iter::zip(goods_amount.clone(), min_goods_amount.iter())
            .find(|(amount, min)| min <= amount )
            .is_some();
        if !contains{ continue }
        
        println!("found at store {store_id} : {:?}", goods_amount.collect::<Vec<_>>());
        
        break;
    }
}