use std::{cmp::Reverse, collections::BTreeMap};

use crate::final_ver::{
    ArenaIndex,
    arena_slot_allocator::ArenaSlotAllocator,
    arena_slot_map::{ArenaSlot, ArenaSlotMap},
};

#[derive(Clone, Copy, Debug)]
pub enum OrderSide {
    Bid,
    Ask,
}

pub struct LimitOrder {
    pub id: ArenaIndex,
    pub side: OrderSide,
    pub limit: u64,
    pub amount: u64,
}

pub struct OrderBook {
    bids: BTreeMap<Reverse<u64>, ArenaSlotMap>,
    asks: BTreeMap<u64, ArenaSlotMap>,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBook {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            // 4096 price levels * 1024 orders per price level = 4_194_304 orders = 128 MB
            // arena: ArenaSlotAllocator::new(4096, 1024),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn place_order(&mut self, order: LimitOrder, arena: &mut ArenaSlotAllocator) -> ArenaIndex {
        let LimitOrder {
            id: _,
            side,
            limit: price,
            amount: _,
        } = order;

        let new_order_id = match side {
            OrderSide::Bid => self
                .bids
                .entry(Reverse(price))
                .or_insert_with(|| ArenaSlotMap::from_arena(arena))
                .insert(order, arena),
            OrderSide::Ask => self
                .asks
                .entry(price)
                .or_insert_with(|| ArenaSlotMap::from_arena(arena))
                .insert(order, arena),
        };

        unsafe {
            let (_, data, _, _) = arena
                .get_unchecked_mut(new_order_id.0 as usize)
                .as_occupied_unchecked_mut();

            data.id = new_order_id;
        }

        new_order_id
    }

    pub fn cancel_order(&mut self, order_index: &ArenaIndex, arena: &mut ArenaSlotAllocator) {
        let (price, side, id) = match arena.get(order_index.0 as usize) {
            Some(ArenaSlot::Occupied {
                generation: _,
                data,
                prev: _,
                next: _,
            }) => (data.limit, data.side, data.id),
            _ => return,
        };

        // TODO: consider if a macro can help with code duplication to avoid double match side
        // branching which probably has negligable impact on performance, but is basically free
        let level = match side {
            OrderSide::Bid => self.bids.get_mut(&Reverse(price)),
            OrderSide::Ask => self.asks.get_mut(&price),
        };

        let Some(level) = level else {
            unsafe { std::hint::unreachable_unchecked() }
        };

        // TODO: check if internal tracking of how many times an empty level was hit during matcher
        // iteration could possibly give a better as to when a level should be removed since level
        // creation has still some overhead. Worst case a level is created and deleted over and over
        // for a single order that keeps appearing instead of just caching the level until it was
        // iterated when it was empty say 10 times before final delition
        let level_is_empty = level.remove(&id, arena);

        if level_is_empty {
            for chunk_index in &level.owned_chunks {
                arena.release_chunk(chunk_index);
            }

            match side {
                OrderSide::Bid => self.bids.remove(&Reverse(price)),
                OrderSide::Ask => self.asks.remove(&price),
            };
        }
    }
}
