use std::fmt::Debug;

pub mod arena_naive;
pub mod arena_optimized;
pub mod fully_optimized;
pub mod ob_arena_naive;
pub mod ob_arena_optimized;
pub mod ob_naive;
pub mod ob_standard;

pub trait OrderBookExt {
    type OrderId;
    type Order;

    fn new() -> Self;
    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId;
    fn cancel_order(&mut self, order_id: Self::OrderId);
    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order>;
}

pub trait OrderMatcherExt {
    type OrderId: Copy;
    type OrderBook: OrderBookExt<OrderId = Self::OrderId>;

    fn new() -> Self;

    fn process(&mut self, command: MatcherCommand<Self::OrderId>) -> Option<Self::OrderId> {
        match command {
            MatcherCommand::PlaceOrder(order) => {
                let order = self.process_limit_order(order);

                if order.amount > 0 {
                    Some(self.place_order(order))
                } else {
                    None
                }
            }
            MatcherCommand::CancelOrder(id) => {
                self.cancel_order(id);
                None
            }
        }
    }

    #[doc(hidden)]
    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId;

    #[doc(hidden)]
    fn cancel_order(&mut self, order_id: Self::OrderId);

    #[doc(hidden)]
    fn process_limit_order(&mut self, request: LimitOrderRequest) -> LimitOrderRequest;

    // testing helper functions
    fn best_bid(&self) -> Option<usize>;
    fn best_ask(&self) -> Option<usize>;
    fn total_volume_at(&self, side: OrderSide, price: usize) -> usize;
    fn order_book(&self) -> &Self::OrderBook;
}

#[derive(Clone, Copy)]
pub enum OrderSide {
    Bid,
    Ask,
}

pub struct LimitOrder<ID> {
    pub id: ID,
    pub side: OrderSide,
    pub limit: u32,
    pub amount: u32,
}

impl<ID> LimitOrder<ID> {
    pub const fn new(id: ID, side: OrderSide, limit: u32, amount: u32) -> Self {
        Self {
            id,
            side,
            limit,
            amount,
        }
    }
}

pub enum MatcherCommand<ID> {
    PlaceOrder(LimitOrderRequest),
    CancelOrder(ID),
}

impl<ID> MatcherCommand<ID> {
    #[must_use]
    pub const fn new_limit_order(side: OrderSide, limit: u32, amount: u32) -> Self {
        Self::PlaceOrder(LimitOrderRequest {
            side,
            limit,
            amount,
        })
    }
}

pub struct LimitOrderRequest {
    pub side: OrderSide,
    pub limit: u32,
    pub amount: u32,
}

pub trait Arena {
    type Data;
    type Utype: TryFrom<usize> + Debug + PartialEq + Copy;

    fn new() -> Self;
    fn insert(&mut self, data: Self::Data) -> Self::Utype;
    fn remove(&mut self, remove_idx: Self::Utype);

    fn total(&self) -> usize;
    fn capacity(&self) -> usize;
    fn is_empty(&self) -> bool;

    fn get(&self, index: usize) -> Option<&Self::Data>;
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Data>;
}

pub trait TestableArena {
    type Data: PartialEq;
    type Utype: TryFrom<usize> + Debug + PartialEq + Copy;

    fn head(&self) -> Option<Self::Utype>;
    fn tail(&self) -> Option<Self::Utype>;
    fn free_head(&self) -> Option<Self::Utype>;
    fn is_occupied(&self, index: usize, data: Self::Data) -> bool;
    fn get_link(&self, index: usize) -> Option<&impl Linkable>;
}

pub trait Linkable {
    fn prev(&self) -> Option<usize>;
    fn next(&self) -> Option<usize>;
}
