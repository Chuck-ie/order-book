use divan::AllocProfiler;
use mimalloc::MiMalloc;
use order_book::{
    common::OrderMatcherExt,
    engine::v4_slot_map_arena::{self},
};

use crate::common::SyntheticOrder;

#[path = "common.rs"]
mod common;

#[global_allocator]
static ALLOC: AllocProfiler<MiMalloc> = AllocProfiler::new(MiMalloc);

fn main() {
    divan::main();
}

macro_rules! bench_engine {
    ($group:ident, $engine:ty, $order_profile:expr) => {
        mod $group {
            use super::*;
            use divan::{Bencher, counter::ItemsCount};

            // #[divan::bench(sample_count = 100, args = [1_000_000])]
            #[divan::bench(sample_count = 1, args = [800_000_000])]
            fn bench_place_orders_cold_start(bencher: Bencher, total_orders: usize) {
                bencher
                    .with_inputs(|| {
                        generate_cold_start_input::<$engine, _>(
                            total_orders,
                            $order_profile,
                            SyntheticOrder::into_order_request,
                        )
                    })
                    .input_counter(move |_| ItemsCount::new(total_orders))
                    .bench_local_values(|(mut engine, commands)| {
                        for cmd in commands {
                            engine.process(divan::black_box(cmd));
                        }
                    });
            }
        }
    };

    // v4 uses an external arena allocator that must persist across benchmark
    // iterations, so it can't share the same macro as v1-v3 which are self-contained.
    (arena; $group:ident, $engine:ty, $order_profile:expr) => {
        mod $group {
            use super::*;
            use divan::{Bencher, counter::ItemsCount};
            use order_book::arena_allocator::ArenaAllocator;

            // #[divan::bench(sample_count = 100, args = [1_000_000])]
            #[divan::bench(sample_count = 1, args = [800_000_000])]
            fn bench_place_orders_cold_start(bencher: Bencher, total_orders: usize) {
                // let mut arena = ArenaAllocator::new(4096, 16384);
                let mut arena = ArenaAllocator::new(16384, 4096);

                bencher
                    .with_inputs(|| {
                        generate_cold_start_input::<$engine, _>(
                            total_orders,
                            $order_profile,
                            SyntheticOrder::into_limit_order,
                        )
                    })
                    .input_counter(move |_| ItemsCount::new(total_orders))
                    .bench_local_values(|(mut engine, commands)| {
                        for cmd in commands {
                            engine.process(divan::black_box(cmd), &mut arena);
                        }

                        engine.clean_up(&mut arena);
                        arena.free_stack = (0..arena.chunk_count()).rev().collect();
                    });
            }
        }
    };
}

#[rustfmt::skip]
mod benches {
    use order_book::engine::{v1_vec_only, v2_btree, v3_slot_map};
    use super::*;

    // bench_engine!(bench_v1_vec_only_p1, v1_vec_only::matcher::OrderMatcher, (0, 100, 0));
    // bench_engine!(bench_v1_vec_only_p2, v1_vec_only::matcher::OrderMatcher, (60, 30, 10));
    //
    // bench_engine!(bench_v2_btree_p1, v2_btree::matcher::OrderMatcher, (0, 100, 0));
    // bench_engine!(bench_v2_btree_p2, v2_btree::matcher::OrderMatcher, (60, 30, 10));

    // bench_engine!(bench_v3_slot_map_p1, v3_slot_map::matcher::OrderMatcher, (0, 100, 0));
    // bench_engine!(bench_v3_slot_map_p2, v3_slot_map::matcher::OrderMatcher, (60, 30, 0));
    bench_engine!(bench_v3_slot_map_p3, v3_slot_map::matcher::OrderMatcher, (5, 90, 5));

    // v4 uses an external arena allocator that must persist across benchmark
    // iterations, so it can't share the same macro as v1-v3 which are self-contained.
    // bench_engine!(arena; bench_v4_slot_map_arena_p1, v4_slot_map_arena::matcher::OrderMatcher, (0, 100, 0));
    // bench_engine!(arena; bench_v4_slot_map_arena_p2, v4_slot_map_arena::matcher::OrderMatcher, (60, 30, 10));
    // bench_engine!(arena; bench_v4_slot_map_arena_p3, v4_slot_map_arena::matcher::OrderMatcher, (5, 90, 5));
}

fn generate_cold_start_input<Engine, Cmd>(
    total_orders: usize,
    order_profile: (usize, usize, usize),
    map_order: impl Fn(SyntheticOrder) -> Cmd,
) -> (Engine, Vec<Cmd>)
where
    Engine: Default,
{
    let engine = Engine::default();
    let commands = common::generate_synthetic_orders(total_orders, order_profile)
        .into_iter()
        .map(map_order)
        .collect();

    (engine, commands)
}
