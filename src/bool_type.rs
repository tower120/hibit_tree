pub trait BoolType: Default + Copy{
    const VALUE: bool;
}

#[derive(Default, Clone, Copy)]
pub struct TrueType;
impl BoolType for TrueType{
    const VALUE: bool = true;
}

#[derive(Default, Clone, Copy)]
pub struct FalseType;
impl BoolType for FalseType{
    const VALUE: bool = false;
}
