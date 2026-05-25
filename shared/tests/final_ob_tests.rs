#[cfg(test)]
mod final_ob_tests {
    use shared::{
        OrderSide,
        final_ver::order_matcher::{MatcherCommand, OrderMatcher},
    };

    #[test]
    pub fn unresolved_spread() {
        let mut matcher = OrderMatcher::new(16, 16);
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 10, 1));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(10), matcher.best_bid());
        assert_eq!(Some(100), matcher.best_ask());
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 10));
        assert_eq!(1, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    #[test]
    pub fn full_fill_order() {
        let mut matcher = OrderMatcher::new(16, 16);
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(None, matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    #[test]
    pub fn partial_fill_order() {
        let mut matcher = OrderMatcher::new(16, 16);
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 2));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    #[test]
    pub fn full_sweep_orders() {
        let mut matcher = OrderMatcher::new(16, 16);
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 10));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 101, 10));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 102, 10));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 90, 30));

        assert_eq!(None, matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 101));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 102));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    #[test]
    pub fn partial_sweep_orders() {
        let mut matcher = OrderMatcher::new(16, 16);
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1000));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 100, 10));

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(990, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    #[test]
    pub fn highest_price_first() {
        let mut matcher = OrderMatcher::new(16, 16);
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 110, 1));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1));

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 110));
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    #[test]
    pub fn orders_fifo_sorted() {
        let mut matcher = OrderMatcher::new(16, 16);
        let first_bid_id = matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1));

        let first_bid = matcher.order_book.get_order(first_bid_id.unwrap());
        let last_bid = matcher.order_book.get_order(last_bid_id.unwrap());

        assert!(first_bid.is_none());
        assert!(last_bid.is_some());
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    #[test]
    pub fn cancel_head_keeps_fifo_sorted() {
        let mut matcher = OrderMatcher::new(16, 16);
        let first_bid_id = matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        matcher.process(MatcherCommand::CancelOrder(first_bid_id.clone().unwrap()));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1));

        let first_bid = matcher.order_book.get_order(first_bid_id.unwrap());
        let last_bid = matcher.order_book.get_order(last_bid_id.unwrap());

        assert!(first_bid.is_none());
        assert!(last_bid.is_none());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }

    #[test]
    pub fn cancel_middle_keeps_fifo_sorted() {
        let mut matcher = OrderMatcher::new(16, 16);
        let first_bid_id = matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        let mid_bid_id = matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
        let last_bid_id = matcher.process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));

        matcher.process(MatcherCommand::CancelOrder(mid_bid_id.clone().unwrap()));
        matcher.process(MatcherCommand::new_limit_order(OrderSide::Ask, 100, 2));

        let first_bid = matcher.order_book.get_order(first_bid_id.unwrap());
        let mid_bid = matcher.order_book.get_order(mid_bid_id.unwrap());
        let last_bid = matcher.order_book.get_order(last_bid_id.unwrap());

        assert!(first_bid.is_none());
        assert!(mid_bid.is_none());
        assert!(last_bid.is_none());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100));
    }
}
