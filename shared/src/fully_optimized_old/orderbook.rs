use std::collections::BTreeMap;

use crate::fully_optimized_old::arena::Arena;

// #[repr(transparent)]
// #[derive(Default)]
// pub struct OrderIndex(u32);
//
// type Price = u32;
//
// pub struct OrderBook {
//     market: [BTreeMap<Price, Arena<OrderIndex>>; 2],
// }
//
// pub struct HotIdx {
//     pub price: Price,
//     pub index: u32,
// }
//
// // #[derive(Default())]
// pub struct PriceGrid {
//     data: BTreeMap<Price, Arena<OrderIndex>>,
//     hot: [Option<Arena<OrderIndex>>; 128],
//     hot_low: Option<HotIdx>,
//     hot_high: Option<HotIdx>,
// }
//
// // impl PriceGrid {
// //     #[must_use]
// //     pub const fn new() -> Self {
// //         Self {
// //             data: BTreeMap::new(),
// //             hot: [None; 128],
// //             hot_low: None,
// //             hot_high: None,
// //         }
// //     }
// // }
// //
// // impl Default for PriceGrid {
// //     fn default() -> Self {
// //         Self::new()
// //     }
// // }
//
// #[repr(u8)]
// #[derive(Clone, Copy)]
// pub enum OrderSide {
//     Bid = 0,
//     Ask = 1,
// }
