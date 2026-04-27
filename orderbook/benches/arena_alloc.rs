use divan::{counter::ItemsCount, Bencher};
use rand::seq::SliceRandom;
use rand_distr::{Distribution, Normal};
use shared::{standard_arena::*, OrderSide};
use std::hint::black_box;

fn main() {
    divan::main();
}

#[divan::bench(args = [1000, 10_000, 100_000, 1_000_000])]
fn bench_insert_only(bencher: Bencher, n: usize) {
    bencher
        .with_inputs(|| {
            let matcher = OrderMatcher::new();
            let mut rng = rand::rng();
            let price_dist = Normal::new(1000.0, 50.0).unwrap();

            let commands: Vec<_> = (0..n)
                .map(|_| {
                    let limit = price_dist.sample(&mut rng) as u64;

                    OrderCommand::Place(LimitOrder {
                        book_order_id: 0,
                        side: OrderSide::Bid,
                        limit: limit.max(1),
                        amount: 1,
                    })
                })
                .collect();
            (matcher, commands)
        })
        .bench_values(|(mut matcher, commands)| {
            let count = commands.len();

            for cmd in commands {
                black_box(matcher.process(cmd));
            }

            ItemsCount::new(count)
        });
}

#[divan::bench(args = [1_000, 10_000, 100_000, 1_000_000])]
fn bench_remove_only(bencher: Bencher, n: usize) {
    bencher
        .with_inputs(|| {
            let mut matcher = OrderMatcher::new();
            let mut rng = rand::rng();
            let price_dist = Normal::new(1000.0, 50.0).unwrap();

            let mut ids: Vec<usize> = Vec::with_capacity(n);

            for _ in 0..n {
                let limit = price_dist.sample(&mut rng) as u64;

                let id = matcher
                    .process(OrderCommand::Place(LimitOrder {
                        book_order_id: 0,
                        side: OrderSide::Bid,
                        limit: limit.max(1),
                        amount: 1,
                    }))
                    .expect("Order should be placed");

                ids.push(id);
            }

            let mut rng = rand::rng();
            ids.shuffle(&mut rng);

            let commands: Vec<_> = ids.into_iter().map(OrderCommand::Remove).collect();
            (matcher, commands)
        })
        .bench_values(|(mut matcher, commands)| {
            let count = commands.len();

            for cmd in commands {
                black_box(matcher.process(cmd));
            }

            ItemsCount::new(count)
        });
}

#[divan::bench(args = [1000, 10_000, 100_000, 1_000_000])]
fn bench_match_processing_single(bencher: Bencher, n: usize) {
    bencher
        .with_inputs(|| {
            let mut matcher = OrderMatcher::new();
            // One huge Ask to be eaten by many small Bids
            matcher.process(OrderCommand::Place(LimitOrder {
                book_order_id: 0,
                side: OrderSide::Ask,
                limit: 100,
                amount: n as u64,
            }));

            let commands: Vec<_> = (0..n)
                .map(|_| {
                    OrderCommand::Place(LimitOrder {
                        book_order_id: 0,
                        side: OrderSide::Bid,
                        limit: 100,
                        amount: 1,
                    })
                })
                .collect();
            (matcher, commands)
        })
        .bench_values(|(mut matcher, commands)| {
            let count = commands.len();

            for cmd in commands {
                black_box(matcher.process(cmd));
            }

            ItemsCount::new(count)
        });
}

#[divan::bench(args = [1_000, 10_000, 100_000, 1_000_000])]
fn bench_match_processing_sweep(bencher: Bencher, n: usize) {
    bencher
        .with_inputs(|| {
            let mut matcher = OrderMatcher::new();
            // Fill book with many small Asks
            for i in 0..n {
                matcher.process(OrderCommand::Place(LimitOrder {
                    book_order_id: 0,
                    side: OrderSide::Ask,
                    limit: 1000 + i as u64,
                    amount: 1,
                }));
            }

            // One big Bid to sweep the book
            let sweep_order = OrderCommand::Place(LimitOrder {
                book_order_id: 0,
                side: OrderSide::Bid,
                limit: 1000 + n as u64,
                amount: n as u64,
            });
            (matcher, sweep_order)
        })
        .bench_values(|(mut matcher, sweep_cmd)| {
            black_box(matcher.process(sweep_cmd));
        });
}
