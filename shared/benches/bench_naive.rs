use divan::{AllocProfiler, counter::ItemsCount};
use mimalloc::MiMalloc;
use rand_distr::{Bernoulli, Distribution, Exp, LogNormal, Uniform, weighted::WeightedIndex};
use shared::{
    MatcherCommand, OrderMatcherExt, OrderSide,
    final_ver::{self, order_book::LimitOrder},
    ob_naive, ob_slot_map_optimized,
};

#[global_allocator]
static ALLOC: AllocProfiler<MiMalloc> = AllocProfiler::new(MiMalloc);

#[allow(clippy::cast_precision_loss)]
fn main() {
    divan::main();
}

#[divan::bench(sample_count = 100, args = [1_000_000])]
fn place_orders_cold_start(bencher: divan::Bencher, total_orders: usize) {
    bencher
        .with_inputs(|| {
            (
                ob_naive::OrderMatcher::new(),
                // ob_slot_map_optimized::OrderMatcher::new(),
                // generate_synthetic_orders(total_orders, (5, 90, 5))
                generate_synthetic_orders(total_orders, (60, 30, 10))
                    .into_iter()
                    .map(SyntheticOrder::into_shared_command::<u64>)
                    .collect::<Vec<shared::MatcherCommand<u64>>>(),
            )
        })
        .input_counter(move |_| divan::counter::ItemsCount::new(total_orders))
        .bench_local_values(|(mut matcher, commands)| {
            for cmd in commands {
                matcher.process(divan::black_box(cmd));
            }
        });
}

fn setup_steady_start<M: OrderMatcherExt<OrderId = u64>>(
    initial_orders: usize,
    benched_orders: usize,
    type_arg: (usize, usize, usize),
) -> (M, Vec<shared::MatcherCommand<u64>>) {
    let mut matcher = M::new();
    let mut commands = generate_synthetic_orders(initial_orders + benched_orders, type_arg)
        .into_iter()
        .map(SyntheticOrder::into_shared_command::<u64>)
        .collect::<Vec<shared::MatcherCommand<u64>>>();

    for cmd in commands.drain(0..initial_orders) {
        matcher.process(divan::black_box(cmd));
    }

    (matcher, commands)
}

fn setup_steady_start_final_ver(
    arena: &mut final_ver::arena_slot_allocator::ArenaSlotAllocator<LimitOrder>,
    initial_orders: usize,
    benched_orders: usize,
    type_arg: (usize, usize, usize),
) -> (
    final_ver::order_matcher::OrderMatcher,
    Vec<final_ver::order_matcher::MatcherCommand>,
) {
    let mut matcher = shared::final_ver::order_matcher::OrderMatcher::new();
    let mut commands = generate_synthetic_orders(initial_orders + benched_orders, type_arg)
        .into_iter()
        .map(SyntheticOrder::into_final_command)
        .collect::<Vec<final_ver::order_matcher::MatcherCommand>>();

    for cmd in commands.drain(0..initial_orders) {
        matcher.process(divan::black_box(cmd), arena);
    }

    (matcher, commands)
}

#[divan::bench(sample_count = 100, args = [1_000_000])]
fn place_orders_steady_start(bencher: divan::Bencher, total_orders: usize) {
    bencher
        .with_inputs(|| {
            setup_steady_start::<ob_naive::OrderMatcher>(
                total_orders,
                total_orders / 10,
                (60, 30, 10),
            )
        })
        .input_counter(move |_| ItemsCount::new(total_orders / 10))
        .bench_local_values(|(mut matcher, commands)| {
            for cmd in commands {
                matcher.process(divan::black_box(cmd));
            }
        });

    // bencher
    //     .with_inputs(|| {
    //         let mut matcher = ob_naive::OrderMatcher::new();
    //         let mut commands =
    //             // generate_synthetic_orders(total_orders + (total_orders / 10), (5, 90, 5))
    //             generate_synthetic_orders(total_orders + (total_orders / 10), (60, 30, 10))
    //                 .into_iter()
    //                 .map(SyntheticOrder::into_shared_command::<u64>)
    //                 .collect::<Vec<shared::MatcherCommand<u64>>>();
    //
    //         for cmd in commands.drain(0..total_orders) {
    //             matcher.process(divan::black_box(cmd));
    //         }
    //
    //         (matcher, commands)
    //     })
    //     .input_counter(move |_| divan::counter::ItemsCount::new(total_orders / 10))
    //     .bench_local_values(|(mut matcher, commands)| {
    //         for cmd in commands {
    //             matcher.process(divan::black_box(cmd));
    //         }
    //     });
}

struct SyntheticOrder {
    pub side: OrderSide,
    pub price: u32,
    pub qty: u32,
}

impl SyntheticOrder {
    fn into_shared_command<ID>(self) -> shared::MatcherCommand<ID> {
        shared::MatcherCommand::new_limit_order(self.side, self.price.into(), self.qty.into())
    }

    const fn into_final_command(self) -> shared::final_ver::order_matcher::MatcherCommand {
        shared::final_ver::order_matcher::MatcherCommand::new_limit_order(
            self.side, self.price, self.qty,
        )
    }
}

fn generate_synthetic_orders(
    total_orders: usize,
    type_arg: (usize, usize, usize),
) -> Vec<SyntheticOrder> {
    let mut orders = Vec::with_capacity(total_orders);
    let mut rng = rand::rng();

    let mid_price: i64 = 10_000;
    let half_spread: i64 = 1;

    // let type_distr = WeightedIndex::new([60, 30, 10]).unwrap();
    let type_distr = WeightedIndex::new([type_arg.0, type_arg.1, type_arg.2]).unwrap();
    // let type_distr = WeightedIndex::new([0, 100, 0]).unwrap();

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

        orders.push(SyntheticOrder {
            side: if is_bid {
                OrderSide::Bid
            } else {
                OrderSide::Ask
            },
            price: price as u32,
            qty,
        });
    }

    orders
}
