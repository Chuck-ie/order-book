use std::time::Instant;

use order_book::{
    arena_allocator::ArenaAllocator,
    common::{LimitOrderRequest, MatcherCommand, OrderMatcherExt},
    engine::{
        LimitOrder,
        arena_order_matcher::{ArenaOrderMatcher, ArenaOrderMatcherExt},
        v3_slot_map, v4_sm_arena, v5_sm_arena_vec_index,
    },
};

use crate::shared::{
    NARROW, WIDE,
    bench_helpers::{OrderProfile, generate_synthetic_orders},
};

mod shared;

fn main() {
    // run_v3();
    run_v4(&NARROW);
    run_v5(&NARROW);
}

fn run_v3() {
    let mut matcher = v3_slot_map::matcher::OrderMatcher::new();

    let commands: Vec<_> = generate_synthetic_orders(&WIDE, 100_000_000)
        .iter()
        .map(|order| {
            MatcherCommand::PlaceOrder(LimitOrderRequest {
                limit: order.limit,
                amount: order.amount,
                side: order.side,
            })
        })
        .collect();

    let total_orders = commands.len();
    let start = Instant::now();

    for cmd in commands {
        matcher.process(std::hint::black_box(cmd));
    }

    let elapsed = start.elapsed();

    println!(
        "{:.3} Mitems/s",
        (total_orders as f64) / elapsed.as_secs_f64() / 1_000_000.
    );
}

fn run_v4(order_profile: &OrderProfile) {
    let mut wrapper = ArenaOrderMatcher {
        arena: ArenaAllocator::new(16384, 16384),
        matcher: v4_sm_arena::matcher::OrderMatcher::new(),
    };

    let commands: Vec<_> = generate_synthetic_orders(order_profile, 100_000_000)
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

fn run_v5(order_profile: &OrderProfile) {
    let mut wrapper = ArenaOrderMatcher {
        arena: ArenaAllocator::new(16384, 16384),
        matcher: v5_sm_arena_vec_index::matcher::OrderMatcher::new(),
    };

    let commands: Vec<_> = generate_synthetic_orders(order_profile, 100_000_000)
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
