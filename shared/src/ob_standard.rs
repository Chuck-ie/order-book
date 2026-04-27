use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
};

use crate::{LimitOrder, LimitOrderRequest, OrderBookExt, OrderMatcherExt, OrderSide};

pub type OrderId = u64;

#[derive(Default)]
pub struct OrderBook {
    pub bids: BTreeMap<Reverse<u32>, Vec<OrderId>>,
    pub asks: BTreeMap<u32, Vec<OrderId>>,
    pub orders: HashMap<OrderId, LimitOrder<OrderId>>,
    next_order_id: OrderId,
}

impl OrderBook {
    const fn next_order_id(&mut self) -> OrderId {
        let id = self.next_order_id;
        self.next_order_id += 1;
        id
    }
}

impl OrderBookExt for OrderBook {
    type OrderId = OrderId;
    type Order = LimitOrder<Self::OrderId>;

    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
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

        match side {
            OrderSide::Bid => {
                self.bids.entry(Reverse(limit)).or_default().push(new_order_id);
            }
            OrderSide::Ask => {
                self.asks.entry(limit).or_default().push(new_order_id);
            }
        }

        self.orders.insert(new_order_id, new_order);
        new_order_id
    }

    fn cancel_order(&mut self, order_id: Self::OrderId) {
        let order = self.orders.get_mut(&order_id).expect("FIXME: missing order");
        let price = order.limit;
        let side = order.side;

        let order_ids = match side {
            OrderSide::Bid => {
                self.bids.get_mut(&Reverse(price)).expect("FIXME: missing price limit")
            }
            OrderSide::Ask => self.asks.get_mut(&price).expect("FIXME: missing price limit"),
        };

        let pos = order_ids.iter().position(|&id| id == order_id).expect("FIXME: missing order_id");
        order_ids.remove(pos);

        if order_ids.is_empty() {
            match side {
                OrderSide::Bid => self.bids.remove(&Reverse(price)),
                OrderSide::Ask => self.asks.remove(&price),
            };
        }

        self.orders.remove(&order_id);
    }

    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order> {
        self.orders.get(&order_id)
    }
}

#[derive(Default)]
pub struct OrderMatcher {
    pub order_book: OrderBook,
    pub queue: Vec<OrderId>,
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
        let limit = request.limit;
        let mut remaining_amount = request.amount;
        let mut orders_to_remove = vec![];

        let side_iterator: Box<dyn Iterator<Item = (&u32, &mut Vec<OrderId>)>> = match request.side
        {
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
            Some(price.0 as usize)
        } else {
            None
        }
    }

    fn best_ask(&self) -> Option<usize> {
        if let Some((price, _)) = self.order_book.asks.first_key_value() {
            Some(*price as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn total_volume_at(&self, side: OrderSide, price: usize) -> usize {
        let Some(order_ids) = (match side {
            OrderSide::Bid => self.order_book.bids.get(&Reverse(price as u32)),
            OrderSide::Ask => self.order_book.asks.get(&(price as u32)),
        }) else {
            return 0;
        };

        order_ids
            .iter()
            .map(|id| self.order_book.get_order(*id).expect("order not found").amount as usize)
            .sum()
    }

    fn order_book(&self) -> &Self::OrderBook {
        &self.order_book
    }
}
