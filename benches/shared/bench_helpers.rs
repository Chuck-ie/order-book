use order_book::common::{MatcherCommand, OrderSide};
use rand_distr::{Bernoulli, Distribution, Exp, LogNormal, Uniform, weighted::WeightedIndex};

use crate::shared::SyntheticOrder;

#[derive(Debug, Clone)]
pub struct OrderProfile {
    /// percentag of order types e.g. [70, 20, 10] means 70% of type(0), 20% of type(1) and 10% of type(2)
    /// 0 = aggressive orders that are over the opposite sides best price, meaning they execute immedietly
    /// 1 = passive orders that are close to the markets mid price, meaning they don't execute immedietly
    /// 2 = far passive orders, same as passive orders but far out
    pub type_weights: [usize; 3],

    /// probability that any given order is a bid e.g. 0.5 = 50% bid/50% ask split.
    pub bid_probability: f64,

    /// passiveness of a type(0) orders
    /// rate parameter λ for the Exp distribution that controls how far type(1) and type(2) orders sit from the mid price.
    /// small λ → spread out book
    /// large λ → orders cluster near the spread
    /// see ``rand_distr::Exp``
    pub passive_lambda: f64,

    /// the lo and hi combined define how many price levels the order will spread
    /// a higher difference means there are more price levels in total
    /// hi - lo needs to be > 0
    pub far_range: Range,

    /// upper bound for how aggressive marketable orders can get. higher values means orders can
    /// reach deeper into the other side, causing more sweeps and therefore stresstesting the
    /// matching engine itself more since there is a less likely chance of the marketable order having
    /// to be interted into the book and a more likely chance of more orders needing to be canceled
    // pub aggression_hi: i64,
    pub aggression_range: Range,

    /// the chance an order will be canceled right after it was inserted. For pure cancel speed testing,
    /// orders can be inserted only on one side of the book so that they dont match with any orders
    /// from the other side, which would cause the order to be filled and therefore not made cancelable
    pub cancel_ratio: f64,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub lo: u32,
    pub hi: u32,
}

impl Range {
    #[must_use]
    pub const fn new(lo: u32, hi: u32) -> Self {
        Self { lo, hi }
    }
}

impl OrderProfile {
    #[must_use]
    pub const fn place_narrow() -> Self {
        Self {
            type_weights: [20, 80, 0],
            bid_probability: 0.5,
            passive_lambda: 10.0,
            far_range: Range::new(0, 1),
            aggression_range: Range::new(0, 5),
            cancel_ratio: 0.0,
        }
    }

    #[must_use]
    pub const fn place_wide() -> Self {
        Self {
            type_weights: [10, 30, 60],
            bid_probability: 0.5,
            passive_lambda: 1.0,
            far_range: Range::new(50, 500),
            aggression_range: Range::new(0, 10),
            cancel_ratio: 0.0,
        }
    }
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn generate_synthetic_orders(
    profile: &OrderProfile,
    orders_to_generate: usize,
) -> Vec<SyntheticOrder> {
    let mut orders = Vec::with_capacity(orders_to_generate);
    let mut rng = rand::rng();

    let mid_price: i64 = 10_000;
    let half_spread: i64 = 1;

    let side_distr = Bernoulli::new(profile.bid_probability).unwrap();
    let type_distr = WeightedIndex::new(profile.type_weights).unwrap();

    // order amount with semi realistic distribution
    // lower mu = higher percentag of smaller orders
    // lower sigma = average small order closer to 0
    let qty_distr = LogNormal::new(3.4, 0.9).unwrap();

    let passive_distr = Exp::new(profile.passive_lambda).unwrap();

    // aggressiveness of a type (0) order, from 0 (mid price +- half_spread) to
    let marketable_distr = Uniform::new(
        i64::from(profile.aggression_range.lo),
        i64::from(profile.aggression_range.hi),
    )
    .unwrap();

    let far_distr = Uniform::new(
        i64::from(profile.far_range.lo),
        i64::from(profile.far_range.hi),
    )
    .unwrap();

    #[allow(clippy::cast_sign_loss)]
    #[allow(clippy::cast_possible_truncation)]
    for _ in 0..orders_to_generate {
        let is_bid = side_distr.sample(&mut rng);
        let order_type = type_distr.sample(&mut rng);
        let qty = (qty_distr.sample(&mut rng) as f64).max(1.0).round() as u32;

        let price: i64 = match order_type {
            0 => {
                let aggression = marketable_distr.sample(&mut rng);
                if is_bid {
                    mid_price + half_spread + aggression
                } else {
                    mid_price - half_spread - aggression
                }
            }
            1 => {
                let dist = ((passive_distr.sample(&mut rng) as f64).round() as i64).clamp(0, 50);
                if is_bid {
                    mid_price - half_spread - dist
                } else {
                    mid_price + half_spread + dist
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
            limit: price as u64,
            amount: u64::from(qty),
        });
    }

    orders
}

#[rustfmt::skip]
pub fn setup_bench_place_orders_2<Order, OrderId>(order_profile: &OrderProfile, orders_to_generate: usize) -> Vec<MatcherCommand<Order, OrderId>>
where
    MatcherCommand<Order, OrderId>: From<SyntheticOrder>,
    Order: Clone,
    OrderId: Clone,
{
    generate_synthetic_orders(order_profile, orders_to_generate)
        .into_iter()
        .map(std::convert::Into::into)
        .collect()
}

#[must_use]
pub fn convert_orders_to_commands<Order, OrderId>(
    orders: Vec<SyntheticOrder>,
) -> Vec<MatcherCommand<Order, OrderId>>
where
    MatcherCommand<Order, OrderId>: From<SyntheticOrder>,
    Order: Clone,
    OrderId: Clone,
{
    orders.into_iter().map(std::convert::Into::into).collect()
}
