## Order Book Matching Engine comparisons in Rust

## Overview
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
