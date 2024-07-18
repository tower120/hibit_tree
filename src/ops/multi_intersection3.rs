use std::marker::PhantomData;
use std::borrow::Borrow;
use std::slice;
use arrayvec::ArrayVec;
use crate::BitBlock;
use crate::const_utils::{ConstArray, ConstInteger};
use crate::sparse_hierarchy2::{SparseHierarchy2, SparseHierarchyState2};
use crate::utils::{Array, Borrowable, Take};

pub struct MultiIntersection<Iter, R> {
    iter: Iter,
    r: R,
}

type IterItem<Iter> = <<Iter as Iterator>::Item as Borrowable>::Borrowed;

impl<Iter, R> SparseHierarchy2 for MultiIntersection<Iter, R>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> R: Resolve<Item<'a> = <IterItem<Iter> as SparseHierarchy2>::Data<'a>> + 'a
{
    type LevelCount = <IterItem<Iter> as SparseHierarchy2>::LevelCount;

    type LevelMaskType = <IterItem<Iter> as SparseHierarchy2>::LevelMaskType;
    type LevelMask<'a> = Self::LevelMaskType where Self: 'a;
    
    type DataType = R::Out;
    type Data<'a> = R::Out where Self: 'a;

    unsafe fn data<I>(&self, level_indices: I) -> Option<Self::Data<'_>>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        todo!()
    }

    unsafe fn data_unchecked<I>(&self, level_indices: I) -> Self::Data<'_>
    where
        I: ConstArray<Item=usize, Cap=Self::LevelCount> + Copy
    {
        todo!()
    }

    type State = MultiIntersectionState<Iter, R>;
}

const N: usize = 32;
type StatesItem<Iter> = (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy2>::State);

pub struct MultiIntersectionState<Iter, R>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
{
    states: ArrayVec<
        (<Iter as Iterator>::Item, <IterItem<Iter> as SparseHierarchy2>::State),
        N
    >,    
    empty_below_n: usize,
    terminal_node_mask: <IterItem<Iter> as SparseHierarchy2>::LevelMaskType,
    phantom_data: PhantomData<(Iter, R)>
}

impl<Iter, R> SparseHierarchyState2 for MultiIntersectionState<Iter, R>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> R: Resolve<Item<'a> = <IterItem<Iter> as SparseHierarchy2>::Data<'a>> + 'a
{
    type This = MultiIntersection<Iter, R>;

    #[inline]
    fn new(this: &Self::This) -> Self {
        let states = ArrayVec::from_iter(
            this.iter.clone()
                .map(|array|{
                    let state = SparseHierarchyState2::new(array.borrow()); 
                    (array, state)
                })
        );
        
        Self {
            states,
            empty_below_n: usize::MAX,
            terminal_node_mask: BitBlock::zero(),
            phantom_data: PhantomData,
        }        
    }

    #[inline]
    unsafe fn select_level_node<'a, N: ConstInteger>(
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        // if we know that upper levels returned empty - return early.
        if N > self.empty_below_n {
            return BitBlock::zero(); 
        }
        
        let mut states_iter = self.states.iter_mut();
        
        let mut acc_mask = 
            if let Some((array, array_state)) = states_iter.next(){
                array_state.select_level_node(
                    (&*array).borrow(), level_n, level_index
                ).take_or_clone()
            } else {
                return BitBlock::zero();
            };
        
        for (array, array_state) in states_iter {
            let mask = array_state.select_level_node(
                (&*array).borrow(), level_n, level_index
            );
            acc_mask &= mask.borrow();
        }
        
        self.empty_below_n = if acc_mask.is_zero(){
             N
        } else {
            usize::MAX
        };
        
        /*const*/ if N::VALUE == <Self::This as SparseHierarchy2>::LevelCount::VALUE - 1 {
            self.terminal_node_mask = acc_mask.clone(); 
        }
        
        acc_mask
    }

    #[inline]
    unsafe fn select_level_node_unchecked<'a, N: ConstInteger> (
        &mut self, this: &'a Self::This, level_n: N, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::LevelMask<'a> {
        let mut states_iter = self.states.iter_mut();
        
        let mut acc_mask = 
            if let Some((array, array_state)) = states_iter.next(){
                array_state.select_level_node_unchecked(
                    (&*array).borrow(), level_n, level_index
                ).take_or_clone()
            } else {
                return BitBlock::zero();
            };
        
        for (array, array_state) in states_iter {
            let mask = array_state.select_level_node_unchecked(
                (&*array).borrow(), level_n, level_index
            );
            acc_mask &= mask.borrow();
        }            
        acc_mask
    }

    #[inline]
    unsafe fn data<'a>(&self, this: &'a Self::This, level_index: usize) 
        -> Option<<Self::This as SparseHierarchy2>::Data<'a>> 
    {
        if !self.terminal_node_mask.get_bit(level_index){
            return None;
        }
        
        Some(self.data_unchecked(this, level_index))
    }

    #[inline]
    unsafe fn data_unchecked<'a>(
        &self, this: &'a Self::This, level_index: usize
    ) -> <Self::This as SparseHierarchy2>::Data<'a> {
        
        let datas = self.states.iter()
            .map(|(array, array_state)| unsafe { 
                array_state.data_unchecked(array.borrow(), level_index)
            });     
        this.r.resolve(datas)
        
        //(this.f)(DataIter{ level_index, states_iter: self.states.iter() })
    }
}

