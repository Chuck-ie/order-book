use shared::{MatcherCommand, OrderMatcherExt, OrderSide, ob_slot_map_unsafe::OrderMatcher};
use std::hint::black_box;

fn main() {
    let iterations = 100_000_000;
    let mut matcher = OrderMatcher::new();
    let mut price = 100u32;

    for i in 0..iterations {
        price = price.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);

        let limit = 95 + (price % 10);

        let side = if i & 1 == 0 {
            OrderSide::Bid
        } else {
            OrderSide::Ask
        };

        matcher.process(black_box(MatcherCommand::new_limit_order(side, limit, 1)));
    }

    black_box(matcher);
}
