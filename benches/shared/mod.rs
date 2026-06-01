use crate::shared::bench_engine::{ArenaBenchEngine, DefaultBenchEngine};
use order_book::{
    arena_allocator::ArenaId,
    common::{LimitOrderRequest, MatcherCommand, OrderSide},
    engine::{
        v1_vec_only, v2_btree, v3_slot_map,
        v4_slot_map_arena::{self, LimitOrder},
    },
};

pub mod bench_engine;
pub mod bench_helpers;
pub mod charts;
pub mod smem_prof;

pub type EngineV1 = DefaultBenchEngine<v1_vec_only::matcher::OrderMatcher>;
pub type EngineV2 = DefaultBenchEngine<v2_btree::matcher::OrderMatcher>;
pub type EngineV3 = DefaultBenchEngine<v3_slot_map::matcher::OrderMatcher>;
pub type EngineV4 = ArenaBenchEngine<v4_slot_map_arena::matcher::OrderMatcher>;

pub const MEMORY_FOOTPRINT_PLACE_ORDERS_CSV_PATH: &str =
    "benches/results/memory_footprint_place_orders_level_scaling.csv";

pub const MEMORY_FOOTPRINT_CANCEL_ORDERS_CSV_PATH: &str =
    "benches/results/memory_footprint_cancel_orders_level_scaling.csv";

pub const LEVEL_SCALINGS: [(usize, usize); 5] = [
    (1, 100_000),
    (10, 10_000),
    (100, 1_000),
    (1_000, 100),
    (10_000, 10),
];

#[must_use]
pub fn generate_level_scaled_orders(
    mid_price: usize,
    total_levels: usize,
    orders_per_level: usize,
) -> Vec<SyntheticOrder> {
    let mut orders = Vec::with_capacity(total_levels * orders_per_level);

    for level in 0..total_levels {
        for _ in 0..orders_per_level {
            orders.push(SyntheticOrder {
                side: OrderSide::Bid,
                limit: (mid_price - level) as u64,
                amount: 1,
            });
        }
    }

    orders
}

pub struct SyntheticOrder {
    pub side: OrderSide,
    pub limit: u64,
    pub amount: u64,
}

impl<OrderId: Clone> From<SyntheticOrder> for MatcherCommand<LimitOrderRequest, OrderId> {
    fn from(value: SyntheticOrder) -> Self {
        Self::PlaceOrder(LimitOrderRequest {
            side: value.side,
            limit: value.limit,
            amount: value.amount,
        })
    }
}

impl From<SyntheticOrder> for MatcherCommand<LimitOrder, ArenaId> {
    #[allow(clippy::cast_possible_truncation)]
    fn from(value: SyntheticOrder) -> Self {
        Self::PlaceOrder(LimitOrder {
            side: value.side,
            limit: value.limit as u32,
            amount: value.amount as u32,
        })
    }
}
