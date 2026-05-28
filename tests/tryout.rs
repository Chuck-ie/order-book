#[cfg(test)]
mod tryout {
    use order_book::{engine::v4_slot_map_arena::LimitOrder, slot_map::chunked::ArenaSlot};

    #[test]
    pub fn tryout() {
        println!(
            "<ArenaSlot<LimitOrder>>: {}",
            std::mem::size_of::<ArenaSlot<LimitOrder>>()
        );
    }
}
