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

#[derive(Default)]
pub struct OrderBook {
    pub bids: BTreeMap<Reverse<u32>, ArenaSlotMap<LimitOrder>>,
    pub asks: BTreeMap<u32, ArenaSlotMap<LimitOrder>>,
}

impl OrderBook {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn place_order(
        &mut self,
        order: LimitOrder,
        arena: &mut ArenaSlotAllocator<LimitOrder>,
    ) -> ArenaId {
        let LimitOrder {
            side,
            limit: price,
            amount: _,
        } = order;

        match side {
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
        }
    }

    // TODO: check if brancing can be avoid by using previous branch checks
    // of the matcher and then just adding place_bid and place_ask
    pub fn cancel_order(&mut self, order_id: &ArenaId, arena: &mut ArenaSlotAllocator<LimitOrder>) {
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
                let price_key = Reverse(price);
                let Some(level) = self.bids.get_mut(&price_key) else {
                    unsafe { std::hint::unreachable_unchecked() }
                };

                let level_is_empty = level.remove(order_id, arena);

                if level_is_empty {
                    for chunk_index in level.owned_chunks.drain(..) {
                        arena.release_chunk(chunk_index);
                    }

                    self.bids.remove(&price_key);
                }
            }
            OrderSide::Ask => {
                let Some(level) = self.asks.get_mut(&price) else {
                    unsafe { std::hint::unreachable_unchecked() }
                };

                let level_is_empty = level.remove(order_id, arena);

                if level_is_empty {
                    for chunk_index in level.owned_chunks.drain(..) {
                        arena.release_chunk(chunk_index);
                    }

                    self.asks.remove(&price);
                }
            }
        }
    }

    #[must_use]
    pub fn get_order<'a>(
        &self,
        arena_id: ArenaId,
        arena: &'a mut ArenaSlotAllocator<LimitOrder>,
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
