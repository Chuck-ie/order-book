use divan::{counter::ItemsCount, Bencher};
use rand::seq::SliceRandom;
use rand_distr::{Distribution, Normal};
use shared::{
    optimized::{LimitOrder, MatcherCommand, OrderMatcher},
    OrderSide,
};
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
                    #[allow(clippy::cast_possible_truncation)]
                    #[allow(clippy::cast_sign_loss)]
                    let limit = price_dist.sample(&mut rng) as u32;

                    MatcherCommand::PlaceOrder(LimitOrder {
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
                if let MatcherCommand::PlaceOrder(order) = cmd {
                    black_box(_ = matcher.place_order(order));
                }
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

            let mut ids: Vec<u32> = Vec::with_capacity(n);

            for _ in 0..n {
                #[allow(clippy::cast_possible_truncation)]
                #[allow(clippy::cast_sign_loss)]
                let limit = price_dist.sample(&mut rng) as u32;

                let id = matcher
                    .place_order(LimitOrder {
                        book_order_id: 0,
                        side: OrderSide::Bid,
                        limit: limit.max(1),
                        amount: 1,
                    })
                    .expect("Order should be placed");

                ids.push(id);
            }

            let mut rng = rand::rng();
            ids.shuffle(&mut rng);

            let commands: Vec<_> = ids.into_iter().map(MatcherCommand::RemoveOrder).collect();
            (matcher, commands)
        })
        .bench_values(|(mut matcher, commands)| {
            let count = commands.len();

            for cmd in commands {
                if let MatcherCommand::RemoveOrder(matcher_id) = cmd {
                    black_box(() = matcher.remove_order(matcher_id));
                }
            }

            ItemsCount::new(count)
        });
}

#[divan::bench(args = [1000, 10_000, 100_000, 1_000_000])]
fn bench_match_processing_single(bencher: Bencher, n: usize) {
    bencher
        .with_inputs(|| {
            let mut matcher = OrderMatcher::new();
            matcher.place_order(LimitOrder {
                book_order_id: 0,
                side: OrderSide::Ask,
                limit: 100,
                amount: n.try_into().unwrap(),
            });

            let commands: Vec<_> = (0..n)
                .map(|_| {
                    MatcherCommand::PlaceOrder(LimitOrder {
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
                if let MatcherCommand::PlaceOrder(order) = cmd {
                    black_box(_ = matcher.place_order(order));
                }
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
                matcher.place_order(LimitOrder {
                    book_order_id: 0,
                    side: OrderSide::Ask,
                    limit: (1000 + i).try_into().unwrap(),
                    amount: 1,
                });
            }

            // One big Bid to sweep the book
            let sweep_order = MatcherCommand::PlaceOrder(LimitOrder {
                book_order_id: 0,
                side: OrderSide::Bid,
                limit: (1000 + n).try_into().unwrap(),
                amount: n.try_into().unwrap(),
            });
            (matcher, sweep_order)
        })
        .bench_values(|(mut matcher, sweep_cmd)| {
            if let MatcherCommand::PlaceOrder(order) = sweep_cmd {
                black_box(_ = matcher.place_order(order));
            }
        });
}
