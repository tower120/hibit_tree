use std::borrow::Borrow;

pub(crate) mod primitive;
pub(crate) mod array;

pub use primitive::*;
pub use array::*;

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

/// Means of converting T/&T to value.
/// 
/// Allows to store/pass reference or value generically.
/// In conjunction with [Borrow] can be used as something like C++'s &&T 
/// (forwarding reference), that can be taken by ref or value.
/// 
/// Everything zero overhead.
pub trait Take<T>{
    /// Takes self as T.
    /// 
    /// # Panics
    /// 
    /// Panics at compile-time if Self is &T.
    fn take(self) -> T;
    
    /// Returned Option variant can
    /// be used for compile-time switch. 
    fn try_take(self) -> Option<T>;
    
    /// Return self as is for T, clone for &T.
    fn take_or_clone(self) -> T where T: Clone;
}

impl<T> Take<T> for T{
    #[inline]
    fn take(self) -> T{
        self
    }
    
    #[inline]
    fn try_take(self) -> Option<T>{
        Some(self)
    }
    
    #[inline]
    fn take_or_clone(self) -> T{
        self
    }
}

impl<T> Take<T> for &T{
    #[inline]
    fn take(self) -> T{
        const{ panic!("Trying to take &T by value.") }
    }
    
    #[inline]
    fn try_take(self) -> Option<T>{
        None
    }
    
    #[inline]
    fn take_or_clone(self) -> T where T: Clone{
        self.clone()
    }
}