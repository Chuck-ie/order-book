use crate::{
    arena_optimized::OptimizedArena, Arena, LimitOrder, LimitOrderRequest, OrderBookExt,
    OrderMatcherExt, OrderSide,
};
use std::{cmp::Reverse, collections::BTreeMap};
pub struct OrderBook {
    pub bids: BTreeMap<Reverse<u32>, OptimizedArena<u32>>,
    pub asks: BTreeMap<u32, OptimizedArena<u32>>,
    pub orders: OptimizedArena<LimitOrder<u32>>,
}
impl OrderBookExt for OrderBook {
    type OrderId = u32;
    type Order = LimitOrder<Self::OrderId>;
    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: OptimizedArena::new(),
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
                .or_default()
                .insert(new_order_id),
            OrderSide::Ask => self.asks.entry(price).or_default().insert(new_order_id),
        };

        let order = self
            .orders
            .get_mut(new_order_id as usize)
            .expect("previous insert failed");
        order.id = level_idx;

        new_order_id
    }

    #[allow(clippy::cast_possible_truncation)]
    fn cancel_order(&mut self, order_id: Self::OrderId) {
        let (price, side, internal_id) = match self.orders.get(order_id as usize) {
            Some(order) => (order.limit, order.side, order.id),
            None => return,
        };

        let level_is_empty = {
            let level = match side {
                OrderSide::Bid => self
                    .bids
                    .get_mut(&Reverse(price))
                    .expect("missing price level"),
                OrderSide::Ask => self.asks.get_mut(&price).expect("missing price level"),
            };

            level.remove(internal_id);
            level.is_empty()
        };

        if level_is_empty {
            match side {
                OrderSide::Bid => self.bids.remove(&Reverse(price)),
                OrderSide::Ask => self.asks.remove(&price),
            };
        }

        self.orders.remove(order_id);
    }

    #[allow(clippy::cast_possible_truncation)]
    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order> {
        self.orders.get(order_id as usize)
    }
}

pub struct OrderMatcher {
    pub order_book: OrderBook,
    pub queue: OptimizedArena<u32>,
}

impl OrderMatcherExt for OrderMatcher {
    type OrderId = u32;
    type OrderBook = OrderBook;

