# **HI**erarchical **BIT**map **TREE**

[![crates.io](https://img.shields.io/crates/v/hibit_tree.svg)](https://crates.io/crates/hibit_tree)
[![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)](#license)
[![Docs](https://docs.rs/hibit_tree/badge.svg)](https://docs.rs/hibit_tree)

Hierarchical bitmap tree is an integer-key fixed-depth prefix-tree 
with no memory overhead[^mem_overhead].
That have unique[^unique_ops], blazingly fast inter-container intersection[^unparalleled_intersection] and union.
That outperforms `HashMap<u32, T>`[^hashmap_conf] most of the time.

Think of it as a map that can do set things. And MUCH more efficiently[^intersection_efficiency]
then traditional set operations combined with map lookups.

* Always stable O(1) random access.

* Predictable insert/remove performance - no tree-balancing, or any hidden performance impact.

* Ordered by key[^sorting]. Have unordered contiguous iteration[^unordered_iter] as well.

* Fast inter-container equality and ordering [Not yet implemented].

[^intersection_efficiency]: Intersection operation directly over data container is much faster, than intersecting 
set + getting items from tree/map. Since with intersection directly over tree - we
are skipping additional tree traverse phase for actually getting data.

[^mem_overhead]: Tree nodes store child-pointers in a dense format - null-pointers are not stored.
While still have O(1) child access, like in traditional sparse format.

[^hashmap_conf]: HashMap<u32, T> with nohash-hasher and uniform key distribution -
ideal condition for HashMap.

[^unparalleled_intersection]: Since the very tree is a form of hierarchical bitmap 
it naturally acts as intersection acceleration structure.

[^unique_ops]: Usually tree/map data structures does not provide such operations.

[^sorting]: Which means, that it can be used for sort acceleration.

[^unordered_iter]: Unordered iteration is as fast as iterating Vec.

## Examples

### Sparse vector dot product

[examples/readme_sparse_vec_dot.rs](./examples/readme_sparse_vec_dot.rs)

```rust
type SparseVec = DenseTree<f32, 4>;

let mut v1: SparseVec = Default::default();
v1.insert(10, 1.0);
v1.insert(20, 10.0);
v1.insert(30, 100.0);

let mut v2: SparseVec = Default::default();
v2.insert(10, 1.0);
v2.insert(30, 0.5);

let mul = intersection(&v1, &v2)            // lazy element-wise mul
    .map(|(e1, e2): (&f32, &f32)| e1 * e2);
let dot: f32 = mul.iter().map(|(_index, element)| element).sum();

assert_eq!(dot, 51.0);
```

### Multi-type intersection

[examples/readme_multi_type_example.rs](./examples/readme_multi_type_example.rs)

```rust
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
```

### Multi-tree intersection with in-place filter[^inplace_filter]:

[examples/readme_store_example.rs](./examples/readme_store_example.rs)

```rust
/// [store_id; good_amount]
type Goods = DenseTree<usize, 4>;

let mut apples : Goods = Default::default();
apples.insert(0, 12);
apples.insert(3, 40);

let mut oranges: Goods = Default::default();
oranges.insert(0, 4);
oranges.insert(1, 15);
oranges.insert(3, 40);     

let mut carrots: Goods = Default::default();
carrots.insert(1, 5);
carrots.insert(3, 100);

// We want 5 apples, 20 oranges, 7 carrots - from the SAME store.
let goods            = [&apples, &oranges, &carrots];
let min_goods_amount = [5      , 20      , 7       ];

let intersection = multi_intersection(goods.iter().copied());
let mut iter = intersection.iter();
while let Some((store_id, goods_amount /*: impl Iterator<usize> */)) = 
    LendingIterator::next(&mut iter)
{
    // `goods_amount` iterator has the same order as `goods`.
    let contains = 
        iter::zip(goods_amount.clone(), min_goods_amount.iter())
        .find(|(amount, min)| min <= amount )
        .is_some();
    if !contains{ continue }
    
    println!("found at store {store_id} : {:?}", goods_amount.collect::<Vec<_>>());
    
    break;
}
```

[^inplace_filter]: hibit_tree acquire intersected data as we go. This is much 
faster than doing set intersection, and then getting data by keys. Since there
is no additional tree traverse. 

## Use cases

### Sparse-vector to sparse-vector multiplication

For dot product we need to element-wise multiply two vectors, then sum it's elements.

Multiply zero equals zero. Sparse vector mainly filled with zeroes.
In result of element-wise multiplication only elements that have BOTH sources non-null
will be non-null.

Let's represent hierarchical bitmap tree as a sparse vector. We store only non-zero 
elements into tree. Intersection of two such trees will return pairs of non-zero elements.
By mapping pairs to multiplication operation - we get element-wise multiplied sparse vector. 
By summing it's all elements - we get dot product.

See [examples/sparse_vector.rs](./examples/sparse_vector.rs)

### Compressed bitset

[hi_sparse_bitset](https://crates.io/crates/hi_sparse_bitset) basically use the same structure as
`hibit_tree`. Can be considered as special case of `hibit_tree`.

## How it works

Hierarchical bitmap tree is a form of a prefix tree. Each node have fixed number of children.
Level count is fixed. This allows to fast calculate in-node indices for by-index access,
without touching nodes, spending ~1 cycle per level.

Thou we have fixed number of children in node - we store only non-null children, using ["bit-block map" technique](#bit-block-map-technique) for access.
```
                Node     
          ┌─────────────┐
          │ 64bit mask  │  <- Mask of existent children. Act as sparse index array.
Level0    │ cap         │
          │ [*Node;cap] │  <- Dense array as FAM.
          └─────────────┘
                ...      
                         
               Node      
          ┌─────────────┐
          │ 64bit mask  │
LevelN    │ cap         │
          │ [usize;cap] │  <- Data indices.
          └─────────────┘
                         
                ┌   ┐    
Data        Vec │ T │    
                └   ┘    
```

Node is like a C99 object with [flexible array member (FAM)](https://en.wikipedia.org/wiki/Flexible_array_member).
Which means, that memory allocated only for existent children pointers.

Node bitmask have raised bits at children indices. Children are always stored ordered by 
their indices.

### "bit-block map" technique

Maps sparse index in bit-block to some data in dense array.


```
                  0 1       2 3          ◁═ popcnt before bit (dense_array index)
                                                                 
 bit_block      0 1 1 0 0 0 1 1 0 0 ...                          
              └───┬─┬───────┬─┬─────────┘                        
                  1 2       6 7          ◁═ bit index (sparse index)
               ┌──┘┌┘ ┌─────┘ │                                  
               │   │  │  ┌────┘                                  
               ▼   ▼  ▼  ▼                                       
dense_array    1, 32, 4, 5               len = bit_block popcnt  

1 => 1  (dense_array[0])
2 => 32 (dense_array[1])
6 => 4  (dense_array[2])
7 => 5  (dense_array[3])
```

Dense array elements must always have the same order as sparse array indices.

On x86 getting dense index costs 3-4 tacts with `bmi1`:
```rust
(bit_block & !(u64::MAX << index)).count_ones()
```
and just 2 with `bmi2`:
```rust
_bzhi_u64(bit_block, index).count_ones()
```

### Switching to uncompressed node

[Unimplemented]

Since children must always remain sorted, insert and remove technically O(N).
That's not a problem for most nodes, since they will have just a few children.
But for dense nodes - it is reasonably to switch to "uncompressed" data storage - 
where insert and remove O(1). 

To do this without introducing branching, we add another bitmask.
If children count less then some threshold (32) it is equal to the original one.
Otherwise - filled with ones. We will use it for [bit-block mapping](#bit-block-map-technique): 
below threshold it will work as usual, above - it's dense-index will equal sparse-index
(because bits before requested sparse index are all ones).

## Hierarchical bitmap

Bitmasks in nodes form hierarchical bitset/bitmap. Which is similar to [hi_sparse_bitset](https://crates.io/crates/hi_sparse_bitset), which in turn was derived from [hibitset](https://crates.io/crates/hibitset). This means, that all operations from hierarchical bitsets are possible, with the
same performance characteristics.

## Intersection

The bread and butter of hibit-tree is superfast intersection.
Intersected (or you may say - common) part of the tree computed on the fly, 
by simply AND-ing nodes bitmasks. Then resulted bitmask can be traversed. Nodes with indexes of 1 bits used to dive deeper, until the terminal node. Hence, tree branches 
that does not have common keys discarded early. The more trees you intersect at once -
the greater the technique benefit.

Alas, it is possible that only at terminal node we get empty bitmask. But even in this
case it is faster then other solutions, since we check multiple intersection possibilities
at once. In worst case, it degenerates to case of intersecting bitvecs without empty blocks. Which is quite fast by itself.

## Laziness

All inter-container operations return lazy trees, which can be used further in inter-tree 
operations. Lazy trees can be materialized to concrete container.

TODO: EXAMPLE

## Design choices

Tree have compile-time defined depth and width. This performs **significantly**
better, then traditional *dynamic* structure. And since we have integer as key,
we need only 8-11 levels depth max - any mem-overhead is neglectable due to tiny node sizes. 
But we assume, that most of the time user will work near u32 index space (4-6 levels).

### Dynamic tree

It is possible to have a dynamic-depth hibit-tree (with single-child nodes skipped from hierarchy).
But on shallow trees (~4 levels/32bit range) - fixed-depth tree outperforms significantly ALWAYS.
And we expect these shallow 32bit range trees to be used the most.

Plus, for dynamic tree to be beneficial, tree must be very sparsely populated across index range. 
Since as soon as there will be no single-childed nodes - dynamic tree
will have the same amount of nodes in depth as a fixed one.

## Uncompressed tree

Lib also provide version of tree with uncompressed nodes (with fixed sized array of children). It have higher memory overhead, but a little
bit faster since it don't need to do [bit-block mapping](#bit-block-map-technique). It still
have much lower memory footprint, then using plain Vec for sparse data, while being faster then HashMap.

You may need it, if your target platform have slow bit manipulation operations. Or memory overhead is 
not an issue - and you want inter-tree intersection as fast as possible.

It also can use SIMD-sized (and accelerated) bitblocks in nodes.