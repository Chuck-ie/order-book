use std::{
    cell::{LazyCell, UnsafeCell},
    thread::LocalKey,
};

use order_book::{
    arena_allocator::{ArenaAllocator, ArenaId},
    common::{LimitOrderRequest, MatcherCommand, OrderMatcherExt, OrderSide},
    engine::v4_slot_map_arena::{self, LimitOrder},
    slot_map::chunked::ArenaSlot,
};
use rand_distr::{Bernoulli, Distribution, Exp, LogNormal, Uniform, weighted::WeightedIndex};

pub struct SyntheticOrder {
    pub side: OrderSide,
    pub price: u64,
    pub amount: u64,
}

impl SyntheticOrder {
    #[must_use]
    pub const fn into_order_request<OrderId>(self) -> MatcherCommand<LimitOrderRequest, OrderId> {
        MatcherCommand::PlaceOrder(LimitOrderRequest {
            side: self.side,
            limit: self.price,
            amount: self.amount,
        })
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub const fn into_limit_order(self) -> MatcherCommand<LimitOrder, ArenaId> {
        MatcherCommand::PlaceOrder(LimitOrder {
            side: self.side,
            limit: self.price as u32,
            amount: self.amount as u32,
        })
    }
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn generate_synthetic_orders(
    total_orders: usize,
    profile: &OrderProfile,
) -> Vec<SyntheticOrder> {
    let mut orders = Vec::with_capacity(total_orders);
    let mut rng = rand::rng();

    let mid_price: i64 = 10_000;
    let half_spread: i64 = 1;

    let type_distr = WeightedIndex::new(profile.type_weights).unwrap();
    let side_distr = Bernoulli::new(profile.bid_prob).unwrap();
    let qty_distr = LogNormal::new(profile.qty_mu, profile.qty_sigma).unwrap();
    let passive_distr = Exp::new(profile.passive_lambda).unwrap();
    let marketable_distr = Uniform::new(0_i64, profile.aggression_hi).unwrap();
    let far_distr =
        Uniform::new(profile.far_near_split as i64, profile.far_range_hi as i64).unwrap();

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
            price: price as u64,
            amount: u64::from(qty),
        });
    }

    orders
}

// #[must_use]
// #[allow(clippy::missing_panics_doc)]
// pub fn generate_synthetic_orders(
//     total_orders: usize,
//     order_profile: (usize, usize, usize),
// ) -> Vec<SyntheticOrder> {
//     let mut orders = Vec::with_capacity(total_orders);
//     let mut rng = rand::rng();
//
//     let mid_price: i64 = 10_000;
//     let half_spread: i64 = 1;
//
//     // let type_distr = WeightedIndex::new([60, 30, 10]).unwrap();
//     let type_distr =
//         WeightedIndex::new([order_profile.0, order_profile.1, order_profile.2]).unwrap();
//     // let type_distr = WeightedIndex::new([0, 100, 0]).unwrap();
//
//     // 50% bids, 50% asks
//     let side_distr = Bernoulli::new(0.50).unwrap();
//
//     let qty_distr = LogNormal::new(3.4, 0.9).unwrap();
//     // let qty_distr = LogNormal::new(5.0, 2.5).unwrap();
//
//     // let passive_distr = Exp::new(0.4).unwrap();
//     let passive_distr = Exp::new(0.05).unwrap();
//
//     let marketable_distr = Uniform::new(0, 30).unwrap();
//
//     // let far_distr = Uniform::new(200, 800).unwrap();
//     let far_distr = Uniform::new(51, 800).unwrap();
//
//     #[allow(clippy::cast_sign_loss)]
//     #[allow(clippy::cast_possible_truncation)]
//     for _ in 0..total_orders {
//         let is_bid = side_distr.sample(&mut rng);
//         let qty = (qty_distr.sample(&mut rng) as f64).max(1.0).round() as u32;
//         let order_type = type_distr.sample(&mut rng);
//
//         let price: i64 = match order_type {
//             0 => {
//                 let dist = ((passive_distr.sample(&mut rng) as f64).round() as i64).clamp(0, 50);
//
//                 if is_bid {
//                     mid_price - half_spread - dist
//                 } else {
//                     mid_price + half_spread + dist
//                 }
//             }
//             1 => {
//                 let aggression = marketable_distr.sample(&mut rng);
//                 if is_bid {
//                     mid_price + half_spread + aggression
//                 } else {
//                     mid_price - half_spread - aggression
//                 }
//             }
//             2 => {
//                 let dist = far_distr.sample(&mut rng);
//                 if is_bid {
//                     mid_price - half_spread - dist
//                 } else {
//                     mid_price + half_spread + dist
//                 }
//             }
//             _ => unreachable!(),
//         };
//
//         orders.push(SyntheticOrder {
//             side: if is_bid {
//                 OrderSide::Bid
//             } else {
//                 OrderSide::Ask
//             },
//             price: price as u64,
//             amount: u64::from(qty),
//         });
//     }
//
//     orders
// }

