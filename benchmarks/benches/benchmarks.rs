use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use shared::{
    MatcherCommand, OrderMatcherExt, OrderSide, ob_naive, ob_slot_map_optimized,
    ob_slot_map_unsafe, ob_standard,
};

#[inline(always)]
#[allow(clippy::inline_always)]
fn run<ID, M>(mut matcher: M, commands: Vec<MatcherCommand<ID>>)
where
    M: OrderMatcherExt<OrderId = ID>,
{
    for cmd in commands {
        black_box(matcher.process(black_box(cmd)));
    }
}

macro_rules! bench_impl {
    ($c:expr, $bench_name:expr, [ $( ($name:expr, $matcher:ty) ),* ], $setup:ident) => {
        let mut group = $c.benchmark_group($bench_name);
        let sizes = [1_000, 10_000, 100_000];

        for n in sizes {
        let (samples, measurement) = match n {
                100_000 => (100, 10),
                10_000  => (250, 20),
                _       => (500, 30),
            };

            $(

                let bench_id = format!("{}/{}", $name, n);
                group.sample_size(samples);
                group.measurement_time(std::time::Duration::from_secs(measurement));
                group.throughput(Throughput::Elements(n as u64));
                group.warm_up_time(std::time::Duration::from_secs(2));
                group.noise_threshold(0.02);

                group.bench_function(&bench_id, |b| {
                    b.iter_batched(
                        || $setup::<$matcher>(n),
                        |(matcher, commands)| run(matcher, commands),
                        BatchSize::LargeInput
                    )
                });
            )*
        }

        group.finish()
    };
}

fn bench_place_order_same_level(c: &mut Criterion) {
    fn setup<M: OrderMatcherExt>(n: usize) -> (M, Vec<MatcherCommand<M::OrderId>>) {
        let matcher = M::new();
        let mut commands = Vec::with_capacity(n);

        (0..n).for_each(|_| {
            commands.push(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        });

        (matcher, commands)
    }

    bench_impl!(
        c,
        "place_order_same_level",
        [
            // ("ob_naive", ob_naive::OrderMatcher),
            // ("ob_standard", ob_standard::OrderMatcher),
            // ("ob_slot_map_naive", ob_slot_map_naive::OrderMatcher),
            // ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher),
            ("ob_slot_map_unsafe", ob_slot_map_unsafe::OrderMatcher)
        ],
        setup
    );
}

fn bench_place_order_different_levels(c: &mut Criterion) {
    fn setup<M: OrderMatcherExt>(n: usize) -> (M, Vec<MatcherCommand<M::OrderId>>) {
        let matcher = M::new();
        let mut commands = Vec::with_capacity(n);

        #[allow(clippy::cast_possible_truncation)]
        (0..n).for_each(|i| {
            commands.push(MatcherCommand::new_limit_order(OrderSide::Bid, i as u32, 1));
        });

        (matcher, commands)
    }

    bench_impl!(
        c,
        "place_order_different_levels",
        [
            // ("ob_naive", ob_naive::OrderMatcher),
            // ("ob_standard", ob_standard::OrderMatcher),
            // ("ob_slot_map_naive", ob_slot_map_naive::OrderMatcher),
            // ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher),
            ("ob_slot_map_unsafe", ob_slot_map_unsafe::OrderMatcher)
        ],
        setup
    );
}

fn bench_cancel_order_same_level(c: &mut Criterion) {
    fn setup<M: OrderMatcherExt>(n: usize) -> (M, Vec<MatcherCommand<M::OrderId>>) {
        let mut matcher = M::new();
        let mut commands = Vec::with_capacity(n);

        (0..n).for_each(|_| {
            let id = matcher
                .process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1))
                .unwrap();

            commands.push(MatcherCommand::CancelOrder(id));
        });

        (matcher, commands)
    }

    bench_impl!(
        c,
        "cancel_order_same_level",
        [
            // ("ob_naive", ob_naive::OrderMatcher),
            // ("ob_standard", ob_standard::OrderMatcher),
            // ("ob_slot_map_naive", ob_slot_map_naive::OrderMatcher),
            // ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher),
            ("ob_slot_map_unsafe", ob_slot_map_unsafe::OrderMatcher)
        ],
        setup
    );
}

fn bench_cancel_order_different_levels(c: &mut Criterion) {
    fn setup<M: OrderMatcherExt>(n: usize) -> (M, Vec<MatcherCommand<M::OrderId>>) {
        let mut matcher = M::new();
        let mut commands = Vec::with_capacity(n);

        #[allow(clippy::cast_possible_truncation)]
        (0..n).for_each(|i| {
            let id = matcher
                .process(MatcherCommand::new_limit_order(OrderSide::Bid, i as u32, 1))
                .unwrap();

            commands.push(MatcherCommand::CancelOrder(id));
        });

        (matcher, commands)
    }

    bench_impl!(
        c,
        "cancel_order_different_levels",
        [
            // ("ob_naive", ob_naive::OrderMatcher),
            // ("ob_standard", ob_standard::OrderMatcher),
            // ("ob_slot_map_naive", ob_slot_map_naive::OrderMatcher),
            // ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher),
            ("ob_slot_map_unsafe", ob_slot_map_unsafe::OrderMatcher)
        ],
        setup
    );
}

criterion_group!(
    benches,
    bench_place_order_same_level,
    bench_place_order_different_levels,
    bench_cancel_order_same_level,
    bench_cancel_order_different_levels
);
criterion_main!(benches);