    fn new() -> Self {
        Self {
            order_book: OrderBook::new(),
            queue: OptimizedArena::new(),
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
        let mut orders_to_remove = vec![];

        let side_iterator: Box<dyn Iterator<Item = (&u32, &mut OptimizedArena<u32>)>> =
            match request.side {
                OrderSide::Bid => Box::new(self.order_book.asks.iter_mut()),
                OrderSide::Ask => Box::new(self.order_book.bids.iter_mut().map(|(r, v)| (&r.0, v))),
            };

        for (price, order_ids) in side_iterator {
            let price_matches = match request.side {
                OrderSide::Bid => *price <= limit,
                OrderSide::Ask => *price >= limit,
            };

            if !price_matches || remaining_amount == 0 {
                break;
            }

            for id in &*order_ids {
                let current_order = self
                    .order_book
                    .orders
                    .get_mut(*id as usize)
                    .expect("orderbook and -matcher out of sync");

                let fill_amount = current_order.amount.min(remaining_amount);

                current_order.amount -= fill_amount;
                remaining_amount -= fill_amount;

                if current_order.amount == 0 {
                    orders_to_remove.push(*id);
                }

                if remaining_amount == 0 {
                    break;
                }
            }
        }

        for id in orders_to_remove {
            self.cancel_order(id);
        }

        request.amount = remaining_amount;
        request
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_bid(&self) -> Option<usize> {
        if let Some((price, _)) = self.order_book.bids.first_key_value() {
            Some(price.0 as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_ask(&self) -> Option<usize> {
        if let Some((price, _)) = self.order_book.asks.first_key_value() {
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

// impl OrderBook {
//
//     #[must_use]
//     pub fn place_order(
//         &mut self,
//         matcher_id: u32,
//         side: OrderSide,
//         price: u32,
//         amount: u32,
//     ) -> u32 {
//         let new_order = BookOrder::new(matcher_id, amount);
//         let (levels, search_res) = self.find_level(side, price);
//
//         match search_res {
//             Ok(i) => levels[i].orders.insert(new_order),
//             Err(i) => {
//                 let mut new_level = PriceLevel::from_price(price);
//                 let level_id = new_level.insert(new_order);
//                 levels.insert(i, new_level);
//                 level_id
//             }
//         }
//     }
//
//     pub fn remove_order(&mut self, side: OrderSide, price: u32, level_id: u32) {
//         let (levels, search_res) = self.find_level(side, price);
//
//         let Ok(i) = search_res else {
//             return;
//         };
//
//         levels[i].orders.remove(level_id);
//     }
//
//     fn find_level(
//         &mut self,
//         side: OrderSide,
//         price: u32,
//     ) -> (&mut Vec<PriceLevel>, Result<usize, usize>) {
//         match side {
//             OrderSide::Bid => {
//                 let bids = &mut self.bids;
//                 let result = bids.binary_search_by(|level| price.cmp(&level.price));
//                 (bids, result)
//             }
//             OrderSide::Ask => {
//                 let asks = &mut self.asks;
//                 let result = asks.binary_search_by(|level| level.price.cmp(&price));
//                 (asks, result)
//             }
//         }
//     }
// }
//
// impl Default for OrderBook {
//     fn default() -> Self {
//         Self::new()
//     }
// }
//
// impl PriceLevel {
//     #[must_use]
//     pub fn from_price(price: u32) -> Self {
//         Self {
//             price,
//             orders: OptimizedArena::new(),
//         }
//     }
//
//     pub fn insert(&mut self, order: BookOrder) -> u32 {
//         self.orders.insert(order)
//     }
// }
//
// impl BookOrder {
//     #[must_use]
//     pub const fn new(matcher_id: u32, amount: u32) -> Self {
//         Self {
//             matcher_id,
//             amount,
//         }
//     }
// }
//
// pub enum MatcherCommand {
//     PlaceOrder(LimitOrder),
//     RemoveOrder(u32),
// }
//
// pub struct LimitOrder {
//     pub book_order_id: u32,
//     pub side: OrderSide,
//     pub limit: u32,
//     pub amount: u32,
// }
//
// pub struct OrderMatcher {
//     pub order_book: OrderBook,
//     pub open_orders: OptimizedArena<LimitOrder>,
// }
//
// impl OrderMatcher {
//     #[must_use]
//     fn new() -> Self {
//         Self {
//             order_book: OrderBook::new(),
//             open_orders: OptimizedArena::new(),
//         }
//     }
//
//     pub fn place_order(&mut self, order: LimitOrder) -> Option<u32> {
//         let order = self.process_limit_order(order);
//
//         if order.amount == 0 {
//             None
//         } else {
//             let matcher_id = self.open_orders.insert(order);
//
//             let Some(Slot::Occupied(order)) = self.open_orders.get_mut(matcher_id as usize) else {
//                 unreachable!("insert order failed");
//             };
//
//             let LimitOrder {
//                 book_order_id,
//                 side,
//                 limit,
//                 amount,
//             } = order;
//
//             *book_order_id = self.order_book.place_order(matcher_id, *side, *limit, *amount);
//
//             Some(matcher_id)
//         }
//     }
//
//     pub fn remove_order(&mut self, matcher_id: u32) {
//         let Some(Slot::Occupied(data)) = self.open_orders.get(matcher_id as usize) else {
//             unreachable!("missing order in queue");
//         };
//
//         self.order_book.remove_order(data.side, data.limit, data.book_order_id);
//         self.open_orders.remove(matcher_id);
//     }
//
//     // Current Bid Prices: [1043, 1042, 1041, 1007, 994, 988, 986, 982, 977]
//     // Current Ask Prices: [977, 982, 986, 988, 994, 1007, 1041, 1042, 1043]
//     fn process_limit_order(&mut self, mut order: LimitOrder) -> LimitOrder {
//         let levels = match &order.side {
//             OrderSide::Bid => &mut self.order_book.asks,
//             OrderSide::Ask => &mut self.order_book.bids,
//         };
//         let mut missing_amount = order.amount;
//
//         for level in levels.iter_mut() {
//             if missing_amount == 0 {
//                 break;
//             }
//
//             let price_matches = match order.side {
//                 OrderSide::Bid => level.price <= order.limit,
//                 OrderSide::Ask => level.price >= order.limit,
//             };
//
//             if !price_matches {
//                 break;
//             }
//
//             let mut remove_book_orders = vec![];
//
//             for book_order in &level.orders {
//                 let Some(Slot::Occupied(current_order)) =
//                     self.open_orders.get_mut(book_order.matcher_id as usize)
//                 else {
//                     unreachable!("tried accessing invalid order");
//                 };
//
//                 if current_order.amount >= missing_amount {
//                     current_order.amount -= missing_amount;
//                     missing_amount = 0;
//                     break;
//                 }
//
//                 missing_amount -= current_order.amount;
//                 current_order.amount = 0;
//                 remove_book_orders.push(current_order.book_order_id);
//             }
//
//             for book_order_id in remove_book_orders {
//                 level.orders.remove(book_order_id);
//             }
//         }
//
//         order.amount = missing_amount;
//         order
//     }
// }
//
// impl Default for OrderMatcher {
//     fn default() -> Self {
//         Self::new()
//     }
// }
