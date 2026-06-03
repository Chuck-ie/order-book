#[cfg(test)]
mod tests {
    use order_book::{
        common::{LimitOrderRequest, MatcherCommand, OrderBookExt, OrderMatcherExt, OrderSide},
        engine::{v1_vec_only, v2_btree, v3_slot_map},
    };

    macro_rules! test_order_matcher_impl {
        ($name:ident, $type:ty) => {
            mod $name {
                use super::*;

                #[test]
                fn unresolved_spread_test() {
                    super::unresolved_spread::<$type>();
                }

                #[test]
                fn full_fill_order_test() {
                    super::full_fill_order::<$type>();
                }

                #[test]
                fn partial_fill_order_test() {
                    super::partial_fill_order::<$type>();
                }

                #[test]
                fn full_sweep_orders_test() {
                    super::full_sweep_orders::<$type>();
                }

                #[test]
                fn partial_sweep_orders_test() {
                    super::partial_sweep_orders::<$type>();
                }

                #[test]
                fn highest_price_first_test() {
                    super::highest_price_first::<$type>();
                }

                #[test]
                fn orders_fifo_sorted_test() {
                    super::orders_fifo_sorted::<$type>();
                }

                #[test]
                fn cancel_head_keeps_fifo_sorted_test() {
                    super::cancel_head_keeps_fifo_sorted::<$type>();
                }

                #[test]
                fn cancel_middle_keeps_fifo_sorted() {
                    super::cancel_middle_keeps_fifo_sorted::<$type>();
                }
            }
        };
    }

    test_order_matcher_impl!(test_v1_vec_only, v1_vec_only::matcher::OrderMatcher);
    test_order_matcher_impl!(test_v2_btree, v2_btree::matcher::OrderMatcher);
    test_order_matcher_impl!(test_v3_slot_map, v3_slot_map::matcher::OrderMatcher);

    fn new_limit_order<OrderId: Clone>(
        side: OrderSide,
        price: u64,
        amount: u64,
    ) -> MatcherCommand<LimitOrderRequest, OrderId> {
        MatcherCommand::PlaceOrder(LimitOrderRequest {
            side,
            limit: price,
            amount,
        })
    }

    pub fn unresolved_spread<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        matcher.process(new_limit_order(OrderSide::Bid, 10, 1));
        matcher.process(new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(10), matcher.best_bid());
        assert_eq!(Some(100), matcher.best_ask());
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 10));
        assert_eq!(1, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn full_fill_order<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        matcher.process(new_limit_order(OrderSide::Bid, 100, 1));
        matcher.process(new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(None, matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn partial_fill_order<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        matcher.process(new_limit_order(OrderSide::Bid, 100, 2));
        matcher.process(new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn full_sweep_orders<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        matcher.process(new_limit_order(OrderSide::Bid, 100, 10));
        matcher.process(new_limit_order(OrderSide::Bid, 101, 10));
        matcher.process(new_limit_order(OrderSide::Bid, 102, 10));
        matcher.process(new_limit_order(OrderSide::Ask, 90, 30));

        assert_eq!(None, matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 101));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 102));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn partial_sweep_orders<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        matcher.process(new_limit_order(OrderSide::Bid, 100, 1000));
        matcher.process(new_limit_order(OrderSide::Ask, 100, 10));

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(990, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn highest_price_first<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        matcher.process(new_limit_order(OrderSide::Bid, 110, 1));
        matcher.process(new_limit_order(OrderSide::Bid, 100, 1));
        matcher.process(new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 110));
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn orders_fifo_sorted<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        let first_bid_id = matcher.process(new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = matcher.process(new_limit_order(OrderSide::Bid, 100, 1));
        matcher.process(new_limit_order(OrderSide::Ask, 100, 1));

        let first_bid = matcher.order_book().get_order(first_bid_id.unwrap());
        let last_bid = matcher.order_book().get_order(last_bid_id.unwrap());

        assert!(first_bid.is_none());
        assert!(last_bid.is_some());
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn cancel_head_keeps_fifo_sorted<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        let first_bid_id = matcher.process(new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = matcher.process(new_limit_order(OrderSide::Bid, 100, 1));
        matcher.process(MatcherCommand::CancelOrder(first_bid_id.unwrap()));
        matcher.process(new_limit_order(OrderSide::Ask, 100, 1));

        let first_bid = matcher.order_book().get_order(first_bid_id.unwrap());
        let last_bid = matcher.order_book().get_order(last_bid_id.unwrap());

        assert!(first_bid.is_none());
        assert!(last_bid.is_none());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    pub fn cancel_middle_keeps_fifo_sorted<M: OrderMatcherExt>() {
        let mut matcher = M::new();
        let first_bid_id = matcher.process(new_limit_order(OrderSide::Bid, 100, 1));
        let mid_bid_id = matcher.process(new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = matcher.process(new_limit_order(OrderSide::Bid, 100, 1));

        matcher.process(MatcherCommand::CancelOrder(mid_bid_id.unwrap()));
        matcher.process(new_limit_order(OrderSide::Ask, 100, 2));

        let first_bid = matcher.order_book().get_order(first_bid_id.unwrap());
        let mid_bid = matcher.order_book().get_order(mid_bid_id.unwrap());
        let last_bid = matcher.order_book().get_order(last_bid_id.unwrap());

        assert!(first_bid.is_none());
        assert!(mid_bid.is_none());
        assert!(last_bid.is_none());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }
}
