use divan::AllocProfiler;
use shared::OrderMatcherExt;
use shared::{MatcherCommand, OrderSide};

#[global_allocator]
static ALLOC: AllocProfiler = AllocProfiler::system();

fn main() {
    divan::main();
}

type Naive = shared::ob_naive::OrderMatcher;
type Standard = shared::ob_standard::OrderMatcher;
type SlotMapStandard = shared::ob_slot_map_standard::OrderMatcher;
type SlotMapOptimized = shared::ob_slot_map_optimized::OrderMatcher;

#[divan::bench_group(name = "place_orders_same_level")]
mod place_orders_same_level {
    use super::{
        Naive, OrderMatcherExt, SlotMapOptimized, SlotMapStandard, Standard,
        generate_same_level_commands,
    };

    macro_rules! bench_place_orders_same_level {
        ($bench_name:ident, $matcher_type:ty) => {
            #[divan::bench(sample_size = 1, sample_count = 1000, args = [1_000, 10_000, 100_000])]
            fn $bench_name(bencher: divan::Bencher, n: usize) {
                bencher
                    .with_inputs(|| (<$matcher_type>::new(), generate_same_level_commands(n)))
                    .input_counter(move |_| divan::counter::ItemsCount::new(n))
                    .bench_values(|(mut matcher, commands)| {
                        for cmd in commands {
                            divan::black_box(matcher.process(divan::black_box(cmd)));
                        }
                    });
            }
        };
    }

    bench_place_orders_same_level!(naive, Naive);
    bench_place_orders_same_level!(standard, Standard);
    bench_place_orders_same_level!(slot_map_standard, SlotMapStandard);
    bench_place_orders_same_level!(slot_map_optimized, SlotMapOptimized);
}

#[divan::bench_group(name = "place_orders_diff_level")]
mod place_orders_diff_level {
    use super::{
        Naive, OrderMatcherExt, SlotMapOptimized, SlotMapStandard, Standard,
        generate_diff_level_commands,
    };

    macro_rules! bench_place_orders_diff_level {
        ($bench_name:ident, $matcher_type:ty) => {
            #[divan::bench(sample_size = 1, sample_count = 1000, args = [(1, 1_000), (10, 10_000), (100, 100_000)])]
            fn $bench_name(bencher: divan::Bencher, args: (usize, usize)) {
                let (total_levels, total_orders) = args;

                bencher
                    .with_inputs(|| {
                        (
                            <$matcher_type>::new(),
                            generate_diff_level_commands(total_levels, total_orders),
                        )
                    })
                    .input_counter(move |_| divan::counter::ItemsCount::new(total_orders))
                    .bench_values(|(mut matcher, commands)| {
                        for cmd in commands {
                            divan::black_box(matcher.process(divan::black_box(cmd)));
                        }
                    });
            }
        };
    }

    bench_place_orders_diff_level!(naive, Naive);
    bench_place_orders_diff_level!(standard, Standard);
    bench_place_orders_diff_level!(slot_map_standard, SlotMapStandard);
    bench_place_orders_diff_level!(slot_map_optimized, SlotMapOptimized);
}

// #[divan::bench(
//     name = "place_orders_diff_level",
//     args = [(1, 1_000), (10, 10_000), (100, 100_000), (1_000, 1_000_000)],
//     // types = [ob_naive::OrderMatcher, ob_standard::OrderMatcher, ob_slot_map_standard::OrderMatcher, ob_slot_map_optimized::OrderMatcher]
//     types = [Naive, Standard, SlotMapStandard, SlotMapOptimized]
// )]
// fn bench_place_orders_diff_level<Matcher>(bencher: Bencher, args: (usize, usize))
// where
//     Matcher: OrderMatcherExt + 'static,
// {
//     let (total_levels, total_orders) = args;
//
//     bencher
//         .with_inputs(|| {
//             let commands = generate_diff_level_commands(total_levels, total_orders);
//             (Matcher::new(), commands)
//         })
//         .input_counter(move |_| divan::counter::ItemsCount::new(total_orders))
//         .bench_values(|(mut matcher, commands)| {
//             for cmd in commands {
//                 divan::black_box(matcher.process(divan::black_box(cmd)));
//             }
//         });
// }

fn generate_same_level_commands<ID>(total_orders: usize) -> Vec<MatcherCommand<ID>> {
    (0..total_orders)
        .map(|_| MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1))
        .collect()
}

fn generate_diff_level_commands<ID>(
    total_levels: usize,
    total_orders: usize,
) -> Vec<MatcherCommand<ID>> {
    let orders_per_level = total_orders / total_levels;

    (0..total_levels)
        .flat_map(|level| {
            (0..orders_per_level)
                .map(move |_| MatcherCommand::new_limit_order(OrderSide::Bid, level as u64, 1))
        })
        .collect()
}
