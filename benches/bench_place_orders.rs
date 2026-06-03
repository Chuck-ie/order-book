use std::{
    fs::File,
    time::{Duration, Instant},
};

use crate::shared::{
    EngineV1, EngineV2, EngineV3, EngineV4, LEVEL_SCALINGS, MEMORY_FOOTPRINT_PLACE_ORDERS_CSV_PATH,
    NARROW, ORDER_STRATEGIES, OrderStrategy, PersistentScalingOrderThroughputRow,
    THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_NARROW_CSV_PATH,
    THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_WIDE_CSV_PATH, WIDE,
    bench_engine::BenchEngine,
    bench_helpers::{OrderProfile, generate_synthetic_orders},
    generate_level_scaled_orders,
    smem_prof::{SMEM_PROF, SMemProfGuard},
};
use criterion::{
    BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput, measurement::WallTime,
};
use csv::Writer;
use rand::seq::SliceRandom;
mod shared;

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_place_orders_level_scaling(&mut criterion);
    bench_place_orders_level_scaling_memory_footprint();
    bench_place_orders_persistent_scaling();
}

#[rustfmt::skip]
fn bench_place_orders_level_scaling(c: &mut Criterion) {
    fn bench_fn<Engine: BenchEngine>(
        group: &mut BenchmarkGroup<'_, WallTime>,
        engine_name: &str,
        strategy: OrderStrategy,
        total_levels: usize,
        orders_per_level: usize,
    ) {
        let parameter_id = format!("levels_{total_levels}/orders_{orders_per_level}");
        let benchmark_id = BenchmarkId::new(engine_name, parameter_id);
        group.measurement_time(Duration::from_secs(10));

        group.bench_with_input(benchmark_id, &(total_levels, orders_per_level), |b, _| {
            b.iter_batched(
                || {
                    setup_place_orders_level_scaling::<Engine>(
                        strategy,
                        10_000,
                        total_levels,
                        orders_per_level,
                    )
                },
                run_place_orders_level_scaling::<Engine>,
                BatchSize::SmallInput,
            );
        });
    }

    for (strategy_name, strategy) in ORDER_STRATEGIES {
        let mut group = c.benchmark_group(format!("Level Scaling/Place Orders/{strategy_name}"));
        group.sample_size(10);
        group.noise_threshold(0.05);

        for (total_levels, orders_per_level) in LEVEL_SCALINGS {
            let total_orders = total_levels * orders_per_level;
            group.throughput(Throughput::Elements(total_orders as u64));

            bench_fn::<EngineV1>(&mut group, "EngineV1", strategy, total_levels, orders_per_level);
            bench_fn::<EngineV2>(&mut group, "EngineV2", strategy, total_levels, orders_per_level);
            bench_fn::<EngineV3>(&mut group, "EngineV3", strategy, total_levels, orders_per_level);
            bench_fn::<EngineV4>(&mut group, "EngineV4", strategy, total_levels, orders_per_level);
        }
    }
}

fn setup_place_orders_level_scaling<Engine: BenchEngine>(
    strategy: OrderStrategy,
    mid_price: usize,
    total_levels: usize,
    orders_per_level: usize,
) -> Vec<Engine::Command> {
    let orders = generate_level_scaled_orders(mid_price, total_levels, orders_per_level);
    let mut place_commands = orders
        .into_iter()
        .map(std::convert::Into::into)
        .collect::<Vec<Engine::Command>>();

    match strategy {
        OrderStrategy::Default => {}
        OrderStrategy::Reverse => place_commands.reverse(),
        OrderStrategy::Random => place_commands.shuffle(&mut rand::rng()),
    }

    place_commands
}

fn run_place_orders_level_scaling<Engine: BenchEngine>(commands: Vec<Engine::Command>) {
    let mut engine = Engine::default();

    for cmd in commands {
        engine.process(std::hint::black_box(cmd));
    }
}

