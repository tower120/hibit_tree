/// Implementable nullary [Fn].
pub trait NullaryFunction {
    type Output;
    fn exec(&self) -> Self::Output;
}

impl<F, Out> NullaryFunction for F
where
    F: Fn() -> Out
{
    type Output = Out;

    #[inline]
    fn exec(&self) -> Self::Output {
        self()
    }
}

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

/// Implementable binary [Fn].
pub trait BinaryFunction<Arg0, Arg1> {
    type Output;
    fn exec(&self, arg0: Arg0, arg1: Arg1) -> Self::Output;
}

impl<F, Arg0, Arg1, Out> BinaryFunction<Arg0, Arg1> for F
where
    F: Fn(Arg0, Arg1) -> Out
{
    type Output = Out;

    #[inline]
    fn exec(&self, arg0: Arg0, arg1: Arg1) -> Self::Output {
        self(arg0, arg1)
    }
}