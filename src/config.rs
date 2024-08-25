//! Hierarchy configurations for [SparseArray].
//! 
//! General rule: use the smallest depth possible. 

pub use crate::sparse_tree_levels::SparseTreeLevels;

use crate::level::{IntrusiveListLevel, SingleBlockLevel};
use crate::level_block::Block;

macro_rules! generate {
    ($LO: ty, $L1: ty, $LN: ty) => {
        type L0 = SingleBlockLevel<$LO>;
        type L1 = IntrusiveListLevel<$L1>;
        type LN = IntrusiveListLevel<$LN>;
        
        pub type depth_1 = (L0,);
        pub type depth_2 = (L0, L1);
        pub type depth_3 = (L0, L1, LN);
        pub type depth_4 = (L0, L1, LN, LN);
        pub type depth_5 = (L0, L1, LN, LN, LN);
        pub type depth_6 = (L0, L1, LN, LN, LN, LN);
        pub type depth_7 = (L0, L1, LN, LN, LN, LN, LN);
        pub type depth_8 = (L0, L1, LN, LN, LN, LN, LN, LN);
    };
}

/// 64 element blocks.
#[allow(non_camel_case_types)]
pub mod width_64 {
    use super::*;
    generate!(Block<u64, [u8; 64]>, Block<u64, [u16; 64]>, Block<u64, [u32; 64]>);    
}

/// 128 element blocks.
#[cfg(feature = "simd")]
#[cfg_attr(docsrs, doc(cfg(feature = "simd")))]
#[allow(non_camel_case_types)]
pub mod width_128 {
    use super::*;
    generate!(Block<wide::u64x2, [u8; 128]>, Block<wide::u64x2, [u16; 128]>, Block<wide::u64x2, [u32; 128]>);
}

/// 256 element blocks.
#[cfg(feature = "simd")]
#[cfg_attr(docsrs, doc(cfg(feature = "simd")))]
#[allow(non_camel_case_types)]
pub mod width_256{
    use super::*;
    generate!(Block<wide::u64x4, [u8; 256]>, Block<wide::u64x4, [u16; 256]>, Block<wide::u64x4, [u32; 256]>);
}

/*pub type _64x1 = w64::d1;
pub type _64x2 = w64::d2;
pub type _64x3 = w64::d3;
pub type _64x4 = w64::d4;
pub type _64x5 = w64::d5;
pub type _64x6 = w64::d6;
pub type _64x7 = w64::d7;
pub type _64x8 = w64::d8;*/

/*
/// Hierarchy, with blocks that use SBO[^sbo] for child array.
///
/// Approximately x2 slower[^slower] then full-sized blocks, but almost x10 smaller.  
/// Full block required to have child array size of the bitmask size. This store
/// child pointers in dense form, in order they appear in bitmask population.
/// 
/// We try to fit SBO into bitmask SIMD-align, so small buffer size 
/// depends on node width:
/// - 6 elements for [width_64]
/// - 7 elements for [width_128]
/// - 15 elements for [width_256]
/// 
/// TODO: Description from hi_sparse_bitset
/// 
/// [^sbo]: small buffer optimization.
/// [^slower]: depends on level count - the more => the slower. Looks like the worst case
/// scenario is x3 slow-down for 8 level hierarchy. Best case is around x1.25-1.5 for 3 levels and less.
#[deprecated = "Deprecated in CompactSparseArray favor."]
pub(crate) mod sbo {
    use super::*;
    use crate::level_block::SmallBlock;
    
    #[allow(non_camel_case_types)]
    pub mod width_64{
        use super::*;
        generate!(
            Block<u64, [u8; 64]>,               // Use full-sized block for root.
            SmallBlock<u64, [u8;1], [u16;64], [u16;7]>,
            SmallBlock<u64, [u8;1], [u32;64], [u32;6]>
        );
    }
    
    #[cfg(feature = "simd")]
    #[cfg_attr(docsrs, doc(cfg(feature = "simd")))]
    #[allow(non_camel_case_types)]
    pub mod width_128{
        use super::*;
        generate!( 
            Block<wide::u64x2, [u8; 128]>,      // Use full-sized block for root.
            SmallBlock<wide::u64x2, [u8;2], [u16;128], [u16;7]>,
            SmallBlock<wide::u64x2, [u8;2], [u32;128], [u32;7]>
        );
    }
    
    #[cfg(feature = "simd")]
    #[cfg_attr(docsrs, doc(cfg(feature = "simd")))]
    #[allow(non_camel_case_types)]
    pub mod width_256{
        use super::*;
        generate!( 
            Block<wide::u64x4, [u8; 256]>,      // Use full-sized block for root.
            SmallBlock<wide::u64x4, [u8;4], [u16;256], [u16;14]>,
            SmallBlock<wide::u64x4, [u8;4], [u32;256], [u32;15]>
        );
    }
}
*/