pub type BenchStateKey<S> = &'static LocalKey<UnsafeCell<LazyCell<S>>>;

pub trait BenchState {
    type Order;
    type OrderId;

    fn process(&mut self, cmd: MatcherCommand<Self::Order, Self::OrderId>);
    fn generate_input(
        &self,
        per_batch_orders: usize,
        order_profile: &OrderProfile,
    ) -> Vec<MatcherCommand<Self::Order, Self::OrderId>>;
}

#[derive(Default)]
pub struct DefaultBenchState<Engine: Default + OrderMatcherExt> {
    engine: Engine,
}

impl<Engine: Default + OrderMatcherExt> BenchState for DefaultBenchState<Engine> {
    type Order = LimitOrderRequest;
    type OrderId = Engine::OrderId;

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn process(&mut self, cmd: MatcherCommand<Self::Order, Self::OrderId>) {
        self.engine.process(cmd);
    }

    fn generate_input(
        &self,
        per_batch_orders: usize,
        order_profile: &OrderProfile,
    ) -> Vec<MatcherCommand<Self::Order, Self::OrderId>> {
        generate_synthetic_orders(per_batch_orders, order_profile)
            .into_iter()
            .map(SyntheticOrder::into_order_request)
            .collect::<Vec<_>>()
    }
}

pub struct ArenaBenchState<Engine: Default> {
    engine: Engine,
    arena: ArenaAllocator<ArenaSlot<LimitOrder>>,
}

impl<Engine: Default> Default for ArenaBenchState<Engine> {
    fn default() -> Self {
        Self {
            engine: Engine::default(),
            arena: ArenaAllocator::new(16384, 4096),
        }
    }
}

impl BenchState for ArenaBenchState<v4_slot_map_arena::matcher::OrderMatcher> {
    type Order = LimitOrder;
    type OrderId = ArenaId;

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn process(&mut self, cmd: MatcherCommand<Self::Order, Self::OrderId>) {
        self.engine.process(cmd, &mut self.arena);
    }

    fn generate_input(
        &self,
        per_batch_orders: usize,
        order_profile: &OrderProfile,
    ) -> Vec<MatcherCommand<Self::Order, Self::OrderId>> {
        generate_synthetic_orders(per_batch_orders, order_profile)
            .into_iter()
            .map(SyntheticOrder::into_limit_order)
            .collect::<Vec<_>>()
    }
}

#[derive(Debug, Clone)]
pub struct OrderProfile {
    /// Relative weights for [passive_limit, aggressive_market, far_out_resting].
    /// Treated as a `WeightedIndex`; values need not sum to 100.
    pub type_weights: [usize; 3],

    /// Probability that any given *new* order is a bid.  0.5 = balanced book.
    pub bid_prob: f64,

    /// Rate parameter λ for the Exp distribution that controls how far passive limit orders sit from mid.
    /// Small λ → spread-out book
    /// Large λ → orders cluster near the spread
    pub passive_lambda: f64,

    /// Upper bound (exclusive) of `Uniform(far_near_split, far_range_hi)` used
    /// for far-out resting orders.  This directly caps the number of distinct
    /// price levels that far-out orders can occupy.
    pub far_range_hi: u32,

    /// Lower bound of the far-out uniform range.  Keep ≥ 51 to avoid overlap
    /// with the passive zone.
    pub far_near_split: u32,

    /// Upper aggression bound for marketable orders: `Uniform(0, aggression_hi)`.
    /// Larger values → orders sweep deeper into the opposing side.
    pub aggression_hi: i64,

    /// (μ, σ) for the LogNormal quantity distribution.
    /// μ=3.4, σ=0.9 gives a realistic heavy-tailed size distribution.
    pub qty_mu: f64,
    pub qty_sigma: f64,

    /// Fraction of processed orders that should be cancel commands.
    /// `0.0` = no cancels (pure placement).  `0.7` = 70 % of the processed
    /// work is cancel commands.
    ///
    /// Because cancel IDs are only known after a successful placement, cancels
    /// cannot be pre-generated.  Profiles with `cancel_ratio > 0` must be run
    /// via the interleaved bench path (`run_bench_interleaved`), not the
    /// pre-generation path.
    pub cancel_ratio: f64,
}

impl OrderProfile {
    // ------------------------------------------------------------------
    // Original three profiles — semantics unchanged, now fully explicit.
    // ------------------------------------------------------------------

