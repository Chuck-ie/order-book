use crate::common::OrderSide;

pub mod book;
pub mod matcher;

#[derive(Debug, PartialEq, Eq)]
pub struct LimitOrder {
    pub limit: u32,
    pub amount: u32,
    pub side: OrderSide,
}
