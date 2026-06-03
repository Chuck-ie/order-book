#[cfg(test)]
mod final_ob_tests {
    use order_book::{
        arena_allocator::{ArenaAllocator, ArenaId},
        common::{MatcherCommand, OrderSide},
        engine::{
            LimitOrder,
            arena_order_matcher::{ArenaOrderMatcher, ArenaOrderMatcherExt},
            v4_sm_arena, v5_sm_arena_vec_index,
        },
    };

    fn new_limit_order(
        side: OrderSide,
        price: u32,
        amount: u32,
    ) -> MatcherCommand<LimitOrder, ArenaId> {
        MatcherCommand::PlaceOrder(LimitOrder {
            limit: price,
            amount,
            side,
        })
    }

    fn new_arena_matcher<M: ArenaOrderMatcherExt>() -> ArenaOrderMatcher<M> {
        ArenaOrderMatcher {
            arena: ArenaAllocator::new(16, 16),
            matcher: M::new(),
        }
    }

    macro_rules! test_arena_matcher_impl {
        ($name:ident, $matcher:ty) => {
            mod $name {
                use super::*;

                #[test]
                fn unresolved_spread_test() {
                    super::unresolved_spread::<$matcher>();
                }

                #[test]
                fn full_fill_order_test() {
                    super::full_fill_order::<$matcher>();
                }

                #[test]
                fn partial_fill_order_test() {
                    super::partial_fill_order::<$matcher>();
                }

                #[test]
                fn full_sweep_orders_test() {
                    super::full_sweep_orders::<$matcher>();
                }

                #[test]
                fn partial_sweep_orders_test() {
                    super::partial_sweep_orders::<$matcher>();
                }

                #[test]
                fn highest_price_first_test() {
                    super::highest_price_first::<$matcher>();
                }

                #[test]
                fn orders_fifo_sorted_test() {
                    super::orders_fifo_sorted::<$matcher>();
                }

                #[test]
                fn cancel_head_keeps_fifo_sorted_test() {
                    super::cancel_head_keeps_fifo_sorted::<$matcher>();
                }

                #[test]
                fn cancel_middle_keeps_fifo_sorted_test() {
                    super::cancel_middle_keeps_fifo_sorted::<$matcher>();
                }
            }
        };
    }

    test_arena_matcher_impl!(test_v4_sm_arena, v4_sm_arena::matcher::OrderMatcher);
    test_arena_matcher_impl!(
        test_v5_sm_arena_vec_index,
        v5_sm_arena_vec_index::matcher::OrderMatcher
    );

    pub fn unresolved_spread<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        wrapper.process(new_limit_order(OrderSide::Bid, 10, 1));
        wrapper.process(new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(10), wrapper.matcher.best_bid());
        assert_eq!(Some(100), wrapper.matcher.best_ask());
        assert_eq!(1, wrapper.total_volume_at(OrderSide::Bid, 10));
        assert_eq!(1, wrapper.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn full_fill_order<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));
        wrapper.process(new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(None, wrapper.matcher.best_bid());
        assert_eq!(None, wrapper.matcher.best_ask());
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn partial_fill_order<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        wrapper.process(new_limit_order(OrderSide::Bid, 100, 2));
        wrapper.process(new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(100), wrapper.matcher.best_bid());
        assert_eq!(None, wrapper.matcher.best_ask());
        assert_eq!(1, wrapper.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn full_sweep_orders<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        wrapper.process(new_limit_order(OrderSide::Bid, 100, 10));
        wrapper.process(new_limit_order(OrderSide::Bid, 101, 10));
        wrapper.process(new_limit_order(OrderSide::Bid, 102, 10));
        wrapper.process(new_limit_order(OrderSide::Ask, 90, 30));

        assert_eq!(None, wrapper.matcher.best_bid());
        assert_eq!(None, wrapper.matcher.best_ask());
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Bid, 101));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Bid, 102));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn partial_sweep_orders<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        wrapper.process(new_limit_order(OrderSide::Bid, 100, 1000));
        wrapper.process(new_limit_order(OrderSide::Ask, 100, 10));

        assert_eq!(Some(100), wrapper.matcher.best_bid());
        assert_eq!(None, wrapper.matcher.best_ask());
        assert_eq!(990, wrapper.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn highest_price_first<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        wrapper.process(new_limit_order(OrderSide::Bid, 110, 1));
        wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));
        wrapper.process(new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(100), wrapper.matcher.best_bid());
        assert_eq!(None, wrapper.matcher.best_ask());
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Bid, 110));
        assert_eq!(1, wrapper.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn orders_fifo_sorted<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        let first_bid_id = wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));
        wrapper.process(new_limit_order(OrderSide::Ask, 100, 1));

        let first_bid = wrapper.get_order(&first_bid_id.unwrap());
        assert!(first_bid.is_none());

        let last_bid = wrapper.get_order(&last_bid_id.unwrap());
        assert!(last_bid.is_some());

        assert_eq!(1, wrapper.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn cancel_head_keeps_fifo_sorted<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        let first_bid_id = wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));

        wrapper.process(MatcherCommand::CancelOrder(first_bid_id.clone().unwrap()));
        wrapper.process(new_limit_order(OrderSide::Ask, 100, 1));

        let first_bid = wrapper.get_order(&first_bid_id.unwrap());
        assert!(first_bid.is_none());

        let last_bid = wrapper.get_order(&last_bid_id.unwrap());
        assert!(last_bid.is_none());

        assert_eq!(0, wrapper.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn cancel_middle_keeps_fifo_sorted<M: ArenaOrderMatcherExt>() {
        let mut wrapper = new_arena_matcher::<M>();
        let first_bid_id = wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));
        let mid_bid_id = wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = wrapper.process(new_limit_order(OrderSide::Bid, 100, 1));

        wrapper.process(MatcherCommand::CancelOrder(mid_bid_id.clone().unwrap()));
        wrapper.process(new_limit_order(OrderSide::Ask, 100, 2));

        let first_bid = wrapper.get_order(&first_bid_id.unwrap());
        assert!(first_bid.is_none());

        let mid_bid = wrapper.get_order(&mid_bid_id.unwrap());
        assert!(mid_bid.is_none());

        let last_bid = wrapper.get_order(&last_bid_id.unwrap());
        assert!(last_bid.is_none());

        assert_eq!(0, wrapper.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, wrapper.total_volume_at(OrderSide::Ask, 100));
    }
}
