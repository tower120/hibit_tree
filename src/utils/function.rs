/// Fn(&T0, &T1) -> Out
pub trait FnRR<T0, T1>
    : Fn(&T0, &T1) -> Self::Out
{
    type Out;
}

impl<F, T0, T1, Out> FnRR<T0, T1> for F 
where
    F: Fn(&T0, &T1) -> Out,
{
    type Out = Out; 
}
