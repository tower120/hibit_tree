# Node compression

Compressed tree use node compression technique based on ["bit-block mapping"](bit_block_mapping.md).

## Abstract

In prefix tree all nodes have exactly N children. Thou most of these children are null/empty. 

The naive *(and probably the fastest possible)* way to store and access children - is to store them 
as N array. Let's call such node **uncompressed**.
Downside of **uncompressed** node is that it consumes significant amount of memory, 
occupied by null pointers[^null-pointer].

[^null-pointer]: Or pointers to some empty node.

Alternative to that is storing only non-null pointers, and some means of access. Let's call such nodes **compressed**.
The most naive **compressed** node would contain only array of non-null pointers WITH indices. 
The child access will be O(log(N)), if store them sorted and use binary search, or O(N) - if store them unordered.
ART(Adaptive Radix Tree) for example, store separately indices and child-pointers, to speed-up access
by searching in more contiguous space. But it is still O(log(N)).

Our node compression method have FAST O(1) access.

## Node compression

We work with 64-child nodes.
Each node contains 64bit bitmask and array with non-null child pointers.
In bitmask each raised bit corresponds to non-null child at the index of bit:
```rust
struct Node{
    bitmask: u64
    len: u8
    children: [*Node; len]   // à la C99 Flexible Array Member
}
```


```
  bitmask      0 1 1 0 0 0 1 1 0 0 ...    
(as sparse)  └───┬─┬───────┬─┬─────────┘  
                ┌┘ └───┐   │ └───────┐    
                │      │   └──┐      │    
           ┌    ▼      ▼      ▼      ▼   ┐
 children  │ *Node1 *Node2 *Node6 *Node7 │
(as dense) └                             ┘
```
Which equals to uncompressed version:
```
[ null, *Node1, *Node2, null, null, null, *Node6, *Node7, ... ]
```

We map from sparse (`bitmask`) to dense(`children`) array with ["bit-block mapping"](bit_block_mapping.md).

Bit-block mapping requires all children to be in order, so insert and remove is O(N).
But! There is a way to seamlessly switch to uncompressed array above certain threshold!

## Switching to uncompressed

> NOTE: *This part is not strictly necessary and can be omitted. It just amortize insert/remove cost for densely populated nodes.*

Using bit-block mapping [bypass property](bit_block_mapping.md#bypass-property), we can
switch to uncompressed array without any branching on access. 

If we exceed some child count threshold, we could switch to Node with `bitmask = u64::MAX` and `len = 64`. Using the same technique for access as before.

The only thing - is that for `contains` check we'll need to branch between taking bit from bitmask for compressed mode and taking children pointer from array for uncompressed
mode. 

To avoid this, we add another bitmask - `active_bitmask`, which is copy of bitmask
in compressed mode and u64::MAX in uncompressed. Access operations now use `active_bitmask`, and `contains` operation use `bitmask`.

```rust
struct Node{
    /// u64::MAX if in uncompressed mode, bitmask otherwise
    active_bitmask: u64  

    bitmask: u64
    len: u8
    children: [*Node; len]   // à la C99 Flexible Array Member
}
```

Now on `insert` we can switch to uncompressed mode, and have insert/remove at O(1)!