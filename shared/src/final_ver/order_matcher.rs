use std::cmp::Reverse;

use crate::{
    OrderSide,
    final_ver::{
        arena_slot_allocator::{ArenaId, ArenaSlotAllocator},
        order_book::{LimitOrder, OrderBook},
    },
};

pub enum MatcherCommand {
    PlaceOrder(LimitOrder),
    CancelOrder(ArenaId),
}

impl MatcherCommand {
    #[must_use]
    #[inline(always)]
    #[allow(clippy::inline_always)]
    // pub const fn new_limit_order(side: OrderSide, limit: u64, amount: u64) -> Self {
    pub const fn new_limit_order(side: OrderSide, limit: u32, amount: u32) -> Self {
        Self::PlaceOrder(LimitOrder {
            limit,
            amount,
            side,
        })
    }
}

pub struct OrderMatcher {
    pub order_book: OrderBook,
    cancelation_buffer: Vec<ArenaId>,
}

impl Default for OrderMatcher {
    fn default() -> Self {
        Self {
            order_book: OrderBook::default(),
            cancelation_buffer: Vec::with_capacity(1024),
        }
    }
}

impl OrderMatcher {
    #[must_use]
    pub fn from_arena(arena: ArenaSlotAllocator<LimitOrder>) -> Self {
        Self {
            order_book: OrderBook::from_arena(arena),
            cancelation_buffer: Vec::with_capacity(1024),
        }
    }

    #[must_use]
    pub fn new(chunk_count: usize, chunk_size: usize) -> Self {
        Self {
            order_book: OrderBook::new(chunk_count, chunk_size),
            cancelation_buffer: Vec::with_capacity(1024),
        }
    }

    pub fn process(&mut self, command: MatcherCommand) -> Option<ArenaId> {
        match command {
            MatcherCommand::PlaceOrder(mut order) => {
                match order.side {
                    OrderSide::Bid => self.process_bid(&mut order),
                    OrderSide::Ask => self.process_ask(&mut order),
                }

                if order.amount > 0 {
                    Some(self.order_book.place_order(order))
                } else {
                    None
                }
            }
            MatcherCommand::CancelOrder(order_id) => {
                self.order_book.cancel_order(&order_id);
                None
            }
        }
    }

    fn process_bid(&mut self, order: &mut LimitOrder) {
        let mut remaining_amount = order.amount;

        for (price, level) in &mut self.order_book.asks {
            if (*price > order.limit) || remaining_amount == 0 {
                break;
            }

            let mut iter = level.walk(&mut self.order_book.arena);

            while remaining_amount > 0 {
                let Some((arena_id, current_order)) = iter.next_pair() else {
                    break;
                };

                let fill_amount = current_order.amount.min(remaining_amount);
                current_order.amount -= fill_amount;
                remaining_amount -= fill_amount;

                if current_order.amount == 0 {
                    self.cancelation_buffer.push(arena_id);
                }
            }
        }

        order.amount = remaining_amount;
        self.clean_up();
    }

    fn process_ask(&mut self, order: &mut LimitOrder) {
        let mut remaining_amount = order.amount;

        for (price, level) in &mut self.order_book.bids.iter_mut().map(|(r, v)| (&r.0, v)) {
            if (*price < order.limit) || remaining_amount == 0 {
                break;
            }

            let mut iter = level.walk(&mut self.order_book.arena);

            while remaining_amount > 0 {
                let Some((arena_id, current_order)) = iter.next_pair() else {
                    break;
                };

                let fill_amount = current_order.amount.min(remaining_amount);
                current_order.amount -= fill_amount;
                remaining_amount -= fill_amount;

                if current_order.amount == 0 {
                    self.cancelation_buffer.push(arena_id);
                }
            }
        }

        order.amount = remaining_amount;
        self.clean_up();
    }

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn clean_up(&mut self) {
        for arena_id in self.cancelation_buffer.drain(..) {
            self.order_book.cancel_order(&arena_id);
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn best_bid(&self) -> Option<usize> {
        if let Some((price, level)) = self.order_book.bids.last_key_value()
            && level.total_occupied > 0
        {
            Some(price.0 as usize)
        } else {
            None
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn best_ask(&self) -> Option<usize> {
        if let Some((price, level)) = self.order_book.asks.first_key_value()
            && level.total_occupied > 0
        {
            Some(*price as usize)
        } else {
            None
        }
    }

    #[must_use]
    // pub fn total_volume_at(&mut self, side: OrderSide, price: u64) -> u64 {
    pub fn total_volume_at(&mut self, side: OrderSide, price: u32) -> u32 {
        let Some(level) = (match side {
            OrderSide::Bid => self.order_book.bids.get_mut(&Reverse(price)),
            OrderSide::Ask => self.order_book.asks.get_mut(&(price)),
        }) else {
            return 0;
        };

        let mut iter = level.walk(&mut self.order_book.arena);

        let mut total_volume = 0;

        while let Some((_, order)) = iter.next_pair() {
            total_volume += order.amount;
        }

        total_volume
    }
}
