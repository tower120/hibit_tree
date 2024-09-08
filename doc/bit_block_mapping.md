# Bit-block mapping

"Bit-block mapping" technique allows to use 64bit bit-block as
a sparse array (where each raised bit index is a sparse index),
and map each sparse index to some dense index.

Dense index = raised bit count before bit at sparse index.

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

Dense array elements must always have the same order as a sparse array indices.

On x86 getting dense index costs 3-4 tacts with `bmi1`:
```rust
(bit_block & !(u64::MAX << index)).count_ones()
```
and just 2 with `bmi2`:
```rust
_bzhi_u64(bit_block, index).count_ones()
```

## Bypass property

If bit-block completely filled[^filled], dense index = sparse index. 

*This can be useful for switching from dense to sparse array, without introducing branching.*

[^filled]: Filled = all bits raised.