# Order Book Matching Engine comparisons in Rust

## Overview
This project implements a total of four Orderbook engines in rust, each using different underlying data structures and making other optimizations. It then
benchmarks each engine and compares them in memory allocation, memory growth, plac order throughput, cancel order throughput and finally place order 
throughput over time. The following engines have been implemented:

1. EngineV1 (Vectors only)
2. EngineV2 (BTreeMap)
3. EngineV3 (Slotmap)
4. EngineV4 (Slotmap + Arena allocator)

### Results
1. Place order thoughput over time (higher is better)
![Place order throughput over time](images/place_order_throughput_persistent_scaling_narrow.png)

2. Place order throughput with M orders per N price levels (higher is better)
![Place order throughput with M orders per N price levels](images/place_order_throughput_level_scaling.png)

3. Cancel order throughput with M orders per N price levels (higher is better)
![Cancel order throughput with M orders per N price levels](images/cancel_order_throughput_level_scaling.png)

4. Memory allocations with M orders per N price levels (lower is better)
![Memory allocations with M orders per N price levels](images/memory_allocations.png)

5. Memory growth with M orders per N price levels (lower is better)
![Memory growth with M orders per N price levels](images/memory_growth.png)

### Engine evolution over time
EngineV1 is the simplest implementation with price levels stored in a sorted Vec, each holding its orders in arrival order. EngineV2 is nearly
identical, only swapping the Vec for a BTreeMap with the expectation that random access 


At first I started with the simplest engine of them all, EngineV1, just a version where all pricelevels are stored inside a sorted
Vector and each price level holding all orders for that price in the order they arrived. EngineV2 is almost identical, with the 
sole exception that price levels are stored inside a BTreeMap instead. Early on I expected this to help with random access e.g.
matching all orders starting at price level N, and early versions of my benchmarks would even point in that direction, but ultimately,
as the final benchmarks result show, V1 and V2 perform almost identical. Thinking about it, this makes a lot of sense. The V1 version
uses a binary search for price levels after all which is O(log n), which is the same as BTreeMap access, with the overhead that a BTreemap
uses much more heap allocationed memory, since every tree entry is essentially a Boxed type, that needs to be resolved by the cpu first,
causing random memory access.


### 1. EngineV1 (Vectors only)


### 2. EngineV2 (BTreeMap)

### 3. EngineV3 (Slotmap)

### 4. EngineV4 (Slotmap + Arena allocator)

<br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br><br>
## Performance 
### Results
### Benchmarking methodology
## Implementations
### 1. Naive
### 2. Standard
### 3. Slot map standard
### 4. Slot map optimized
## Key findings

// TODO: KEEP THAT I TRIED TO OPTIMIZED THE REMOVE FUNCTION TO JUST NOT CLEAN EMPTY LEVELS BECAUSE
// IT SEEMED LIKE IT WAS CAUSING SLOW DOWN BECAUSE DEALLOC AND ALLOC WAS TAKING QUITE SOME SPACE
// BUT KEEPING EMPTY PRICE LEVELS ACTUALLY DEGRADES PERFORMANCE BECAUSE I HAVE TO ITERATE EMPTY
// LEVELS INSIDE A BTREEMAP WHICH IS VERY SLOW (ALSO MENTION IT WAS BECAUSE OF REAL DATA BENCHES)

// MENTION THAT REVERSE KEY IS ACTUALLY FASTER BY ~7NS

// MENTION THAT prealloc for slot vecs doesnt help with performance

// MENTION THAT AoS was actually faster than SoA in my case because data is accessed randomly per
// slot, so fetching all data at ones was obviously better in hindsight

// https://data.lobsterdata.com/info/DataSamples.php
