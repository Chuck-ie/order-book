#[cfg(test)]
mod tests {
    use std::{marker::PhantomData, num::NonZeroU32};

    pub enum TestSlot<T> {
        Free { next_free: u32 },
        Occupied { data: T, prev: u32, next: u32 },
    }

    pub struct Order {
        pub side: OrderSide,
        pub price: u128,
        pub qty: u128,
        pub level_slot_idx: u32,
    }

    #[derive(Clone, Copy)]
    pub enum OrderSide {
        Bid,
        Ask,
    }

    #[test]
    fn random() {
        println!("TestSlot<u32>: {}", std::mem::size_of::<TestSlot<u32>>());
        println!(
            "TestSlot<Order>: {}",
            std::mem::size_of::<TestSlot<Order>>()
        );
    }
}
