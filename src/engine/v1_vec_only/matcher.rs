use crate::{
    common::{LimitOrderRequest, OrderBookExt, OrderIdU64, OrderMatcherExt, OrderSide},
    engine::v1_vec_only::book::OrderBook,
};

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
        let side = request.side;
        let limit = request.limit;

        let levels = match side {
            OrderSide::Bid => &mut self.order_book.asks,
            OrderSide::Ask => &mut self.order_book.bids,
        };

        let mut remaining_amount = request.amount;
        let mut orders_to_remove = vec![];

        for level in levels.iter_mut() {
            let price_matches = match side {
                OrderSide::Bid => level.price <= limit,
                OrderSide::Ask => level.price >= limit,
            };

            if !price_matches || remaining_amount == 0 {
                break;
            }

            for id in &level.order_ids {
                let current_order = self
                    .order_book
                    .orders
                    .get_mut(id)
                    .expect("FIXME: order_book");
                let deduction = current_order.amount.min(remaining_amount);

                current_order.amount -= deduction;
                remaining_amount -= deduction;

                if current_order.amount == 0 {
                    orders_to_remove.push(current_order.id);
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
        self.order_book
            .bids
            .first()
            .map(|level| usize::try_from(level.price).expect("usize should be u64"))
    }

    fn best_ask(&self) -> Option<usize> {
        self.order_book
            .asks
            .first()
            .map(|level| usize::try_from(level.price).expect("usize should be u64"))
    }

    fn total_volume_at(&self, side: OrderSide, price: usize) -> usize {
        let (levels, search_res) = match side {
            OrderSide::Bid => {
                let bids = &self.order_book.bids;
                let result = bids.binary_search_by(|level| {
                    price.cmp(&(usize::try_from(level.price).expect("usize should be u64")))
                });
                (bids, result)
            }
            OrderSide::Ask => {
                let asks = &self.order_book.asks;
                let result = asks.binary_search_by(|level| level.price.cmp(&(price as u64)));
                (asks, result)
            }
        };

        let Ok(level_idx) = search_res else {
            return 0;
        };

        let level = &levels[level_idx];
        level
            .order_ids
            .iter()
            .map(|id| {
                usize::try_from(
                    self.order_book
                        .get_order(*id)
                        .expect("order not found")
                        .amount,
                )
                .expect("usize should be u64")
            })
            .sum()
    }

    fn order_book(&self) -> &Self::OrderBook {
        &self.order_book
    }
}
