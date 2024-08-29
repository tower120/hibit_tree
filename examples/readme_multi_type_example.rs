use itertools::assert_equal;
use hibit_tree::{intersection, RegularHibitTree, DenseTree, HibitTree};

fn main(){
    // index as user-id.
    type Tree<T> = DenseTree<T, 4>;
    let mut ages : Tree<usize>  = Default::default();
    ages.insert(100, 20);
    ages.insert(200, 30);
    ages.insert(300, 40);
    
    let mut names: Tree<String> = Default::default();
    names.insert(200, "John".into());
    names.insert(234, "Zak".into());
    names.insert(300, "Ernie".into());
    
    let users = intersection(&ages, &names)
        .map(|(i, s): (&usize, &String)| format!("{s} age: {i}"));
    
    assert_equal(users.iter(), [
        (200, String::from("John age: 30")),
        (300, String::from("Ernie age: 40")),
    ]);
}