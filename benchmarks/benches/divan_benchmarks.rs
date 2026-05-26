use divan::AllocProfiler;
use mimalloc::MiMalloc;
use rand_distr::weighted::WeightedIndex;
use rand_distr::{Bernoulli, Distribution, Exp, LogNormal, Normal, Uniform};
use rgb::RGB8;
use shared::OrderSide;
use shared::final_ver::order_matcher::MatcherCommand;
use shared::{LimitOrderRequest, OrderMatcherExt};
use textplots::{Chart, ColorPlot, Shape};

#[global_allocator]
static ALLOC: AllocProfiler<MiMalloc> = AllocProfiler::new(MiMalloc);

// #[global_allocator]
// static GLOBAL: MiMalloc = MiMalloc;

#[allow(clippy::cast_precision_loss)]
fn main() {
    divan::main();

    // // testng synthetically generated commands
    // let synthetic_commands = generate_synthetic_commands::<u64>(1_000);
    //
    // let mut bid_points: Vec<(f32, f32)> = Vec::new();
    // let mut ask_points: Vec<(f32, f32)> = Vec::new();
    //
    // for (idx, cmd) in synthetic_commands.iter().enumerate() {
    //     if let MatcherCommand::PlaceOrder(LimitOrderRequest {
    //         side,
    //         limit: price,
    //         amount: _,
    //     }) = cmd
    //     {
    //         match side {
    //             OrderSide::Bid => bid_points.push((idx as f32, *price as f32)),
    //             OrderSide::Ask => ask_points.push((idx as f32, *price as f32)),
    //         }
    //     }
    // }
    //
    // Chart::new(100, 30, 0.0, synthetic_commands.len() as f32)
    //     .linecolorplot(
    //         &Shape::Points(&bid_points),
    //         RGB8 {
    //             r: 30,
    //             g: 100,
    //             b: 255,
    //         },
    //     )
    //     .linecolorplot(
    //         &Shape::Points(&ask_points),
    //         RGB8 {
    //             r: 220,
    //             g: 50,
    //             b: 50,
    //         },
    //     )
    //     .display();
}

type Naive = shared::ob_naive::OrderMatcher;
type Standard = shared::ob_standard::OrderMatcher;
type SlotMapStandard = shared::ob_slot_map_standard::OrderMatcher;
type SlotMapOptimized = shared::ob_slot_map_optimized::OrderMatcher;
type ArenaSlotMap = shared::ob_arena_slot_map::OrderMatcher;

#[divan::bench_group(name = "place_orders_same_level")]
mod place_orders_same_level {
    use super::{
        Naive, OrderMatcherExt, SlotMapOptimized, SlotMapStandard, Standard,
        generate_same_level_commands,
    };

    fn run_bench<M: OrderMatcherExt>(bencher: divan::Bencher, n: usize) {
        bencher
            .with_inputs(|| (M::new(), generate_same_level_commands(n)))
            .input_counter(move |_| divan::counter::ItemsCount::new(n))
            .bench_values(|(mut matcher, commands)| {
                for cmd in commands {
                    divan::black_box(matcher.process(divan::black_box(cmd)));
                }
            });
    }

    macro_rules! register_bench {
        ($bench_name:ident, $matcher_type:ty) => {
            #[divan::bench(sample_size = 1, sample_count = 100, args = [1_000, 10_000, 100_000])]
            fn $bench_name(bencher: divan::Bencher, n: usize) {
                run_bench::<$matcher_type>(bencher, n);
            }
        };
    }

    // register_bench!(naive, Naive);
    // register_bench!(standard, Standard);
    // register_bench!(slot_map_standard, SlotMapStandard);
    // register_bench!(slot_map_optimized, SlotMapOptimized);
}

#[divan::bench_group(name = "place_orders_diff_level")]
mod place_orders_diff_level {
    use super::{
        Naive, OrderMatcherExt, SlotMapOptimized, SlotMapStandard, Standard,
        generate_diff_level_commands,
    };

