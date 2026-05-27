use std::{
    cmp::Reverse,
    collections::{BTreeMap, HashMap},
};

use crate::common::{LimitOrder, LimitOrderRequest, OrderBookExt, OrderIdU64, OrderSide};

#[derive(Default)]
pub struct OrderBook {
    pub(crate) bids: BTreeMap<Reverse<u64>, Vec<OrderIdU64>>,
    pub(crate) asks: BTreeMap<u64, Vec<OrderIdU64>>,
    pub(crate) orders: HashMap<OrderIdU64, LimitOrder<OrderIdU64>>,
    next_order_id: OrderIdU64,
}

impl OrderBook {
    const fn next_order_id(&mut self) -> OrderIdU64 {
        let id = self.next_order_id;
        self.next_order_id.0 += 1;
        id
    }
}

impl OrderBookExt for OrderBook {
    type OrderId = OrderIdU64;
    type Order = LimitOrder<Self::OrderId>;

    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: HashMap::new(),
            next_order_id: OrderIdU64::default(),
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
                self.bids
                    .entry(Reverse(limit))
                    .or_default()
                    .push(new_order_id);
            }
            OrderSide::Ask => {
                self.asks.entry(limit).or_default().push(new_order_id);
            }
        }

        self.orders.insert(new_order_id, new_order);
        new_order_id
    }

    fn cancel_order(&mut self, order_id: Self::OrderId) {
        let order = self
            .orders
            .get_mut(&order_id)
            .expect("FIXME: missing order");
        let price = order.limit;
        let side = order.side;

        let order_ids = match side {
            OrderSide::Bid => self
                .bids
                .get_mut(&Reverse(price))
                .expect("FIXME: missing price limit"),
            OrderSide::Ask => self
                .asks
                .get_mut(&price)
                .expect("FIXME: missing price limit"),
        };

        let pos = order_ids
            .iter()
            .position(|&id| id == order_id)
            .expect("FIXME: missing order_id");
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

    fn capacity(&self) -> usize {
        self.orders.len()
    }
}
