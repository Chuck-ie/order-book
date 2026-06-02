use std::{fs::File, time::Duration};

use crate::shared::{
    EngineV1, EngineV2, EngineV3, EngineV4, LEVEL_SCALINGS,
    MEMORY_FOOTPRINT_CANCEL_ORDERS_CSV_PATH, ORDER_STRATEGIES, OrderStrategy,
    bench_engine::BenchEngine,
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
    bench_cancel_orders_level_scaling(&mut criterion);
    bench_cancel_orders_level_scaling_memory_footprint();
}

#[rustfmt::skip]
fn bench_cancel_orders_level_scaling(c: &mut Criterion) {
    fn bench_fn<Engine: BenchEngine>(
        group: &mut BenchmarkGroup<'_, WallTime>,
        engine_name: &str,
        strategy: OrderStrategy,
        total_levels: usize,
        orders_per_level: usize,
    )
    {
        let parameter_id = format!("levels_{total_levels}/orders_{orders_per_level}");
        let benchmark_id = BenchmarkId::new(engine_name, parameter_id);
        group.measurement_time(Duration::from_secs(10));

        group.bench_with_input(benchmark_id, &(total_levels, orders_per_level), |b, _| {
            b.iter_batched(
                || {
                    setup_cancel_orders_level_scaling::<Engine>(
                        strategy,
                        10_000,
                        total_levels,
                        orders_per_level,
                    )
                },
                |(engine, commands)| {
                    run_cancel_orders_level_scaling::<Engine>(engine, commands);
                },
                BatchSize::SmallInput,
            );
        });
    }

    for (strategy_name, strategy) in ORDER_STRATEGIES {
        let mut group = c.benchmark_group(format!("Level Scaling/Cancel Orders/{strategy_name}"));
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

        group.finish();
    }
}

fn setup_cancel_orders_level_scaling<Engine: BenchEngine>(
    strategy: OrderStrategy,
    mid_price: usize,
    total_levels: usize,
    orders_per_level: usize,
) -> (Engine, Vec<Engine::Command>) {
    let orders = generate_level_scaled_orders(mid_price, total_levels, orders_per_level);
    let commands = orders
        .into_iter()
        .map(std::convert::Into::into)
        .collect::<Vec<Engine::Command>>();

    let mut engine = Engine::default();
    let mut cancel_commands = Vec::with_capacity(total_levels * orders_per_level);

    for cmd in commands {
        let order_id = engine
            .process(std::hint::black_box(cmd))
            .expect("Did not receive an order id from process");

        cancel_commands.push(Engine::new_cancel_order(order_id));
    }

    match strategy {
        OrderStrategy::Default => {}
        OrderStrategy::Reverse => cancel_commands.reverse(),
        OrderStrategy::Random => cancel_commands.shuffle(&mut rand::rng()),
    }

    (engine, cancel_commands)
}

fn run_cancel_orders_level_scaling<Engine: BenchEngine>(
    mut engine: Engine,
    commands: Vec<Engine::Command>,
) {
    for cmd in commands {
        engine.process(std::hint::black_box(cmd));
    }
}

#[rustfmt::skip]
fn bench_cancel_orders_level_scaling_memory_footprint() {
    fn run_and_record<Engine: BenchEngine>(
        writer: &mut Writer<File>,
        engine_name: &str,
        total_levels: usize,
        orders_per_level: usize,
    ) {
        let (engine, commands) = setup_cancel_orders_level_scaling::<Engine>(
            OrderStrategy::Default,
            10_000,
            total_levels,
            orders_per_level,
        );

        SMEM_PROF.reset();

        let guard = SMemProfGuard::new();
        run_cancel_orders_level_scaling(engine, commands);
        drop(guard);

        writer
            .serialize(SMEM_PROF.as_row(engine_name, total_levels, orders_per_level))
            .expect("failed to write row");
    }

    let file = File::create(MEMORY_FOOTPRINT_CANCEL_ORDERS_CSV_PATH)
        .expect("could not create file");

    let mut writer = Writer::from_writer(file);

    for (total_levels, orders_per_level) in LEVEL_SCALINGS {
        run_and_record::<EngineV1>(&mut writer, "EngineV1", total_levels, orders_per_level);
        run_and_record::<EngineV2>(&mut writer, "EngineV2", total_levels, orders_per_level);
        run_and_record::<EngineV3>(&mut writer, "EngineV3", total_levels, orders_per_level);
        run_and_record::<EngineV4>(&mut writer, "EngineV4", total_levels, orders_per_level);
    }
}
