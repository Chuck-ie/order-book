use std::cmp::Reverse;

use crate::{
    arena_allocator::{ArenaAllocator, ArenaId},
    common::{MatcherCommand, OrderSide},
    engine::{LimitOrder, arena_order_matcher::ArenaOrderMatcherExt, v4_sm_arena::book::OrderBook},
    slot_map::chunked::ArenaSlot,
};

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

impl ArenaOrderMatcherExt for OrderMatcher {
    fn new() -> Self {
        Self {
            order_book: OrderBook::new(),
            cancelation_buffer: Vec::with_capacity(1024),
        }
    }

    // helper so that perf report can always see this function
    // #[inline(never)]
    fn process(
        &mut self,
        command: MatcherCommand<LimitOrder, ArenaId>,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) -> Option<ArenaId> {
        match command {
            MatcherCommand::PlaceOrder(mut order) => {
                match order.side {
                    OrderSide::Bid => self.process_bid(&mut order, arena),
                    OrderSide::Ask => self.process_ask(&mut order, arena),
                }

                // TODO: could be duplicated and moved up into the match to skip the place_order
                // side branch by directly calling place_bid/place_ask
                if order.amount > 0 {
                    Some(self.order_book.place_order(order, arena))
                } else {
                    None
                }
            }
            MatcherCommand::CancelOrder(order_id) => {
                self.order_book.cancel_order(&order_id, arena);
                None
            }
        }
    }

    fn process_bid(
        &mut self,
        order: &mut LimitOrder,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) {
        let mut remaining_amount = order.amount;

        for (price, level) in &mut self.order_book.asks {
            if (*price > order.limit) || remaining_amount == 0 {
                break;
            }

            let mut iter = level.iter(arena);

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

        for arena_id in self.cancelation_buffer.drain(..) {
            self.order_book.cancel_order(&arena_id, arena);
        }
    }

    fn process_ask(
        &mut self,
        order: &mut LimitOrder,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) {
        let mut remaining_amount = order.amount;

        for (price, level) in &mut self.order_book.bids.iter_mut().map(|(r, v)| (&r.0, v)) {
            if (*price < order.limit) || remaining_amount == 0 {
                break;
            }

            let mut iter = level.iter(arena);

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

        for arena_id in self.cancelation_buffer.drain(..) {
            self.order_book.cancel_order(&arena_id, arena);
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_bid(&self) -> Option<usize> {
        if let Some((price, level)) = self.order_book.bids.last_key_value()
            && level.total_occupied > 0
        {
            Some(price.0 as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_ask(&self) -> Option<usize> {
        if let Some((price, level)) = self.order_book.asks.first_key_value()
            && level.total_occupied > 0
        {
            Some(*price as usize)
        } else {
            None
        }
    }

    fn total_volume_at(
        &mut self,
        side: OrderSide,
        price: u32,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) -> u32 {
        let Some(level) = (match side {
            OrderSide::Bid => self.order_book.bids.get_mut(&Reverse(price)),
            OrderSide::Ask => self.order_book.asks.get_mut(&(price)),
        }) else {
            return 0;
        };

        let mut iter = level.iter(arena);

        let mut total_volume = 0;

        while let Some((_, order)) = iter.next_pair() {
            total_volume += order.amount;
        }

        total_volume
    }
}
