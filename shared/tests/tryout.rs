#[cfg(test)]
mod tryout {

    use shared::{
        LimitOrder, final_ver::arena_slot_map::ArenaSlot, ob_arena_slot_map::ArenaId,
        slot_map::optimized::Slot,
    };

    #[test]
    pub fn random() {
        println!("Slot<u32>: {}", std::mem::size_of::<Slot<u32>>());
        println!("Slot<usize>: {}", std::mem::size_of::<Slot<usize>>());
        println!(
            "Slot<LimitOrder<u32>>: {}",
            std::mem::size_of::<Slot<LimitOrder<u32>>>()
        );

        println!(
            "Slot<LimitOrder<ArenaId>>: {}",
            std::mem::size_of::<Slot<LimitOrder<ArenaId>>>()
        );

        println!(
            "ArenaSlot<LimitOrder>-size: {}",
            std::mem::size_of::<ArenaSlot<shared::final_ver::order_book::LimitOrder>>()
        );
        println!(
            "ArenaSlot<LimitOrder>-align: {}",
            std::mem::align_of::<ArenaSlot<shared::final_ver::order_book::LimitOrder>>()
        );
    }
}