    fn run_bench<M: OrderMatcherExt>(bencher: divan::Bencher, args: (usize, usize)) {
        let (total_levels, total_orders) = args;

        bencher
            .with_inputs(|| {
                (
                    M::new(),
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

    macro_rules! register_bench {
        ($bench_name:ident, $matcher_type:ty) => {
            #[divan::bench(sample_size = 1, sample_count = 10, args = [(1, 1_000), (10, 10_000), (100, 100_000), (1_000, 100_000)])]
            fn $bench_name(bencher: divan::Bencher, args: (usize, usize)) {
                run_bench::<$matcher_type>(bencher, args);
            }
        };
    }

    // register_bench!(naive, Naive);
    // register_bench!(standard, Standard);
    // register_bench!(slot_map_standard, SlotMapStandard);
    // register_bench!(slot_map_optimized, SlotMapOptimized);
}

#[divan::bench_group(name = "place_synthetic_orders")]
mod place_synthetic_orders {
    use shared::{OrderBookExt, SlotMap};

    use crate::ArenaSlotMap;

    use super::{
        Naive, OrderMatcherExt, SlotMapOptimized, SlotMapStandard, Standard,
        generate_synthetic_commands,
    };

    fn run_bench<M: OrderMatcherExt>(bencher: divan::Bencher, total_orders: usize) {
        bencher
            .with_inputs(|| {
                let matcher = M::new();
                (matcher, generate_synthetic_commands(total_orders))
            })
            .input_counter(move |_| divan::counter::ItemsCount::new(total_orders))
            .bench_values(|(mut matcher, commands)| {
                for cmd in commands {
                    matcher.process(divan::black_box(cmd));
                }
            });
    }

    macro_rules! register_bench {
        ($bench_name:ident, $matcher_type:ty) => {
            // #[divan::bench(sample_count = 100, args = [10_000, 100_000, 1_000_000])]
            // #[divan::bench(sample_count = 1, args = [100_000_000])]
            #[divan::bench(sample_count = 100, args = [100_000])]
            fn $bench_name(bencher: divan::Bencher, n: usize) {
                run_bench::<$matcher_type>(bencher, n);
            }
        };
    }

    // register_bench!(naive, Naive);
    // register_bench!(standard, Standard);
    // register_bench!(slot_map_standard, SlotMapStandard);
    // register_bench!(slot_map_optimized, SlotMapOptimized);
    // register_bench!(arena_slot_map, ArenaSlotMap);
}

#[divan::bench_group(name = "place_synthetic_orders_2")]
mod place_synthetic_orders_2 {
    use crate::generate_synthetic_commands_2;
    use shared::final_ver::arena_slot_allocator::ArenaSlotAllocator;

    // #[divan::bench(sample_count = 10, args = [10_000, 100_000, 1_000_000])]
    // #[divan::bench(sample_count = 1, args = [100_000_000])]
    #[divan::bench(sample_count = 1000, args = [100_000])]
    fn run_bench(bencher: divan::Bencher, total_orders: usize) {
        let mut arena = ArenaSlotAllocator::new(4096, 4096);

        bencher
            .with_inputs(|| {
                let matcher = shared::final_ver::order_matcher::OrderMatcher::new();
                (matcher, generate_synthetic_commands_2(total_orders))
            })
            .input_counter(move |_| divan::counter::ItemsCount::new(total_orders))
            .bench_local_values(|(mut matcher, commands)| {
                let commands = divan::black_box(commands);

                for cmd in commands {
                    matcher.process(cmd, &mut arena);
                }

                matcher.clean_up(&mut arena);
                arena.free_stack = (0..arena.chunk_count()).rev().collect();
            });
    }
}

fn generate_same_level_commands<ID>(total_orders: usize) -> Vec<shared::MatcherCommand<ID>> {
    (0..total_orders)
        .map(|_| shared::MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1))
        .collect()
}

fn generate_diff_level_commands<ID>(
    total_levels: usize,
    total_orders: usize,
) -> Vec<shared::MatcherCommand<ID>> {
    let orders_per_level = total_orders / total_levels;

    (0..total_levels)
        .flat_map(|level| {
            (0..orders_per_level).map(move |_| {
                shared::MatcherCommand::new_limit_order(OrderSide::Bid, level as u64, 1)
            })
        })
        .collect()
}

fn generate_synthetic_commands<ID>(total_orders: usize) -> Vec<shared::MatcherCommand<ID>> {
    let mut commands = Vec::with_capacity(total_orders);
    let mut rng = rand::rng();

    let mid_price: i64 = 10_000;
    let half_spread: i64 = 1;

    // let type_distr = WeightedIndex::new([60, 30, 10]).unwrap();

    let type_distr = WeightedIndex::new([5, 90, 5]).unwrap();
    // let type_distr = WeightedIndex::new([90, 5, 5]).unwrap();

    // 50% bids, 50% asks
    let side_distr = Bernoulli::new(0.50).unwrap();

    let qty_distr = LogNormal::new(3.4, 0.9).unwrap();

    let passive_distr = Exp::new(0.4).unwrap();

    let marketable_distr = Uniform::new(0, 4).unwrap();

    let far_distr = Uniform::new(20, 80).unwrap();

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    for _ in 0..total_orders {
        let is_bid = side_distr.sample(&mut rng);
        let qty = (qty_distr.sample(&mut rng) as f64).max(1.0).round() as u64;
        let order_type = type_distr.sample(&mut rng);

        let price: i64 = match order_type {
            0 => {
                let dist = ((passive_distr.sample(&mut rng) as f64).round() as i64).clamp(0, 50);

                if is_bid {
                    mid_price - half_spread - dist
                } else {
                    mid_price + half_spread + dist
                }
            }
            1 => {
                let aggression = marketable_distr.sample(&mut rng);
                if is_bid {
                    mid_price + half_spread + aggression
                } else {
                    mid_price - half_spread - aggression
                }
            }
            2 => {
                let dist = far_distr.sample(&mut rng);
                if is_bid {
                    mid_price - half_spread - dist
                } else {
                    mid_price + half_spread + dist
                }
            }
            _ => unreachable!(),
        };

        commands.push(shared::MatcherCommand::new_limit_order(
            if is_bid {
                OrderSide::Bid
            } else {
                OrderSide::Ask
            },
            price as u64,
            qty,
        ));
    }
    commands
}

fn generate_synthetic_commands_2(total_orders: usize) -> Vec<MatcherCommand> {
    let mut commands = Vec::with_capacity(total_orders);
    let mut rng = rand::rng();

    let mid_price: i64 = 10_000;
    let half_spread: i64 = 1;

    // let type_distr = WeightedIndex::new([60, 30, 10]).unwrap();
    let type_distr = WeightedIndex::new([5, 90, 5]).unwrap();
    // let type_distr = WeightedIndex::new([90, 5, 5]).unwrap();

    // 50% bids, 50% asks
    let side_distr = Bernoulli::new(0.50).unwrap();

    let qty_distr = LogNormal::new(3.4, 0.9).unwrap();

    let passive_distr = Exp::new(0.4).unwrap();

    let marketable_distr = Uniform::new(0, 4).unwrap();

    let far_distr = Uniform::new(20, 80).unwrap();

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    for _ in 0..total_orders {
        let is_bid = side_distr.sample(&mut rng);
        let qty = (qty_distr.sample(&mut rng) as f64).max(1.0).round() as u32;
        let order_type = type_distr.sample(&mut rng);

        let price: i64 = match order_type {
            0 => {
                let dist = ((passive_distr.sample(&mut rng) as f64).round() as i64).clamp(0, 50);

                if is_bid {
                    mid_price - half_spread - dist
                } else {
                    mid_price + half_spread + dist
                }
            }
            1 => {
                let aggression = marketable_distr.sample(&mut rng);
                if is_bid {
                    mid_price + half_spread + aggression
                } else {
                    mid_price - half_spread - aggression
                }
            }
            2 => {
                let dist = far_distr.sample(&mut rng);
                if is_bid {
                    mid_price - half_spread - dist
                } else {
                    mid_price + half_spread + dist
                }
            }
            _ => unreachable!(),
        };

        commands.push(MatcherCommand::new_limit_order(
            if is_bid {
                OrderSide::Bid
            } else {
                OrderSide::Ask
            },
            price as u32,
            qty,
        ));
    }
    commands
}
