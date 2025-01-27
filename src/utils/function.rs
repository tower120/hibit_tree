/// Implementable nullary [Fn].
/// 
/// Closures and nullary `fn`s implement it.
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
/// 
/// Closures and unary `fn`s implement it.
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
/// 
/// Closures and binary `fn`s implement it.
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

/// Generates stateless generic [UnaryFunction]. Ala generic closure.
/// 
/// # Syntax
/// 
/// ```text
/// fun!([generics list] |argument: type| -> type { .. }
/// fun!([generics list] |argument: type| where [where bounds] -> type { .. }
/// ```
/// 
/// # Examples
/// 
/// ```
/// # use hibit_tree::fun;
/// # use hibit_tree::utils::Primitive;
/// fun!(['a, T: Primitive, I: Iterator<Item=&'a T>] |xs: I| -> T {
///     xs.fold(T::ZERO, |acc, v| acc + *v) 
/// });
/// ```
/// 
/// With `where` bounds:
/// 
/// ```
/// # use hibit_tree::fun;
/// # use hibit_tree::utils::Primitive;
/// fun!(['a, T, I] |xs: I| -> T where [T: Primitive, I: Iterator<Item=&'a T>] {
///     xs.fold(T::ZERO, |acc, v| acc + *v) 
/// });
/// ```
#[macro_export]
macro_rules! fun {
    // --- UnaryFunction ---

    ([$($generics:tt)*] |$arg:ident: $arg_type:ty| -> $output:ty $body:block) => {
        fun!([$($generics)*] |$arg: $arg_type| -> $output where [] $body)
    };
    
    ([$($generics:tt)*] |$arg:ident: $arg_type:ty| -> $output:ty where [$($bounds:tt)*] $body:block) => {
        {
            struct F;
            impl<$($generics)*> $crate::utils::function::UnaryFunction<$arg_type> for F
            where
                $($bounds)*
            {
                type Output = $output;
    
                #[inline]
                fn exec(&self, mut $arg: $arg_type) -> Self::Output {
                    $body
                }
            }
            F
        }
    };
}