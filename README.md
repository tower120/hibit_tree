# **HI**erarchical **BIT**map **TREE**

Hierarchical bitmap tree is an integer-indexed prefix-tree without
memory overhead[^mem_overhead] and performance higher than a HashMap with no 
hasher[^hashmap_perf]. That also has unparalleled[^unparalleled_intersection] 
intersection performance, and other[^unique_ops] set-like operations between containers.

Most operations are O(1), there is no tree-balancing, or any hidden performance drag.

[^mem_overhead]: Tree nodes store child-pointers in dense. Null-pointers are not stored.
While still have O(1) access.

[^hashmap_perf]: HashMap<u32, T> with nohash-hasher and uniform index distribution -
ideal conditions for HashMap.

[^unparalleled_intersection]: Since the very tree is a form of hierarchical bitmap 
it naturally acts as intersection acceleration structure.

[^unique_ops]: Usually tree/map data structures does not provide such operations.

## How it works

## Intersection

The bread and butter of hibit-tree is intersection that does not require any kind of
scan operation. Intersected part of the tree computed on the fly, by simply AND-ing
nodes bitmasks. Then resulted bitmask can be traversed. Nodes with indexes of 1 bits used to dive deeper, until the terminal node.

Basically you get each intersected pair[^intersected] in O(1) time!

[^intersected]: Or iterator, if you intersect slice of trees.

## Design choices

Tree have compile-time defined depth and width. This performs **significantly**
better, then traditional *dynamic* structure. And since we have integer as key,
we need only 8-11 levels depth max - any mem-overhead is neglectable due to tiny node sizes. But we assume, that most of the time user will work near u32 index space (4 levels).

P.S. *Technically, it is possible to make true/dynamic prefix-tree with such technique. But use cases for it is not clear thou, since access-wise HashMap will probably perform better
in most cases. Looks like that the only viable candidate - is fast-intersection between string-key containers - but it looks too vague now.*