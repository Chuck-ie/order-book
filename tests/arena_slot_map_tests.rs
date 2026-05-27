#[cfg(test)]
mod tests {
    use order_book::{
        arena_allocator::{ArenaAllocator, ArenaId},
        slot_map::{
            NonMaxU32,
            chunked::{ArenaSlot, ChunkedSlotMap},
        },
    };

    fn get_test_arena(chunk_count: usize, chunk_size: usize) -> ArenaAllocator<ArenaSlot<u32>> {
        ArenaAllocator::new(chunk_count, chunk_size)
    }

    fn get_test_slot_map(arena: &mut ArenaAllocator<ArenaSlot<u32>>) -> ChunkedSlotMap<u32> {
        ChunkedSlotMap::from_arena(arena)
    }

    fn setup(
        chunk_count: usize,
        chunk_size: usize,
    ) -> (ArenaAllocator<ArenaSlot<u32>>, ChunkedSlotMap<u32>) {
        let mut arena = get_test_arena(chunk_count, chunk_size);
        let slot_map = get_test_slot_map(&mut arena);
        (arena, slot_map)
    }

    fn extract_occupied(
        index: &ArenaId,
        arena: &mut ArenaAllocator<ArenaSlot<u32>>,
    ) -> (u32, u32, NonMaxU32, NonMaxU32) {
        let (generation, data, prev, next) = unsafe {
            arena
                .get_unchecked(index.index as usize)
                .as_occupied_unchecked()
        };

        (*generation, *data, *prev, *next)
    }

    #[test]
    fn test_init() {
        let (_, slot_map) = setup(1, 1);

        assert!(slot_map.head.is_none());
        assert!(slot_map.tail.is_none());
        assert!(slot_map.free_head.is_none());
        assert_eq!(1, slot_map.owned_chunks.len());
        assert_eq!(1, slot_map.total_capacity);
        assert_eq!(0, slot_map.total_len);
        assert_eq!(0, slot_map.total_occupied);
    }

    #[test]
    fn test_insert_single() {
        let (mut arena, mut slot_map) = setup(1, 1);

        let val1_idx = slot_map.insert(1, &mut arena);
        assert_eq!(0, val1_idx.generation);
        assert_eq!(0, val1_idx.index);

        let (gen1, data, prev, next) = extract_occupied(&val1_idx, &mut arena);
        assert_eq!(0, gen1);
        assert_eq!(1, data);
        assert!(prev.is_none());
        assert!(next.is_none());

        assert!(slot_map.head.is_some());
        assert!(slot_map.tail.is_some());
        assert_eq!(slot_map.head, slot_map.tail);
        assert!(slot_map.free_head.is_none());

        assert_eq!(1, slot_map.total_capacity);
        assert_eq!(1, slot_map.total_len);
        assert_eq!(1, slot_map.total_occupied);
    }

    #[test]
    fn test_insert_multiple_single_chunk() {
        let (mut arena, mut slot_map) = setup(1, 4);

        let val1_idx = slot_map.insert(1, &mut arena);
        let val2_idx = slot_map.insert(2, &mut arena);
        let val3_idx = slot_map.insert(3, &mut arena);

        let (_, data1, prev1, next1) = extract_occupied(&val1_idx, &mut arena);
        assert_eq!(data1, 1);
        assert!(prev1.is_none());
        assert!(next1.is_some());

        let (_, data2, prev2, next2) = extract_occupied(&val2_idx, &mut arena);
        assert_eq!(data2, 2);
        assert!(prev2.is_some());
        assert!(next2.is_some());

        let (_, data3, prev3, next3) = extract_occupied(&val3_idx, &mut arena);
        assert_eq!(data3, 3);
        assert!(prev3.is_some());
        assert!(next3.is_none());

        assert_ne!(slot_map.head, slot_map.tail);
        assert_eq!(slot_map.head.0, val1_idx.index);
        assert_eq!(slot_map.tail.0, val3_idx.index);

        assert_eq!(4, slot_map.total_capacity);
        assert_eq!(3, slot_map.total_len);
        assert_eq!(3, slot_map.total_occupied);
    }

    #[test]
    fn test_insert_multiple_multiple_chunks() {
        let (mut arena, mut slot_map1) = setup(3, 2);
        let mut slot_map2 = ChunkedSlotMap::from_arena(&mut arena);

        let val1_idx = slot_map1.insert(1, &mut arena);
        let val2_idx = slot_map1.insert(2, &mut arena);
        let _val3_idx = slot_map2.insert(3, &mut arena);
        let val5_idx = slot_map1.insert(5, &mut arena);
        assert_eq!(4, val5_idx.index);

        let (_, data2, prev2, next2) = extract_occupied(&val2_idx, &mut arena);
        assert_eq!(data2, 2);
        assert!(prev2.is_some());
        assert!(next2.is_some());
        assert_eq!(next2.0, 4);

        let (_, data5, prev5, next5) = extract_occupied(&val5_idx, &mut arena);
        assert_eq!(data5, 5);
        assert!(prev5.is_some());
        assert!(next5.is_none());
        assert_eq!(prev5.0, 1);

        assert_ne!(slot_map1.head, slot_map1.tail);
        assert_eq!(slot_map1.head.0, val1_idx.index);
        assert_eq!(slot_map1.tail.0, val5_idx.index);

        assert_eq!(4, slot_map1.total_capacity);
        assert_eq!(3, slot_map1.total_len);
        assert_eq!(3, slot_map1.total_occupied);
    }

