use std::borrow::{Borrow, BorrowMut};

pub trait Borrowable: Borrow<Self::Borrowed>{
    type Borrowed;
}

// Not necessary?
/*pub trait BorrowableMut: Borrowable + BorrowMut<Self::Borrowed>{}*/