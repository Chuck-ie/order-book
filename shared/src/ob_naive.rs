use crate::{LimitOrder, LimitOrderRequest, OrderBookExt, OrderMatcherExt, OrderSide};
use std::collections::HashMap;

pub type OrderId = u64;

#[derive(Default)]
pub struct OrderBook {
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub orders: HashMap<u64, LimitOrder<OrderId>>,
    next_order_id: OrderId,
}

#[derive(Debug)]
pub struct PriceLevel {
    pub price: usize,
    pub order_ids: Vec<OrderId>,
}

pub struct OrderMatcher {
    pub order_book: OrderBook,
    pub queue: Vec<OrderId>,
}

impl OrderBook {
    const fn next_order_id(&mut self) -> OrderId {
        let id = self.next_order_id;
        self.next_order_id += 1;
        id
    }

    fn find_level(
        &mut self,
        side: OrderSide,
        price: usize,
    ) -> (&mut Vec<PriceLevel>, Result<usize, usize>) {
        match side {
            OrderSide::Bid => {
                let bids = &mut self.bids;
                let result = bids.binary_search_by(|level| price.cmp(&level.price));
                (bids, result)
            }
            OrderSide::Ask => {
                let asks = &mut self.asks;
                let result = asks.binary_search_by(|level| level.price.cmp(&price));
                (asks, result)
            }
        }
    }
}

impl OrderBookExt for OrderBook {
    type OrderId = OrderId;
    type Order = LimitOrder<Self::OrderId>;

    fn new() -> Self {
        Self {
            bids: vec![],
            asks: vec![],
            orders: HashMap::new(),
            next_order_id: 0,
        }
    }

    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        let LimitOrderRequest {
            side,
            limit,
            amount,
        } = request;

        let new_order_id = self.next_order_id();
        let new_order = LimitOrder::new(new_order_id, side, limit, amount);
        let (levels, search_res) = self.find_level(side, limit as usize);

        match search_res {
            Ok(i) => levels[i].order_ids.push(new_order_id),
            Err(i) => levels.insert(i, PriceLevel::from_order(limit, new_order_id)),
        }

        self.orders.insert(new_order_id, new_order);
        new_order_id
    }

    fn cancel_order(&mut self, order_id: Self::OrderId) {
        let Some(LimitOrder { side, limit, .. }) = self.orders.get(&order_id) else {
            return;
        };

        let (levels, search_res) = self.find_level(*side, *limit as usize);
        let Ok(i) = search_res else {
            return;
        };

        let level = &mut levels[i];

        let pos = level
            .order_ids
            .iter()
            .position(|&id| id == order_id)
            .expect("FIXME: order_book");
        level.order_ids.remove(pos);

        if level.order_ids.is_empty() {
            levels.remove(i);
        }

        self.orders.remove(&order_id);
    }

    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order> {
        self.orders.get(&order_id)
    }
}

impl PriceLevel {
    #[must_use]
    pub const fn empty(price: u32) -> Self {
        Self {
            price: price as usize,
            order_ids: vec![],
        }
    }

    #[must_use]
    pub fn from_order(price: u32, order_id: OrderId) -> Self {
        Self {
            price: price as usize,
            order_ids: vec![order_id],
        }
    }
}

impl OrderMatcherExt for OrderMatcher {
    type OrderId = OrderId;
    type OrderBook = OrderBook;

    fn new() -> Self {
        Self {
            order_book: OrderBook::new(),
            queue: vec![],
        }
    }

    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        let new_order_id = self.order_book.place_order(request);
        self.queue.push(new_order_id);

        new_order_id
    }

    fn cancel_order(&mut self, order_id: Self::OrderId) {
        let pos = self
            .queue
            .iter()
            .position(|id| id == &order_id)
            .expect("FIXME: order not found in queue");

        self.queue.remove(pos);
        self.order_book.cancel_order(order_id);
    }

    fn process_limit_order(&mut self, mut request: LimitOrderRequest) -> LimitOrderRequest {
        let side = request.side;
        let limit = request.limit as usize;

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
        self.order_book.bids.first().map(|level| level.price)
    }

    fn best_ask(&self) -> Option<usize> {
        self.order_book.asks.first().map(|level| level.price)
    }

    fn total_volume_at(&self, side: OrderSide, price: usize) -> usize {
        let (levels, search_res) = match side {
            OrderSide::Bid => {
                let bids = &self.order_book.bids;
                let result = bids.binary_search_by(|level| price.cmp(&level.price));
                (bids, result)
            }
            OrderSide::Ask => {
                let asks = &self.order_book.asks;
                let result = asks.binary_search_by(|level| level.price.cmp(&price));
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
