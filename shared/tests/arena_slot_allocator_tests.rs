#[cfg(test)]
mod arena_slot_allocator {
    use shared::final_ver::arena_slot_allocator::ArenaSlotAllocator;

    #[test]
    fn test_init() {
        let arena = ArenaSlotAllocator::<u32>::new(1, 2);

        assert_eq!(1, arena.chunk_count());
        assert_eq!(2, arena.chunk_size());
        assert_eq!(1, arena.free_count());
        assert_eq!(2, arena.slot_count());
    }

    #[test]
    #[should_panic(expected = "ArenaSlotAllocator overflowed")]
    fn test_claim() {
        let mut arena = ArenaSlotAllocator::<u32>::new(1, 2);

        assert_eq!(1, arena.free_count());
        let chunk_index = unsafe { arena.claim_chunk() };
        assert_eq!(0, chunk_index.0);
        assert_eq!(0, arena.free_count());

        // this overdraws the chunks, which will cause the debug assert in the claim_chunk function
        unsafe { arena.claim_chunk() };
    }

    #[test]
    fn test_release() {
        let mut arena = ArenaSlotAllocator::<u32>::new(1, 2);

        assert_eq!(1, arena.free_count());
        let chunk_index = unsafe { arena.claim_chunk() };
        arena.release_chunk(chunk_index);
        assert_eq!(1, arena.free_count());
    }

    /// A few AI generated tests as additions, though the arena also gets
    /// kind of tested when via the slot map implementation tests
    #[test]
    fn test_multi_chunk_claim_order() {
        let mut arena = ArenaSlotAllocator::<u32>::new(3, 2);
        assert_eq!(3, arena.free_count());

        let chunk1 = unsafe { arena.claim_chunk() };
        let chunk2 = unsafe { arena.claim_chunk() };
        let chunk3 = unsafe { arena.claim_chunk() };

        assert_eq!(0, arena.free_count());
        assert_ne!(chunk1.0, chunk2.0);
        assert_ne!(chunk2.0, chunk3.0);
    }

    #[test]
    fn test_fragmented_release_and_reclaim() {
        let mut arena = ArenaSlotAllocator::<u32>::new(3, 2);

        let _chunk0 = unsafe { arena.claim_chunk() };
        let chunk1 = unsafe { arena.claim_chunk() };
        let _chunk2 = unsafe { arena.claim_chunk() };

        let chunk1_val = chunk1.0;
        arena.release_chunk(chunk1);
        assert_eq!(1, arena.free_count());

        let recycled_chunk = unsafe { arena.claim_chunk() };
        assert_eq!(chunk1_val, recycled_chunk.0);
        assert_eq!(0, arena.free_count());
    }
}
