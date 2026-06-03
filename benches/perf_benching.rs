use std::time::Instant;

use order_book::{
    arena_allocator::{ArenaAllocator, ArenaId},
    common::MatcherCommand,
    engine::{
        LimitOrder,
        arena_order_matcher::{ArenaOrderMatcher, ArenaOrderMatcherExt},
        v4_sm_arena, v5_sm_arena_vec_index,
    },
};

use crate::shared::{
    NARROW, WIDE, bench_helpers::generate_synthetic_orders, generate_level_scaled_orders,
};

mod shared;

fn main() {
    let mut wrapper = ArenaOrderMatcher {
        arena: ArenaAllocator::new(16384, 16384),
        matcher: v4_sm_arena::matcher::OrderMatcher::new(),
    };

    let commands: Vec<_> = generate_synthetic_orders(&WIDE, 100_000_000)
        .iter()
        .map(|order| {
            MatcherCommand::PlaceOrder(LimitOrder {
                limit: order.limit as u32,
                amount: order.amount as u32,
                side: order.side,
            })
        })
        .collect();

    let total_orders = commands.len();
    let start = Instant::now();

    for cmd in commands {
        wrapper.process(std::hint::black_box(cmd));
    }

    let elapsed = start.elapsed();

    println!(
        "{:.3} Mitems/s",
        (total_orders as f64) / elapsed.as_secs_f64() / 1_000_000.
    );
}
