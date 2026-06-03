use ahash::AHashMap;

use crate::{
    arena_allocator::{ArenaAllocator, ArenaId},
    common::OrderSide,
    engine::LimitOrder,
    slot_map::chunked::{ArenaSlot, ChunkedSlotMap},
};

#[derive(Default)]
pub struct OrderBook {
    pub bids: AHashMap<u32, ChunkedSlotMap<LimitOrder>>,
    pub asks: AHashMap<u32, ChunkedSlotMap<LimitOrder>>,
    pub bid_prices: Vec<u32>,
    pub ask_prices: Vec<u32>,
}

impl OrderBook {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bids: AHashMap::new(),
            asks: AHashMap::new(),
            bid_prices: vec![],
            ask_prices: vec![],
        }
    }

    #[must_use]
    pub fn place_order(
        &mut self,
        order: LimitOrder,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) -> ArenaId {
        let LimitOrder {
            side,
            limit: price,
            amount: _,
        } = order;

        match side {
            OrderSide::Bid => {
                if let Some(level) = self.bids.get_mut(&price) {
                    level.insert(order, arena)
                } else {
                    let Err(i) = self.bid_prices.binary_search(&price) else {
                        // Safety: we just checked and know that the level/index wont exist
                        unsafe { std::hint::unreachable_unchecked() }
                    };

                    let mut new_level = ChunkedSlotMap::from_arena(arena);
                    let order_id = new_level.insert(order, arena);

                    self.bid_prices.insert(i, price);
                    self.bids.insert(price, new_level);
                    order_id
                }
            }
            OrderSide::Ask => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.insert(order, arena)
                } else {
                    let Err(i) = self.ask_prices.binary_search(&price) else {
                        // Safety: we just checked and know that the level/index wont exist
                        unsafe { std::hint::unreachable_unchecked() }
                    };

                    let mut new_level = ChunkedSlotMap::from_arena(arena);
                    let order_id = new_level.insert(order, arena);

                    self.ask_prices.insert(i, price);
                    self.asks.insert(price, new_level);
                    order_id
                }
            }
        }
    }

    // TODO: check if brancing can be avoid by using previous branch checks
    // of the matcher and then just adding place_bid and place_ask
    pub fn cancel_order(
        &mut self,
        order_id: &ArenaId,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) {
        let (price, side) = match arena.get(order_id.index as usize) {
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
                let Some(level) = self.bids.get_mut(&price) else {
                    // Safety: we checked the arena if the order exist, so the level must exist too
                    unsafe { std::hint::unreachable_unchecked() }
                };

                let level_is_empty = level.remove(order_id, arena);

                if level_is_empty {
                    for chunk_index in level.owned_chunks.drain(..) {
                        arena.release_chunk(chunk_index);
                    }

                    self.bids.remove(&price);

                    if let Ok(idx) = self.bid_prices.binary_search(&price) {
                        self.bid_prices.remove(idx);
                    }
                }
            }
            OrderSide::Ask => {
                let Some(level) = self.asks.get_mut(&price) else {
                    // Safety: we checked the arena if the order exist, so the level must exist too
                    unsafe { std::hint::unreachable_unchecked() }
                };

                let level_is_empty = level.remove(order_id, arena);

                if level_is_empty {
                    for chunk_index in level.owned_chunks.drain(..) {
                        arena.release_chunk(chunk_index);
                    }

                    self.asks.remove(&price);

                    if let Ok(idx) = self.ask_prices.binary_search(&price) {
                        self.ask_prices.remove(idx);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn get_order<'a>(
        &self,
        arena_id: &ArenaId,
        arena: &'a mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) -> Option<&'a LimitOrder> {
        if let ArenaSlot::Occupied {
            data,
            generation: _,
            prev: _,
            next: _,
        } = unsafe { arena.get_unchecked(arena_id.index as usize) }
        {
            Some(data)
        } else {
            None
        }
    }
}
