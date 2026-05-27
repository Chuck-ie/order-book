use order_book::{
    arena_allocator::ArenaId,
    common::{LimitOrderRequest, MatcherCommand, OrderIdU32, OrderMatcherExt, OrderSide},
    engine::v4_slot_map_arena::LimitOrder,
};
use rand_distr::{Bernoulli, Distribution, Exp, LogNormal, Uniform, weighted::WeightedIndex};

pub fn setup_steady_start<M: OrderMatcherExt<OrderId = OrderIdU32>>(
    initial_orders: usize,
    benched_orders: usize,
    type_arg: (usize, usize, usize),
) -> (M, Vec<MatcherCommand<LimitOrderRequest, OrderIdU32>>) {
    let mut matcher = M::new();
    let mut commands = generate_synthetic_orders(initial_orders + benched_orders, type_arg)
        .into_iter()
        .map(SyntheticOrder::into_order_request::<OrderIdU32>)
        .collect::<Vec<MatcherCommand<LimitOrderRequest, OrderIdU32>>>();

    for cmd in commands.drain(0..initial_orders) {
        matcher.process(divan::black_box(cmd));
    }

    (matcher, commands)
}

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
            price: price as u64,
            amount: u64::from(qty),
        });
    }

    orders
}
