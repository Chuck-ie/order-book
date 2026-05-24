#[cfg(test)]
mod tests {
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

        println!("ArenaSlot: {}", std::mem::size_of::<ArenaSlot>());
    }
}
