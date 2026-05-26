use std::{cmp::Reverse, collections::BTreeMap};

use crate::{
    OrderSide,
    final_ver::{
        ArenaId,
        arena_slot_allocator::ArenaSlotAllocator,
        arena_slot_map::{ArenaSlot, ArenaSlotMap},
    },
};

#[derive(Debug, PartialEq, Eq)]
pub struct LimitOrder {
    pub limit: u32,
    pub amount: u32,
    pub side: OrderSide,
}

pub struct OrderBook {
    pub arena: ArenaSlotAllocator<LimitOrder>,
    // pub level_arena:
    pub bids: BTreeMap<Reverse<u32>, ArenaSlotMap<LimitOrder>>,
    pub asks: BTreeMap<u32, ArenaSlotMap<LimitOrder>>,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self {
            arena: ArenaSlotAllocator::new(128, 1024),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }
}

impl OrderBook {
    #[must_use]
    pub const fn from_arena(arena: ArenaSlotAllocator<LimitOrder>) -> Self {
        Self {
            arena,
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn new(chunk_count: usize, chunk_size: usize) -> Self {
        Self {
            arena: ArenaSlotAllocator::new(chunk_count, chunk_size),
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn place_order(&mut self, order: LimitOrder) -> ArenaId {
        let LimitOrder {
            side,
            limit: price,
            amount: _,
        } = order;

        match side {
            OrderSide::Bid => self
                .bids
                .entry(Reverse(price))
                .or_insert_with(|| ArenaSlotMap::from_arena(&mut self.arena))
                .insert(order, &mut self.arena),
            OrderSide::Ask => self
                .asks
                .entry(price)
                .or_insert_with(|| ArenaSlotMap::from_arena(&mut self.arena))
                .insert(order, &mut self.arena),
        }
    }

    // TODO: check if brancing can be avoid by using previous branch checks
    // of the matcher and then just adding place_bid and place_ask
    pub fn cancel_order(&mut self, order_id: &ArenaId) {
        let (price, side) = match self.arena.get(order_id.index as usize) {
            Some(ArenaSlot::Occupied {
                generation: _,
                data,
                prev: _,
                next: _,
            }) => (data.limit, data.side),
            _ => return,
        };

        match side {
            OrderSide::Bid => {
                let price_key = Reverse(price);
                let Some(level) = self.bids.get_mut(&price_key) else {
                    unsafe { std::hint::unreachable_unchecked() }
                };

                let level_is_empty = level.remove(order_id, &mut self.arena);

                if level_is_empty {
                    for chunk_index in level.owned_chunks.drain(..) {
                        self.arena.release_chunk(chunk_index);
                    }

                    self.bids.remove(&price_key);
                }
            }
            OrderSide::Ask => {
                let Some(level) = self.asks.get_mut(&price) else {
                    unsafe { std::hint::unreachable_unchecked() }
                };

                let level_is_empty = level.remove(order_id, &mut self.arena);

                if level_is_empty {
                    for chunk_index in level.owned_chunks.drain(..) {
                        self.arena.release_chunk(chunk_index);
                    }

                    self.asks.remove(&price);
                }
            }
        }
    }

    #[must_use]
    pub fn get_order(&self, arena_id: ArenaId) -> Option<&LimitOrder> {
        if let ArenaSlot::Occupied {
            data,
            generation: _,
            prev: _,
            next: _,
        } = unsafe { self.arena.get_unchecked(arena_id.index as usize) }
        {
            Some(data)
        } else {
            None
        }
    }
}
