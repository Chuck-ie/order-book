use serde::Deserialize;
//
#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderIdU64(pub u64);

#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderIdU32(pub u32);

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum OrderSide {
    Bid,
    Ask,
}

// helper implementation for parsing from csv
impl From<i8> for OrderSide {
    fn from(val: i8) -> Self {
        if val == 1 { Self::Bid } else { Self::Ask }
    }
}

pub struct LimitOrder<OrderId: Copy + PartialEq + Eq> {
    pub id: OrderId,
    pub side: OrderSide,
    pub limit: u64,
    pub amount: u64,
}

impl<OrderId: Copy + PartialEq + Eq> LimitOrder<OrderId> {
    pub const fn new(id: OrderId, side: OrderSide, limit: u64, amount: u64) -> Self {
        Self {
            id,
            side,
            limit,
            amount,
        }
    }
}

#[derive(Debug)]
pub enum MatcherCommand<Order, OrderId> {
    PlaceOrder(Order),
    CancelOrder(OrderId),
}

#[derive(Debug)]
pub struct LimitOrderRequest {
    pub side: OrderSide,
    pub limit: u64,
    pub amount: u64,
}

pub trait OrderBookExt {
    type OrderId;
    type Order;

    fn new() -> Self;
    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId;
    fn cancel_order(&mut self, order_id: Self::OrderId);
    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order>;
    fn capacity(&self) -> usize;
}

pub trait OrderMatcherExt {
    type OrderId: Copy + PartialEq + Eq;
    type OrderBook: OrderBookExt<OrderId = Self::OrderId>;

    fn new() -> Self;

    fn process(
        &mut self,
        command: MatcherCommand<LimitOrderRequest, Self::OrderId>,
    ) -> Option<Self::OrderId> {
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
