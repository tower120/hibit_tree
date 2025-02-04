use wide::u64x2;
use crate::BitBlock;
use crate::const_utils::{ConstInteger, ConstUsize};

pub trait Config: Default + 'static {
    type Mask: BitBlock;
    type LevelCount: ConstInteger;
}

#[derive(Default)]
pub struct _64bit<const LEVELS: usize>;

impl<const LEVELS: usize> Config for _64bit<LEVELS>
where
    ConstUsize<LEVELS>: ConstInteger,
{
    type Mask = u64;
    type LevelCount = ConstUsize<LEVELS>;
}

#[derive(Default)]
pub struct _128bit<const LEVELS: usize>;

impl<const LEVELS: usize> Config for _128bit<LEVELS>
where
    ConstUsize<LEVELS>: ConstInteger,
{
    type Mask = u64x2;
    type LevelCount = ConstUsize<LEVELS>;
}

// We do not implement Config256bit, since we use one index as sentinel.
// That would not fit u8 range (256+1 > u8::MAX).