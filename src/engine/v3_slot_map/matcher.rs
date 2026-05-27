use std::cmp::Reverse;

use crate::{
    common::{LimitOrderRequest, OrderBookExt, OrderIdU32, OrderMatcherExt, OrderSide},
    engine::v3_slot_map::book::OrderBook,
    slot_map::SlotMap,
};

pub struct OrderMatcher {
    order_book: OrderBook,
    cancelation_buffer: Vec<OrderIdU32>,
}

impl Default for OrderMatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderMatcherExt for OrderMatcher {
    type OrderId = OrderIdU32;
    type OrderBook = OrderBook;

    fn new() -> Self {
        Self {
            order_book: OrderBook::new(),
            cancelation_buffer: Vec::with_capacity(1024),
        }
    }

    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        self.order_book.place_order(request)
    }

    fn cancel_order(&mut self, order_id: Self::OrderId) {
        self.order_book.cancel_order(order_id);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn process_limit_order(&mut self, mut request: LimitOrderRequest) -> LimitOrderRequest {
        let limit = request.limit;
        let mut remaining_amount = request.amount;

        macro_rules! execute_matching {
            ($iter:expr, $op:tt) => {
                for (price, order_ids) in $iter {
                    if !(*price $op limit) || remaining_amount == 0 {
                        break;
                    }

                    for id in &*order_ids {
                        let current_order = self.order_book.orders.get_occupied_unchecked_mut(id.0 as usize);
                        let fill_amount = current_order.amount.min(remaining_amount);

                        current_order.amount -= fill_amount;
                        remaining_amount -= fill_amount;

                        if current_order.amount == 0 {
                            self.cancelation_buffer.push(*id);
                        }

                        if remaining_amount == 0 {
                            break;
                        }
                    }
                }
            };
        }

        match request.side {
            OrderSide::Bid => {
                execute_matching!(self.order_book.asks.iter_mut(), <=);
            }
            OrderSide::Ask => {
                execute_matching!(self.order_book.bids.iter_mut().map(|(r, v)| (&r.0, v)), >=);
            }
        }

        for i in 0..self.cancelation_buffer.len() {
            self.cancel_order(self.cancelation_buffer[i]);
        }

        self.cancelation_buffer.clear();

        request.amount = remaining_amount;
        request
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_bid(&self) -> Option<usize> {
        if let Some((price, ids)) = self.order_book.bids.last_key_value()
            && ids.capacity() != 0
        {
            Some(price.0 as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_ask(&self) -> Option<usize> {
        if let Some((price, ids)) = self.order_book.asks.first_key_value()
            && ids.capacity() != 0
        {
            Some(*price as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn total_volume_at(&self, side: OrderSide, price: usize) -> usize {
        let Some(order_ids) = (match side {
            OrderSide::Bid => self.order_book.bids.get(&Reverse(price as u64)),
            OrderSide::Ask => self.order_book.asks.get(&(price as u64)),
        }) else {
            return 0;
        };

        order_ids
            .iter()
            .map(|id| {
                self.order_book
                    .get_order(*id)
                    .expect("order not found")
                    .amount as usize
            })
            .sum()
    }

    fn order_book(&self) -> &Self::OrderBook {
        &self.order_book
    }
}
