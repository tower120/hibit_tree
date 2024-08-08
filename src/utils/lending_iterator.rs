pub trait LendingIterator{
    // The only place where GAT work as needed?
    type Item<'a> where Self:'a;
    
    fn next(&mut self) -> Option<Self::Item<'_>>;
}