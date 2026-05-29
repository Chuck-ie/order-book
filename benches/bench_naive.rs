use std::{
    sync::atomic::Ordering::Relaxed,
    time::{Duration, Instant},
};

use crate::shared::{
    bench_helpers::{
        ArenaBenchState, BenchState, DefaultBenchState, OrderProfile, SyntheticOrder,
        setup_bench_place_orders,
    },
    charts::{SMemProfSnapshot, get_results_registry, update_shared_memory_chart},
    smem_prof::{SMEM_PROF, SMemProfGuard},
};
use criterion::{BenchmarkId, Criterion, Throughput};
use order_book::{
    common::MatcherCommand,
    engine::{v1_vec_only, v2_btree, v3_slot_map, v4_slot_map_arena},
};

mod shared;

type EngineV1 = DefaultBenchState<v1_vec_only::matcher::OrderMatcher>;
type EngineV2 = DefaultBenchState<v2_btree::matcher::OrderMatcher>;
type EngineV3 = DefaultBenchState<v3_slot_map::matcher::OrderMatcher>;
type EngineV4 = ArenaBenchState<v4_slot_map_arena::matcher::OrderMatcher>;

static NARROW: OrderProfile = OrderProfile::place_narrow();
static WIDE: OrderProfile = OrderProfile::place_wide();

macro_rules! bench_place_orders_persistent {
    ($engine:ty, $bench_name:ident, $order_profile:expr, $total_batches:expr, $orders_per_batch:expr) => {
        fn $bench_name(c: &mut criterion::Criterion) {
            run_bench_place_orders_persistent::<$engine>(
                c,
                stringify!($bench_name),
                $order_profile,
                $total_batches,
                $orders_per_batch,
            );
        }
    };
}

// all versions
bench_place_orders_persistent!(EngineV1, v1_bpop_full_n, &NARROW, 10, 10_000);
bench_place_orders_persistent!(EngineV1, v1_bpop_full_w, &WIDE, 10, 10_000);
bench_place_orders_persistent!(EngineV2, v2_bpop_full_n, &NARROW, 10, 10_000);
bench_place_orders_persistent!(EngineV2, v2_bpop_full_w, &WIDE, 10, 10_000);
bench_place_orders_persistent!(EngineV3, v3_bpop_full_n, &NARROW, 10, 10_000);
bench_place_orders_persistent!(EngineV3, v3_bpop_full_w, &WIDE, 10, 10_000);
bench_place_orders_persistent!(EngineV4, v4_bpop_full_n, &NARROW, 10, 10_000);
bench_place_orders_persistent!(EngineV4, v4_bpop_full_w, &WIDE, 10, 10_000);

// optimized versions only
bench_place_orders_persistent!(EngineV3, v3_bpop_optimized_n, &NARROW, 1000, 100_000);
bench_place_orders_persistent!(EngineV3, v3_bpop_optimized_w, &WIDE, 1000, 100_000);
bench_place_orders_persistent!(EngineV4, v4_bpop_optimized_n, &NARROW, 1000, 100_000);
bench_place_orders_persistent!(EngineV4, v4_bpop_optimized_w, &WIDE, 1000, 100_000);

criterion::criterion_group!(
    bench_place_orders_persistent,
    // all versions
    v1_bpop_full_n,
    v1_bpop_full_w,
    v2_bpop_full_n,
    v2_bpop_full_w,
    v3_bpop_full_n,
    v3_bpop_full_w,
    v4_bpop_full_n,
    v4_bpop_full_w,
    // optimized versions only
    v3_bpop_optimized_n,
    v3_bpop_optimized_w,
    v4_bpop_optimized_n,
    v4_bpop_optimized_w,
);

