**HI**erarchical **BIT**map **TREE**

Hierarchical bitmap tree is an integer-indexed prefix-tree without
memory overhead[^mem_overhead] and performance higher than a HashMap with no 
hasher[^hashmap_perf]. That also has unparalleled[^unparalleled_intersection] 
intersection performance, and other set-like operations between containers.

[^mem_overhead]: Tree nodes store child-pointers in dense. Null-pointers are not stored.
While still have O(1) access.

[^hashmap_perf]: HashMap<u32, T> with nohash-hasher and uniform index distribution -
ideal conditions for HashMap.

[^unique_ops]: Unique for tree/map containers.
