use std::collections::HashMap;

use crate::common::OrderId;

#[derive(Default)]
pub struct OrderBook {
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub orders: HashMap<u64, LimitOrder<OrderId>>,
    next_order_id: OrderId,
}

pub struct PriceLevel {
    pub price: u64,
    pub order_ids: Vec<OrderId>,
}
