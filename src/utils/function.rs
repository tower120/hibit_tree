/// Implementable unary [Fn].
pub trait UnaryFunction<Arg> {
    type Output;
    fn exec(&self, arg: Arg) -> Self::Output;
}

impl<F, Arg, Out> UnaryFunction<Arg> for F
where
    F: Fn(Arg) -> Out
{
    type Output = Out;

    #[inline]
    fn exec(&self, arg: Arg) -> Self::Output {
        self(arg)
    }
}