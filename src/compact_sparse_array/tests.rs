use itertools::assert_equal;
use crate::FromSparseHierarchy;
use crate::sparse_hierarchy::SparseHierarchy;
use super::CompactSparseArray;

#[test]
fn test(){
    let mut a: CompactSparseArray<usize, 3> = Default::default();
    assert_eq!(a.get(15), None);

    *a.get_or_insert(15) = 89;
    assert_eq!(a.get(15), Some(&89));

    *a.get_or_insert(16) = 90;
    assert_eq!(a.get(20), None);
            
    assert_eq!(*a.get_or_insert(15), 89);
    assert_eq!(*a.get_or_insert(0), 0);
    assert_eq!(*a.get_or_insert(100), 0);
}

#[test]
fn test2(){
    let mut a: CompactSparseArray<usize, 3> = Default::default();

    #[cfg(not(miri))]
    const COUNT: usize = 200_000;
    #[cfg(miri)]
    const COUNT: usize = 200;
    
    for i in 0..COUNT{
        *a.get_or_insert(i) = i;
    }
    
    for i in 0..COUNT{
        let v = a.get(i);
        assert_eq!(v, Some(&i));
    }
    
    for i in 0..COUNT{
        a.remove(i);
    }

    for i in 0..COUNT{
        let v = a.get(i);
        assert_eq!(v, None);
    }
}

#[test]
fn test_remove(){
    let mut a: CompactSparseArray<usize, 2> = Default::default();
    *a.get_or_insert(10) = 10;
    *a.get_or_insert(11) = 11;
    //*a.get_or_insert(12) = 12;
    
    a.remove(11);
    a.remove(10);
}    

#[test]
fn test_remove2(){
    let mut a: CompactSparseArray<usize, 2> = Default::default();

    for i in 0..2000{
        *a.get_or_insert(i) = i;
    }

    a.remove(0).unwrap();
    a.remove(1).unwrap();
}

#[test]
fn test_exact_from(){
    let mut a: CompactSparseArray<usize, 4> = Default::default();
    a.insert(15, 15);
    a.insert(4500, 4500);
    
    let b = CompactSparseArray::from_sparse_hierarchy(a);
    assert_equal(b.iter(),
        [(15, &15), (4500, &4500)]
    );
}
