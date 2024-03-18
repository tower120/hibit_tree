use crate::primitive::Primitive;

pub trait Block: Sized /*+ Default*/{
    fn empty() -> Self; 
    fn is_empty() -> bool;
    
    const SIZE_POT_EXPONENT: usize;
    
/*    // Does this needed?
    type Data;
    fn data(&self) -> &Self::Data; 
    unsafe fn data_mut(&mut self) -> &mut Self::Data;*/
    
    fn as_u64_mut(&mut self) -> &mut u64;
    
    /// Restore empty state, after as_u64_mut() mutation.
    fn restore_empty_u64(&mut self);
}

/// Hierarchy block
pub trait HiBlock: Block {
    /*type Mask/*: BitBlock*/;
    
    fn mask(&self) -> &Self::Mask; 
    unsafe fn mask_mut(&mut self) -> &mut Self::Mask;*/
    
    // TODO: BlockIndex
    type Item: Primitive;
    
    /*/// # Safety
    ///
    /// - index is not checked for out-of-bounds.
    /// - index is not checked for validity (must exist).
    unsafe fn get_unchecked(&self, index: usize) -> Self::Item;*/
    
    /// Returns 0 if item does not exist at `index`.
    /// 
    /// # Safety
    /// 
    /// index is not checked for out-of-bounds.
    unsafe fn get_or_zero(&self, index: usize) -> Self::Item;
    
    /// # Safety
    ///
    /// `index` is not checked.
    unsafe fn get_or_insert(
        &mut self,
        index: usize,
        f: impl FnMut() -> Self::Item
    ) -> Self::Item;
    
    /// Return previous mask bit.
    /// 
    /// # Safety
    ///
    /// * `index` must be set
    /// * `index` is not checked for out-of-bounds.
    unsafe fn remove_unchecked(&mut self, index: usize);
    
/*    #[inline]
    fn is_empty(&self) -> bool {
        todo!()
        //Self::Mask::is_zero(self.mask())
    }*/
}