fn bench_place_orders_level_scaling_memory_footprint() {
    fn run_and_record<Engine: BenchEngine>(
        writer: &mut Writer<File>,
        engine_name: &str,
        total_levels: usize,
        orders_per_level: usize,
    ) {
        let setup = setup_place_orders_level_scaling::<Engine>(
            OrderStrategy::Random,
            10_000,
            total_levels,
            orders_per_level,
        );

        SMEM_PROF.reset();

        let guard = SMemProfGuard::new();
        run_place_orders_level_scaling::<Engine>(setup);
        drop(guard);

        writer
            .serialize(SMEM_PROF.as_row(engine_name, total_levels, orders_per_level))
            .expect("failed to write row");
    }

    let file = File::create(MEMORY_FOOTPRINT_PLACE_ORDERS_CSV_PATH).expect("could not create file");
    let mut writer = Writer::from_writer(file);

    for (total_levels, orders_per_level) in LEVEL_SCALINGS {
        run_and_record::<EngineV1>(&mut writer, "EngineV1", total_levels, orders_per_level);
        run_and_record::<EngineV2>(&mut writer, "EngineV2", total_levels, orders_per_level);
        run_and_record::<EngineV3>(&mut writer, "EngineV3", total_levels, orders_per_level);
        run_and_record::<EngineV4>(&mut writer, "EngineV4", total_levels, orders_per_level);
    }

    writer.flush().expect("failed to write file");
}

#[rustfmt::skip]
fn bench_place_orders_persistent_scaling() {
    fn bench_fn<Engine: BenchEngine>(
        writer: &mut Writer<File>,
        engine_name: &str,
        order_profile: &OrderProfile,
        total_batches: usize,
        orders_per_batch: usize,
    ) {
        println!("Benching: {engine_name}; total_batches: {total_batches}, orders_per_batch: {orders_per_batch}");
        let mut bench_state = Engine::default();

        for batch_idx in 0..total_batches {
            let target_commands =
                generate_synthetic_orders(order_profile, orders_per_batch)
                    .into_iter()
                    .map(std::convert::Into::into)
                    .collect::<Vec<Engine::Command>>();

            let start = Instant::now();

            for cmd in target_commands {
                bench_state.process(std::hint::black_box(cmd));
            }

            let duration = start.elapsed();
            let seconds = duration.as_secs_f64();

            let m_orders_per_second = if seconds > 0.0 {
                (orders_per_batch as f64 / 1_000_000.0) / seconds
            } else {
                0.0
            };

            let row = PersistentScalingOrderThroughputRow {
                engine: engine_name.to_string(),
                batch: batch_idx + 1,
                m_orders_per_second
            };

            writer.serialize(row).expect("failed to write row");
        }
    }

    let file = File::create(THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_NARROW_CSV_PATH).expect("could not create file");
    let mut writer = Writer::from_writer(file);

    bench_fn::<EngineV1>(&mut writer, "EngineV1", &NARROW, 1000, 1000);
    bench_fn::<EngineV2>(&mut writer, "EngineV2", &NARROW, 1000, 1000);
    bench_fn::<EngineV3>(&mut writer, "EngineV3", &NARROW, 1000, 1000);
    bench_fn::<EngineV4>(&mut writer, "EngineV4", &NARROW, 1000, 1000);
    writer.flush().expect("failed to write to file");


    let file = File::create(THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_WIDE_CSV_PATH).expect("could not create file");
    let mut writer = Writer::from_writer(file);

    bench_fn::<EngineV1>(&mut writer, "EngineV1", &WIDE, 1000, 1000);
    bench_fn::<EngineV2>(&mut writer, "EngineV2", &WIDE, 1000, 1000);
    bench_fn::<EngineV3>(&mut writer, "EngineV3", &WIDE, 1000, 1000);
    bench_fn::<EngineV4>(&mut writer, "EngineV4", &WIDE, 1000, 1000);
    writer.flush().expect("failed to write to file");
}