/// Need this, because Rust does not support generic closures.
pub trait Resolve{
    type Item<'a> where Self:'a;
    type Out;
    
    /// Guaranteed to have at least one element.
    fn resolve<'a, I>(&'a self, elements: I) -> Self::Out
    where
        I: Iterator<Item=Self::Item<'a>>;
}

// TODO: use this
/*// States slice to Data iterator adapter.
pub struct DataIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>
{
    level_index: usize,
    states_iter: slice::Iter<'a, StatesItem<I>>
}

impl<'a, I> Iterator for DataIter<'a, I>
where
    I: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>
{
    /// <I::Item as SparseHierarchy2>::Data<'a>
    type Item = <IterItem<I> as SparseHierarchy2>::Data<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Compiler optimizes away additional branching here.
        self.states_iter
            .next()
            .map(|(array, array_state)| unsafe { 
                array_state.data_unchecked(array.borrow(), self.level_index)
            })
    }
}*/

impl<Iter, R> Borrowable for MultiIntersection<Iter, R>{ type Borrowed = Self; }

/*#[inline]
pub fn multi_intersection2<Iter, F, T>(iter: Iter, f: F) 
    -> MultiIntersection<Iter, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> F: Fn(DataIter<'a, Iter>) -> T
{
    MultiIntersection{ iter, f, phantom_data: Default::default() }
}
*/

#[inline]
pub fn multi_intersection<Iter, R>(iter: Iter, r: R) 
    -> MultiIntersection<Iter, R>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    for<'a> R: Resolve<Item<'a> = <IterItem<Iter> as SparseHierarchy2>::Data<'a>> + 'a
{
    MultiIntersection{ iter, r }
}


pub struct FoldResolve<Iter, Init, F, T>{
    init: Init,
    f: F,
    phantom_data: PhantomData<(Iter, T)>
}

impl<Iter, Init, F, T> Resolve for FoldResolve<Iter, Init, F, T>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>>,
    for<'a> Init: Fn(<IterItem<Iter> as SparseHierarchy2>::Data<'a>) -> T,
    for<'a> F: Fn(T, <IterItem<Iter> as SparseHierarchy2>::Data<'a>) -> T,    
{
    type Item<'a> = <IterItem<Iter> as SparseHierarchy2>::Data<'a> where Self: 'a ;
    type Out = T;

    #[inline]
    fn resolve<'a, I>(&'a self, elements: I) -> Self::Out
    where
        I: Iterator<Item=Self::Item<'a>>
    {
        todo!()
    }
}

/// Fold style intersection
#[inline]
pub fn fold_intersection<Iter, Init, F, T>(iter: Iter, init: Init, f: F) 
    -> MultiIntersection<Iter, FoldResolve<Iter, Init, F, T>>
where
    Iter: Iterator<Item: Borrowable<Borrowed: SparseHierarchy2>> + Clone,
    
{
    todo!()
    //MultiIntersection{ iter, FoldResolve{init, f} }
}



#[cfg(test)]
mod test{
    use itertools::assert_equal;
    use crate::compact_sparse_array2::CompactSparseArray2;
    use crate::ops2::multi_intersection3::{multi_intersection, Resolve};
    use crate::sparse_hierarchy2::SparseHierarchy2;
    use crate::utils::Take;

    #[test]
    fn smoke_test(){
        type Array = CompactSparseArray2<usize, 3>;
        let mut a1 = Array::default();
        let mut a2 = Array::default();
        let mut a3 = Array::default();
        
        *a1.get_or_insert(10) = 10;
        *a1.get_or_insert(15) = 15;
        *a1.get_or_insert(200) = 200;
        
        *a2.get_or_insert(100) = 100;
        *a2.get_or_insert(15)  = 15;
        *a2.get_or_insert(200) = 200;
        
        *a3.get_or_insert(300) = 300;
        *a3.get_or_insert(15)  = 15;
        
        let arrays = [a1, a2, a3];
        
        struct R;
        impl Resolve for R {
            type Item<'a> = &'a usize where Self: 'a;
            type Out = usize;

            fn resolve<'a, I>(&'a self, elements: I) -> Self::Out
            where
                I: Iterator<Item=Self::Item<'a>>
                //I: Iterator<Item=&'a usize>
            {
                let mut s: usize = 0;
                for e in elements{
                    let v: usize = e.take_or_clone(); 
                    s += v;
                } 
                s
                //elements.sum()
            }
        }
        
        let intersection = multi_intersection(arrays.iter(), R ); 
        
        assert_equal(intersection.iter(), [(15, 45)]);
    }

}