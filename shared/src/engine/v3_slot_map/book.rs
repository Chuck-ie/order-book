use std::{cmp::Reverse, collections::BTreeMap};

use crate::{
    common::{LimitOrder, LimitOrderRequest, OrderBookExt, OrderIdU32, OrderSide},
    slot_map::{SlotMap, optimized::SlotMapOptimized},
};

pub struct OrderBook {
    pub(crate) bids: BTreeMap<Reverse<u64>, SlotMapOptimized<OrderIdU32>>,
    pub(crate) asks: BTreeMap<u64, SlotMapOptimized<OrderIdU32>>,
    pub(crate) orders: SlotMapOptimized<LimitOrder<OrderIdU32>>,
}

impl OrderBookExt for OrderBook {
    type OrderId = OrderIdU32;
    type Order = LimitOrder<Self::OrderId>;

    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: SlotMapOptimized::new(),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        let LimitOrderRequest {
            side,
            limit: price,
            amount,
        } = request;
        let new_order = LimitOrder::new(OrderIdU32::default(), side, price, amount);
        let new_order_id = self.orders.insert(new_order);
        let level_idx = match side {
            OrderSide::Bid => self
                .bids
                .entry(Reverse(price))
                .or_default()
                .insert(new_order_id),
            OrderSide::Ask => self.asks.entry(price).or_default().insert(new_order_id),
        };

        self.orders
            .get_occupied_unchecked_mut(new_order_id.0 as usize)
            .id = OrderIdU32(level_idx.0);

        new_order_id
    }

    #[allow(clippy::cast_possible_truncation)]
    fn cancel_order(&mut self, order_id: Self::OrderId) {
        let (price, side, internal_id) = match self.orders.get(order_id.0 as usize) {
            Some(order) => (order.limit, order.side, order.id),
            None => return,
        };

        let level_is_empty = {
            let level = match side {
                OrderSide::Bid => self
                    .bids
                    .get_mut(&Reverse(price))
                    .expect("missing price level"),
                OrderSide::Ask => self.asks.get_mut(&price).expect("missing price level"),
            };

            level.remove(internal_id);
            level.is_empty()
        };

        if level_is_empty {
            match side {
                OrderSide::Bid => self.bids.remove(&Reverse(price)),
                OrderSide::Ask => self.asks.remove(&price),
            };
        }

        self.orders.remove(order_id);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order> {
        self.orders.get(order_id.0 as usize)
    }

    fn capacity(&self) -> usize {
        self.orders.capacity()
    }
}
