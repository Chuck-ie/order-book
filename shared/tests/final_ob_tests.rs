#[cfg(test)]
mod final_ob_tests {
    use shared::{
        OrderSide,
        final_ver::{
            arena_slot_allocator::ArenaSlotAllocator,
            order_matcher::{MatcherCommand, OrderMatcher},
        },
    };

    #[test]
    pub fn unresolved_spread() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 10, 1),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1),
            &mut arena,
        );

        assert_eq!(Some(10), matcher.best_bid());
        assert_eq!(Some(100), matcher.best_ask());
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 10, &mut arena));
        assert_eq!(1, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }

    #[test]
    pub fn full_fill_order() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1),
            &mut arena,
        );

        assert_eq!(None, matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }

    #[test]
    pub fn partial_fill_order() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 2),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1),
            &mut arena,
        );

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }

    #[test]
    pub fn full_sweep_orders() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 10),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 101, 10),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 102, 10),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 90, 30),
            &mut arena,
        );

        assert_eq!(None, matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 101, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 102, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }

    #[test]
    pub fn partial_sweep_orders() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1000),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 100, 10),
            &mut arena,
        );

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(
            990,
            matcher.total_volume_at(OrderSide::Bid, 100, &mut arena)
        );
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }

    #[test]
    pub fn highest_price_first() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 110, 1),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1),
            &mut arena,
        );

        assert_eq!(Some(100), matcher.best_bid());
        assert_eq!(None, matcher.best_ask());
        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 110, &mut arena));
        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }

    #[test]
    pub fn orders_fifo_sorted() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        let first_bid_id = matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );
        let last_bid_id = matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1),
            &mut arena,
        );

        let first_bid = matcher
            .order_book
            .get_order(&first_bid_id.unwrap(), &mut arena);
        assert!(first_bid.is_none());

        let last_bid = matcher
            .order_book
            .get_order(&last_bid_id.unwrap(), &mut arena);
        assert!(last_bid.is_some());

        assert_eq!(1, matcher.total_volume_at(OrderSide::Bid, 100, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }

    #[test]
    pub fn cancel_head_keeps_fifo_sorted() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        let first_bid_id = matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );
        let last_bid_id = matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::CancelOrder(first_bid_id.clone().unwrap()),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 100, 1),
            &mut arena,
        );

        let first_bid = matcher
            .order_book
            .get_order(&first_bid_id.unwrap(), &mut arena);
        assert!(first_bid.is_none());

        let last_bid = matcher
            .order_book
            .get_order(&last_bid_id.unwrap(), &mut arena);
        assert!(last_bid.is_none());

        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }

    #[test]
    pub fn cancel_middle_keeps_fifo_sorted() {
        let mut arena = ArenaSlotAllocator::new(16, 16);
        let mut matcher = OrderMatcher::new();
        let first_bid_id = matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );
        let mid_bid_id = matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );
        let last_bid_id = matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1),
            &mut arena,
        );

        matcher.process(
            MatcherCommand::CancelOrder(mid_bid_id.clone().unwrap()),
            &mut arena,
        );
        matcher.process(
            MatcherCommand::new_limit_order(OrderSide::Ask, 100, 2),
            &mut arena,
        );

        let first_bid = matcher
            .order_book
            .get_order(&first_bid_id.unwrap(), &mut arena);
        assert!(first_bid.is_none());

        let mid_bid = matcher
            .order_book
            .get_order(&mid_bid_id.unwrap(), &mut arena);
        assert!(mid_bid.is_none());

        let last_bid = matcher
            .order_book
            .get_order(&last_bid_id.unwrap(), &mut arena);
        assert!(last_bid.is_none());

        assert_eq!(0, matcher.total_volume_at(OrderSide::Bid, 100, &mut arena));
        assert_eq!(0, matcher.total_volume_at(OrderSide::Ask, 100, &mut arena));
    }
}
