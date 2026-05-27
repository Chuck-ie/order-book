use crate::common::{LimitOrder, LimitOrderRequest, OrderBookExt, OrderIdU64, OrderSide};
use std::collections::HashMap;

#[derive(Default)]
pub struct OrderBook {
    pub(crate) bids: Vec<PriceLevel>,
    pub(crate) asks: Vec<PriceLevel>,
    pub(crate) orders: HashMap<OrderIdU64, LimitOrder<OrderIdU64>>,
    next_order_id: OrderIdU64,
}

impl OrderBook {
    const fn next_order_id(&mut self) -> OrderIdU64 {
        let id = self.next_order_id;
        self.next_order_id.0 += 1;
        id
    }

    fn find_level(
        &mut self,
        side: OrderSide,
        price: u64,
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
    type OrderId = OrderIdU64;
    type Order = LimitOrder<Self::OrderId>;

    fn new() -> Self {
        Self {
            bids: vec![],
            asks: vec![],
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
        let (levels, search_res) = self.find_level(side, limit);

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

        let (levels, search_res) = self.find_level(*side, *limit);
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

    fn capacity(&self) -> usize {
        self.orders.len()
    }
}

pub struct PriceLevel {
    pub price: u64,
    pub order_ids: Vec<OrderIdU64>,
}

impl PriceLevel {
    #[must_use]
    pub const fn empty(price: u64) -> Self {
        Self {
            price,
            order_ids: vec![],
        }
    }

    #[must_use]
    pub fn from_order(price: u64, order_id: OrderIdU64) -> Self {
        Self {
            price,
            order_ids: vec![order_id],
        }
    }
}
