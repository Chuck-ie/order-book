use crate::common::OrderSide;

pub mod arena_order_matcher;
pub mod v1_vec_only;
pub mod v2_btree;
pub mod v3_slot_map;
pub mod v4_sm_arena;
pub mod v5_sm_arena_vec_index;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitOrder {
    pub limit: u32,
    pub amount: u32,
    pub side: OrderSide,
}
