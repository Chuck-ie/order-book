use crate::{
    LimitOrder, LimitOrderRequest, OrderBookExt, OrderMatcherExt, OrderSide, SlotMap,
    slot_map_unsafe::SlotMapUnsafe,
};
use std::{cmp::Reverse, collections::BTreeMap};

pub struct OrderBook {
    pub bids: BTreeMap<Reverse<u32>, SlotMapUnsafe<u32>>,
    pub asks: BTreeMap<u32, SlotMapUnsafe<u32>>,
    pub orders: SlotMapUnsafe<LimitOrder<u32>>,
}

impl OrderBookExt for OrderBook {
    type OrderId = u32;
    type Order = LimitOrder<Self::OrderId>;

    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: SlotMapUnsafe::new(),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        let LimitOrderRequest {
            side,
            limit: price,
            amount,
        } = request;
        let new_order = LimitOrder::new(0, side, price, amount);
        let new_order_id = self.orders.insert(new_order);
        let level_idx = match side {
            OrderSide::Bid => self
                .bids
                .entry(Reverse(price))
                .or_insert_with(|| SlotMapUnsafe::with_capacity(16_384)) // 2^14 * 16 = 262144 = 256
                .insert(new_order_id),
            OrderSide::Ask => self
                .asks
                .entry(price)
                .or_insert_with(|| SlotMapUnsafe::with_capacity(16_384)) // 2^14 * 16 = 262144 = 256
                .insert(new_order_id),
        };

        self.orders
            .get_occupied_unchecked_mut(new_order_id as usize)
            .id = level_idx;

        new_order_id
    }

    #[allow(clippy::cast_possible_truncation)]
    fn cancel_order(&mut self, order_id: Self::OrderId) {
        let (price, side, internal_id) = match self.orders.get(order_id as usize) {
            Some(order) => (order.limit, order.side, order.id),
            None => return,
        };

        let level = match side {
            OrderSide::Bid => self
                .bids
                .get_mut(&Reverse(price))
                .expect("missing price level"),
            OrderSide::Ask => self.asks.get_mut(&price).expect("missing price level"),
        };

        level.remove(internal_id);
        self.orders.remove(order_id);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order> {
        self.orders.get(order_id as usize)
    }
}

pub struct OrderMatcher {
    pub order_book: OrderBook,
    pub queue: SlotMapUnsafe<u32>,
    cancelation_buffer: Vec<u32>,
}

impl OrderMatcherExt for OrderMatcher {
    type OrderId = u32;
    type OrderBook = OrderBook;

    fn new() -> Self {
        Self {
            order_book: OrderBook::new(),
            queue: SlotMapUnsafe::with_capacity(1_048_576), // 2^20 * 16 = 16777216 = 16MB
            cancelation_buffer: Vec::with_capacity(128),
        }
    }

    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        let new_order_id = self.order_book.place_order(request);
        self.queue.insert(new_order_id)
    }

    fn cancel_order(&mut self, order_id: Self::OrderId) {
        self.order_book.cancel_order(order_id);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn process_limit_order(&mut self, mut request: LimitOrderRequest) -> LimitOrderRequest {
        let limit = request.limit;
        let mut remaining_amount = request.amount;

        macro_rules! execute_matching {
            ($iter:expr, $op:tt) => {
                for (price, order_ids) in $iter {
                    if !(*price $op limit) || remaining_amount == 0 {
                        break;
                    }

                    for id in &*order_ids {
                        let current_order = self.order_book.orders.get_occupied_unchecked_mut(*id as usize);
                        let fill_amount = current_order.amount.min(remaining_amount);

                        current_order.amount -= fill_amount;
                        remaining_amount -= fill_amount;

                        if current_order.amount == 0 {
                            self.cancelation_buffer.push(*id);
                        }

                        if remaining_amount == 0 {
                            break;
                        }
                    }
                }
            };
        }

        match request.side {
            OrderSide::Bid => {
                execute_matching!(self.order_book.asks.iter_mut(), <=);
            }
            OrderSide::Ask => {
                execute_matching!(self.order_book.bids.iter_mut().map(|(r, v)| (&r.0, v)), >=);
            }
        }

        for i in 0..self.cancelation_buffer.len() {
            self.cancel_order(self.cancelation_buffer[i]);
        }

        self.cancelation_buffer.clear();

        request.amount = remaining_amount;
        request
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_bid(&self) -> Option<usize> {
        if let Some((price, ids)) = self.order_book.bids.last_key_value()
            && ids.capacity() != 0
        {
            Some(price.0 as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_ask(&self) -> Option<usize> {
        if let Some((price, ids)) = self.order_book.asks.first_key_value()
            && ids.capacity() != 0
        {
            Some(*price as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn total_volume_at(&self, side: OrderSide, price: usize) -> usize {
        let Some(order_ids) = (match side {
            OrderSide::Bid => self.order_book.bids.get(&Reverse(price as u32)),
            OrderSide::Ask => self.order_book.asks.get(&(price as u32)),
        }) else {
            return 0;
        };

        order_ids
            .iter()
            .map(|id| {
                self.order_book
                    .get_order(*id)
                    .expect("order not found")
                    .amount as usize
            })
            .sum()
    }

    fn order_book(&self) -> &Self::OrderBook {
        &self.order_book
    }
}