fn run_bench_place_orders_persistent<S: BenchState + Default>(
    c: &mut Criterion,
    bench_name: &str,
    order_profile: &OrderProfile,
    total_batches: usize,
    orders_per_batch: usize,
) where
    MatcherCommand<S::Order, S::OrderId>: From<SyntheticOrder>,
{
    let batches_of_commands: Vec<_> = (0..total_batches)
        .map(|_| setup_bench_place_orders(order_profile, orders_per_batch))
        .collect();

    let mut group = c.benchmark_group(bench_name);
    group.throughput(Throughput::Elements(orders_per_batch as u64));
    group.sample_size(10);
    group.noise_threshold(0.05);
    group.warm_up_time(Duration::from_nanos(1));
    group.measurement_time(Duration::from_nanos(1));

    for batch_idx in 0..10 {
        group.bench_with_input(
            BenchmarkId::new("Batch: ", batch_idx + 1),
            &batch_idx,
            |b, &current_batch_idx| {
                b.iter_custom(|iters| {
                    let mut total_duration = Duration::ZERO;

                    for _ in 0..iters {
                        let mut bench_state = S::default();
                        let previous_commands =
                            batches_of_commands[0..current_batch_idx].iter().cloned();

                        for previous_batch in previous_commands {
                            for cmd in previous_batch {
                                bench_state.process(std::hint::black_box(cmd));
                            }
                        }

                        let target_commands = batches_of_commands[current_batch_idx].clone();
                        let start = Instant::now();

                        for cmd in target_commands {
                            bench_state.process(std::hint::black_box(cmd));
                        }

                        total_duration += start.elapsed();
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

macro_rules! bench_memory_footprint {
    ($engine:ty, $bench_name:ident, $order_profile:expr, $chart_file_name:expr, $total_batches:expr, $orders_per_batch:expr) => {
        fn $bench_name(_c: &mut criterion::Criterion) {
            run_bench_memory_footprint::<$engine>(
                stringify!($bench_name),
                $order_profile,
                $total_batches,
                $orders_per_batch,
                $chart_file_name,
                stringify!($engine, $bench_name),
            );
        }
    };
}

/// short name for ``bench_memory_footprint_all`` benchmarks
const FULL_BMF_FN: &str = "bench_memory_footprint_full.html";
/// short name for ``bench_memory_footprint_optimized`` benchmarks
const OPT_BMF_FN: &str = "bench_memory_footprint_optimized.html";

bench_memory_footprint!(EngineV1, v1_bmf_n, &NARROW, FULL_BMF_FN, 1, 100_000);
bench_memory_footprint!(EngineV1, v1_bmf_w, &WIDE, FULL_BMF_FN, 1, 100_000);
bench_memory_footprint!(EngineV2, v2_bmf_n, &NARROW, FULL_BMF_FN, 1, 100_000);
bench_memory_footprint!(EngineV2, v2_bmf_w, &WIDE, FULL_BMF_FN, 1, 100_000);
bench_memory_footprint!(EngineV3, v3_bmf_full_n, &NARROW, FULL_BMF_FN, 1, 100_000);
bench_memory_footprint!(EngineV3, v3_bmf_full_w, &WIDE, FULL_BMF_FN, 1, 100_000);
bench_memory_footprint!(EngineV4, v4_bmf_full_n, &NARROW, FULL_BMF_FN, 1, 100_000);
bench_memory_footprint!(EngineV4, v4_bmf_full_w, &WIDE, FULL_BMF_FN, 1, 100_000);

criterion::criterion_group!(
    bench_memory_footprint_full,
    v1_bmf_n,
    v1_bmf_w,
    v2_bmf_n,
    v2_bmf_w,
    v3_bmf_full_n,
    v3_bmf_full_w,
    v4_bmf_full_n,
    v4_bmf_full_w
);

bench_memory_footprint!(EngineV3, v3_bmf_opt_n, &NARROW, OPT_BMF_FN, 100, 100_000);
bench_memory_footprint!(EngineV3, v3_bmf_opt_w, &WIDE, OPT_BMF_FN, 100, 100_000);
bench_memory_footprint!(EngineV4, v4_bmf_opt_n, &NARROW, OPT_BMF_FN, 100, 100_000);
bench_memory_footprint!(EngineV4, v4_bmf_opt_w, &WIDE, OPT_BMF_FN, 100, 100_000);

criterion::criterion_group!(
    bench_memory_footprint_optimized,
    v3_bmf_opt_n,
    v3_bmf_opt_w,
    v4_bmf_opt_n,
    v4_bmf_opt_w
);

fn run_bench_memory_footprint<S: BenchState + Default>(
    bench_name: &str,
    order_profile: &OrderProfile,
    total_batches: usize,
    orders_per_batch: usize,
    chart_file_name: &str,
    chart_id: &str,
) where
    MatcherCommand<S::Order, S::OrderId>: From<SyntheticOrder>,
{
    let mut bench_state = S::default();
    SMEM_PROF.reset();

    for _ in 0..total_batches {
        let commands = setup_bench_place_orders(order_profile, orders_per_batch);
        let guard = SMemProfGuard::new();

        for cmd in commands {
            bench_state.process(std::hint::black_box(cmd));
        }

        drop(guard);
    }

    println!("{bench_name}: SMemProfStats: {SMEM_PROF:#?}");

    #[allow(clippy::cast_precision_loss)]
    let snapshot = SMemProfSnapshot {
        name: chart_file_name.to_string(),
        id: chart_id.to_string(),
        alloc_bytes_mb: SMEM_PROF.alloc_bytes.load(Relaxed) as f64 / 1_048_576.0,
        dealloc_bytes_mb: SMEM_PROF.dealloc_bytes.load(Relaxed) as f64 / 1_048_576.0,
        grow_bytes_mb: SMEM_PROF.grow_bytes.load(Relaxed) as f64 / 1_048_576.0,
    };

    {
        let mut registry = get_results_registry().lock().unwrap();
        registry.push(snapshot);
    }

    update_shared_memory_chart(chart_file_name);
}

criterion::criterion_main!(
    // bench_memory_footprint_full,
    // bench_memory_footprint_optimized,
    bench_place_orders_persistent,
);
