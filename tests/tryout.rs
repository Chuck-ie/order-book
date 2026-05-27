#[cfg(test)]
mod tryout {
    use order_book::{
        common::{LimitOrder, OrderIdU32},
        slot_map::optimized::Slot,
    };

    #[test]
    pub fn tryout() {
        println!(
            "Slot<LimitOrder<OrderIdU64>>>: {}",
            std::mem::size_of::<Slot<LimitOrder<OrderIdU32>>>()
        );
    }
}
