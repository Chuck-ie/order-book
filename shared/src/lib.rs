use std::fmt::Debug;

use serde::Deserialize;

pub mod common;
pub mod engine;
pub mod final_ver;
pub mod ob_arena_slot_map;
pub mod ob_naive;
pub mod ob_slot_map_optimized;
pub mod ob_slot_map_standard;
pub mod ob_standard;
pub mod slot_map;

pub trait OrderBookExt {
    type OrderId;
    type Order;

    fn new() -> Self;
    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId;
    fn cancel_order(&mut self, order_id: Self::OrderId);
    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order>;
    fn capacity(&self) -> usize;
}

pub trait OrderMatcherExt {
    type OrderId: Copy;
    type OrderBook: OrderBookExt<OrderId = Self::OrderId>;

    fn new() -> Self;

    fn process(&mut self, command: MatcherCommand<Self::OrderId>) -> Option<Self::OrderId> {
        match command {
            MatcherCommand::PlaceOrder(order) => {
                let order = self.process_limit_order(order);

                if order.amount > 0 {
                    Some(self.place_order(order))
                } else {
                    None
                }
            }
            MatcherCommand::CancelOrder(id) => {
                self.cancel_order(id);
                None
            }
        }
    }

    #[doc(hidden)]
    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId;

    #[doc(hidden)]
    fn cancel_order(&mut self, order_id: Self::OrderId);

    #[doc(hidden)]
    fn process_limit_order(&mut self, request: LimitOrderRequest) -> LimitOrderRequest;

    // testing helper functions
    fn best_bid(&self) -> Option<usize>;
    fn best_ask(&self) -> Option<usize>;
    fn total_volume_at(&self, side: OrderSide, price: usize) -> usize;
    fn order_book(&self) -> &Self::OrderBook;
}

#[derive(Debug)]
pub struct LimitOrderRequest {
    pub side: OrderSide,
    pub limit: u64,
    pub amount: u64,
}
