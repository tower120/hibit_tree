use std::borrow::Borrow;

/// `Borrowable` means it can be either T or &T.
/// 
/// Using this over [Borrow], allow accepting T or &T
/// as argument with type-deduction:
/// ```
/// # use hi_sparse_array::utils::Borrowable;
/// # use std::fmt::Debug;
/// #[derive(Debug)]
/// struct S;
/// impl Borrowable for S {type Borrowed = S;}
/// 
/// fn test(v: impl Borrowable<Borrowed: Debug>){
///     println!("{:?}", v.borrow());
/// }
///
/// fn main(){
///     test(S);
///     test(&S);
/// }
/// ```
/// While [Borrow] will fail to compile for this case:
/// 
/// ```compile_fail
/// # use std::borrow::Borrow;
/// # use std::fmt::Debug;
/// #[derive(Debug)]
/// struct S;
///
/// fn test<S: Debug>(v: impl Borrow<S>){
///     println!("{:?}", v.borrow());
/// }
///
/// fn main(){
///     test(S);
///     test(&S);   // error: type annotations needed.
///                 // cannot infer type for type parameter `S` declared on the function `test`
/// }
/// ```
pub trait Borrowable: Borrow<Self::Borrowed>{
    type Borrowed;
}

impl<T: Borrowable> Borrowable for &T{
    type Borrowed = T;
}

// Not necessary?
/*pub trait BorrowableMut: Borrowable + BorrowMut<Self::Borrowed>{}*/