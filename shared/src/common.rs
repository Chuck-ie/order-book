use serde::Deserialize;

#[repr(transparent)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrderId(usize);

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

pub struct LimitOrder<ID> {
    pub id: ID,
    pub side: OrderSide,
    pub limit: u64,
    pub amount: u64,
}

impl<ID> LimitOrder<ID> {
    pub const fn new(id: ID, side: OrderSide, limit: u64, amount: u64) -> Self {
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

// impl<Order, OrderId> MatcherCommand<Order, OrderId> {
//     #[must_use]
//     pub const fn new_limit_order(side: OrderSide, limit: u64, amount: u64) -> Self {
//         Self::PlaceOrder(LimitOrderRequest {
//             side,
//             limit,
//             amount,
//         })
//     }
// }
