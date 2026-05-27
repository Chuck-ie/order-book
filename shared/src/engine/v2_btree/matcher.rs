use std::cmp::Reverse;

use crate::{
    common::{LimitOrderRequest, OrderBookExt, OrderIdU64, OrderMatcherExt, OrderSide},
    engine::v2_btree::book::OrderBook,
};

#[derive(Default)]
pub struct OrderMatcher {
    order_book: OrderBook,
}

impl OrderMatcherExt for OrderMatcher {
    type OrderId = OrderIdU64;
    type OrderBook = OrderBook;

    fn new() -> Self {
        Self {
            order_book: OrderBook::new(),
        }
    }

    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        self.order_book.place_order(request)
    }

    fn cancel_order(&mut self, order_id: Self::OrderId) {
        self.order_book.cancel_order(order_id);
    }

    fn process_limit_order(&mut self, mut request: LimitOrderRequest) -> LimitOrderRequest {
        let limit = request.limit;
        let mut remaining_amount = request.amount;
        let mut orders_to_remove = vec![];

        let side_iterator: Box<dyn Iterator<Item = (&u64, &mut Vec<OrderIdU64>)>> =
            match request.side {
                OrderSide::Bid => Box::new(self.order_book.asks.iter_mut()),
                OrderSide::Ask => Box::new(self.order_book.bids.iter_mut().map(|(r, v)| (&r.0, v))),
            };

        for (price, order_ids) in side_iterator {
            let price_matches = match request.side {
                OrderSide::Bid => *price <= limit,
                OrderSide::Ask => *price >= limit,
            };

            if !price_matches || remaining_amount == 0 {
                break;
            }

            for id in order_ids.iter() {
                let current_order = self.order_book.orders.get_mut(id).unwrap();
                let fill_amount = current_order.amount.min(remaining_amount);

                current_order.amount -= fill_amount;
                remaining_amount -= fill_amount;

                if current_order.amount == 0 {
                    orders_to_remove.push(*id);
                }

                if remaining_amount == 0 {
                    break;
                }
            }
        }

        for id in orders_to_remove {
            self.cancel_order(id);
        }

        request.amount = remaining_amount;
        request
    }

    fn best_bid(&self) -> Option<usize> {
        if let Some((price, _)) = self.order_book.bids.first_key_value() {
            Some(usize::try_from(price.0).expect("usize should be u64"))
        } else {
            None
        }
    }

    fn best_ask(&self) -> Option<usize> {
        if let Some((price, _)) = self.order_book.asks.first_key_value() {
            Some(usize::try_from(*price).expect("usize should be u64"))
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