    #[test]
    fn test_insert_tombstone_middle() {
        let (mut arena, mut slot_map) = setup(1, 3);
        let _val1_idx = slot_map.insert(1, &mut arena);
        let val2_idx = slot_map.insert(2, &mut arena);
        let val3_idx = slot_map.insert(3, &mut arena);

        slot_map.remove(&val2_idx, &mut arena);
        let val4_idx = slot_map.insert(4, &mut arena);
        assert_eq!(val4_idx.index, val2_idx.index);
        assert_eq!(val4_idx.generation, val2_idx.generation + 1);

        let (_, _, _, next3) = extract_occupied(&val3_idx, &mut arena);
        assert!(next3.is_some());
        assert_eq!(next3.0, val4_idx.index);

        let (gen4, _, prev4, next4) = extract_occupied(&val4_idx, &mut arena);
        assert_eq!(gen4, val2_idx.generation + 1);
        assert!(prev4.is_some());
        assert!(next4.is_none());
        assert_eq!(prev4.0, val3_idx.index);

        assert_eq!(val4_idx.index, slot_map.tail.0);
        assert!(slot_map.free_head.is_none());
    }

    #[test]
    fn test_remove_head() {
        let (mut arena, mut slot_map) = setup(1, 3);
        let val1_idx = slot_map.insert(1, &mut arena);
        let val2_idx = slot_map.insert(2, &mut arena);
        let _val3_idx = slot_map.insert(3, &mut arena);

        slot_map.remove(&val1_idx, &mut arena);

        let (_, _, prev2, _) = extract_occupied(&val2_idx, &mut arena);
        assert!(prev2.is_none());
        assert_eq!(slot_map.head.0, val2_idx.index);
        assert!(slot_map.free_head.is_some());
        assert_eq!(val1_idx.index, slot_map.free_head.0);

        assert_eq!(3, slot_map.total_capacity);
        assert_eq!(3, slot_map.total_len);
        assert_eq!(2, slot_map.total_occupied);
    }

    #[test]
    fn test_remove_tail() {
        let (mut arena, mut slot_map) = setup(1, 3);
        let _val1_idx = slot_map.insert(1, &mut arena);
        let val2_idx = slot_map.insert(2, &mut arena);
        let val3_idx = slot_map.insert(3, &mut arena);

        slot_map.remove(&val3_idx, &mut arena);

        let (_, _, _, next2) = extract_occupied(&val2_idx, &mut arena);
        assert!(next2.is_none());
        assert_eq!(slot_map.tail.0, val2_idx.index);
        assert!(slot_map.free_head.is_some());
        assert_eq!(val3_idx.index, slot_map.free_head.0);

        assert_eq!(3, slot_map.total_capacity);
        assert_eq!(3, slot_map.total_len);
        assert_eq!(2, slot_map.total_occupied);
    }

    #[test]
    fn test_remove_middle() {
        let (mut arena, mut slot_map) = setup(1, 3);
        let val1_idx = slot_map.insert(1, &mut arena);
        let val2_idx = slot_map.insert(2, &mut arena);
        let val3_idx = slot_map.insert(3, &mut arena);

        slot_map.remove(&val2_idx, &mut arena);

        let (_, _, _, next1) = extract_occupied(&val1_idx, &mut arena);
        let (_, _, prev3, _) = extract_occupied(&val3_idx, &mut arena);
        assert!(next1.is_some());
        assert!(prev3.is_some());
        assert_eq!(next1.0, val3_idx.index);
        assert_eq!(prev3.0, val1_idx.index);
        assert!(slot_map.free_head.is_some());
        assert_eq!(val2_idx.index, slot_map.free_head.0);

        assert_eq!(3, slot_map.total_capacity);
        assert_eq!(3, slot_map.total_len);
        assert_eq!(2, slot_map.total_occupied);
    }

    #[test]
    fn test_prevent_double_free() {
        let (mut arena, mut slot_map) = setup(1, 3);
        let val1_idx = slot_map.insert(1, &mut arena);
        slot_map.remove(&val1_idx, &mut arena);
        let val2_idx = slot_map.insert(2, &mut arena);
        slot_map.remove(&val1_idx, &mut arena);

        let (gen2, data2, _, _) = extract_occupied(&val2_idx, &mut arena);
        assert_eq!(2, data2);
        assert_eq!(1, gen2);
    }

    #[test]
    fn test_order_book_price_level_cleared_and_recycled() {
        let (mut arena, mut slot_map1) = setup(2, 1);
        let val1_idx = slot_map1.insert(11, &mut arena);
        let val2_idx = slot_map1.insert(12, &mut arena);
        assert_eq!(0, arena.free_count());
        assert_eq!(2, slot_map1.owned_chunks.len());

        slot_map1.remove(&val1_idx, &mut arena);
        let is_empty = slot_map1.remove(&val2_idx, &mut arena);

        assert!(is_empty);

        for chunk_idx in slot_map1.owned_chunks.drain(..) {
            arena.release_chunk(chunk_idx);
        }

        assert_eq!(2, arena.free_count());
        assert_eq!(0, slot_map1.owned_chunks.len());

        let mut slot_map2 = ChunkedSlotMap::from_arena(&mut arena);
        let _val3_idx = slot_map2.insert(13, &mut arena);
        assert_eq!(1, arena.free_count());
        assert_eq!(1, slot_map2.owned_chunks.len());
    }
}
