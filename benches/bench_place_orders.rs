use std::fs::File;

use crate::shared::{
    EngineV1, EngineV2, EngineV3, EngineV4, LEVEL_SCALINGS, MEMORY_FOOTPRINT_PLACE_ORDERS_CSV_PATH,
    bench_engine::BenchEngine,
    generate_level_scaled_orders,
    smem_prof::{SMEM_PROF, SMemProfGuard},
};
use criterion::{
    BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput, measurement::WallTime,
};
use csv::Writer;
mod shared;

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    // bench_place_orders_level_scaling(&mut criterion);
    bench_place_orders_level_scaling_memory_footprint();
}

fn bench_place_orders_level_scaling(c: &mut Criterion) {
    fn bench_fn<Engine: BenchEngine>(
        group: &mut BenchmarkGroup<'_, WallTime>,
        engine_name: &str,
        total_levels: usize,
        orders_per_level: usize,
    ) {
        let parameter_id = format!("levels_{total_levels}/orders_{orders_per_level}");
        let benchmark_id = BenchmarkId::new(engine_name, parameter_id);

        group.bench_with_input(benchmark_id, &(total_levels, orders_per_level), |b, _| {
            b.iter_batched(
                || {
                    setup_place_orders_level_scaling::<Engine>(
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

    let mut group = c.benchmark_group("Level Scaling/Place Orders");
    group.sample_size(10);
    group.noise_threshold(0.05);

    for (total_levels, orders_per_level) in LEVEL_SCALINGS {
        let total_orders = total_levels * orders_per_level;
        group.throughput(Throughput::Elements(total_orders as u64));

        bench_fn::<EngineV1>(&mut group, "EngineV1", total_levels, orders_per_level);
        bench_fn::<EngineV2>(&mut group, "EngineV2", total_levels, orders_per_level);
        bench_fn::<EngineV3>(&mut group, "EngineV3", total_levels, orders_per_level);
        bench_fn::<EngineV4>(&mut group, "EngineV4", total_levels, orders_per_level);
    }
}

fn setup_place_orders_level_scaling<Engine: BenchEngine>(
    mid_price: usize,
    total_levels: usize,
    orders_per_level: usize,
) -> Vec<Engine::Command> {
    let orders = generate_level_scaled_orders(mid_price, total_levels, orders_per_level);
    orders.into_iter().map(std::convert::Into::into).collect()
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
        let setup =
            setup_place_orders_level_scaling::<Engine>(10_000, total_levels, orders_per_level);

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