    /// p1 — mostly passive, light aggression, very few far-out orders.
    /// Builds a moderately deep book; the dominant workload is limit placement
    /// near the spread.
    #[must_use]
    pub const fn p1() -> Self {
        Self {
            type_weights: [75, 20, 5],
            bid_prob: 0.50,
            passive_lambda: 0.05,
            far_range_hi: 800,
            far_near_split: 51,
            aggression_hi: 30,
            qty_mu: 3.4,
            qty_sigma: 0.9,
            cancel_ratio: 0.0,
        }
    }

    /// p2 — mostly aggressive (market orders), very few resting limits.
    /// The book stays shallow; the dominant workload is match-and-consume.
    #[must_use]
    pub const fn p2() -> Self {
        Self {
            type_weights: [20, 75, 5],
            bid_prob: 0.50,
            passive_lambda: 0.05,
            far_range_hi: 800,
            far_near_split: 51,
            aggression_hi: 30,
            qty_mu: 3.4,
            qty_sigma: 0.9,
            cancel_ratio: 0.0,
        }
    }

    /// p3 — balanced mix, elevated far-out resting share.
    /// Builds a wide book with many occupied price levels; stresses index
    /// cardinality in BTree and slot-map implementations.
    #[must_use]
    pub const fn p3() -> Self {
        Self {
            type_weights: [40, 40, 20],
            bid_prob: 0.50,
            passive_lambda: 0.05,
            far_range_hi: 800,
            far_near_split: 51,
            aggression_hi: 30,
            qty_mu: 3.4,
            qty_sigma: 0.9,
            cancel_ratio: 0.0,
        }
    }

    /// p4 — deep resting book.
    ///
    /// Almost exclusively passive limit orders placed with very low λ, so they
    /// scatter widely and occupy hundreds of distinct price levels.  The book
    /// grows continuously with no matching pressure to drain it.
    ///
    /// **Stresses:** per-level allocation cost (BTree node allocs, slot-map
    /// growth), v1 vec memory spikes, v4 arena pre-allocation adequacy.
    #[must_use]
    pub const fn p4() -> Self {
        Self {
            type_weights: [90, 5, 5],
            bid_prob: 0.50,
            passive_lambda: 0.01,
            far_range_hi: 800,
            far_near_split: 51,
            aggression_hi: 10,
            qty_mu: 3.4,
            qty_sigma: 0.9,
            cancel_ratio: 0.0,
        }
    }

    /// p5 — lopsided aggression.
    ///
    /// 90 % market orders, 75 % on the bid side.  The ask book is continuously
    /// swept; the bid book grows lopsided.  Repeated full-level sweeps with
    /// wide aggression expose worst-case match traversal.
    ///
    /// **Stresses:** BTree in-order traversal on repeated sweep, v1 vec linear
    /// scan cost, match-loop throughput across all implementations.
    #[must_use]
    pub const fn p5() -> Self {
        Self {
            type_weights: [5, 90, 5],
            bid_prob: 0.75,
            passive_lambda: 0.05,
            far_range_hi: 800,
            far_near_split: 51,
            aggression_hi: 80,
            qty_mu: 3.4,
            qty_sigma: 0.9,
            cancel_ratio: 0.0,
        }
    }

    /// p6 — high cancel churn.
    ///
    /// 65 % of processed work is cancel commands against live resting orders.
    /// Price levels are deliberately narrow (≤ 20 distinct levels) so the book
    /// itself stays small; the stress is entirely on the alloc → dealloc cycle.
    ///
    /// **Stresses:** slot-map slot recycling (v3), arena slot reuse (v4),
    /// v1 vec erase/shift cost on cancel.
    ///
    /// Requires the interleaved bench path.
    #[must_use]
    pub const fn p6() -> Self {
        Self {
            type_weights: [70, 5, 25],
            bid_prob: 0.50,
            passive_lambda: 0.5,
            far_range_hi: 71,
            far_near_split: 51,
            aggression_hi: 5,
            qty_mu: 3.4,
            qty_sigma: 0.9,
            cancel_ratio: 0.65,
        }
    }

    /// p7 — narrow book, high frequency.
    ///
    /// All orders confined to ≈ 5 active price levels (high passive_lambda
    /// clusters everything near mid; far zone is only 4 levels wide).
    /// 30 % cancel ratio so slots at those levels churn constantly.
    ///
    /// **Stresses:** per-level amortisation — implementations that cache or
    /// reuse level structures win; those that allocate fresh on every visit
    /// pay the full cost repeatedly.  Directly tests v4 arena reuse on a hot
    /// working set.
    ///
    /// Requires the interleaved bench path.
    #[must_use]
    pub const fn p7() -> Self {
        Self {
            type_weights: [55, 40, 5],
            bid_prob: 0.50,
            passive_lambda: 2.0,
            far_range_hi: 55,
            far_near_split: 51,
            aggression_hi: 15,
            qty_mu: 3.4,
            qty_sigma: 0.9,
            cancel_ratio: 0.30,
        }
    }
}
