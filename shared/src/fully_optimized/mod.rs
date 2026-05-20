use std::{cmp::Reverse, collections::BTreeMap, marker::PhantomData};

use crate::fully_optimized::slot_map::{Slot, SlotMap};

pub mod slot_map;

type LevelSlotIdx = u32;
type GlobalSlotIdx = u32;

pub struct OrderBook {
    pub bids: BTreeMap<Reverse<u128>, SlotMap<PackedSlot<Tagged>>>,
    pub asks: BTreeMap<u128, SlotMap<PackedSlot<Tagged>>>,
    pub orders: SlotMap<OrderSlot>,
}

pub struct OrderMatcher {
    pub orderbook: OrderBook,
    pub queue: SlotMap<PackedSlot<Tagged>>,
}

pub struct Order {
    pub side: OrderSide,
    pub price: u128,
    pub qty: u128,
    pub level_slot_idx: LevelSlotIdx,
}

#[derive(Clone, Copy)]
pub enum OrderSide {
    Bid,
    Ask,
}

// pub struct OrderSlot {
//     pub order: Order,
//     pub prev: u32,
//     pub next: u32,
// }

pub enum OrderSlot {
    Free { next_free: u32 },
    Occupied { order: Order, prev: u32, next: u32 },
}

impl Slot for OrderSlot {}

// TODO: benchmark with perf to see difference between 12/16 byte alignment for cache misses
// #[repr(align(16))]
pub struct PackedSlot<S: SlotState> {
    // if free -> next_free else data
    value: GlobalSlotIdx,
    prev: u32,
    next: u32,
    _marker: PhantomData<S>,
}

impl Slot for PackedSlot<Tagged> {}

// macro_rules! define_states {
//     ($($name:ident),*) => {
//         $(
//             pub struct $name;
//             impl SlotState for $name {}
//         )*
//     };
// }

pub trait SlotState {}
pub struct Tagged;
pub struct Free;
pub struct Occupied;

impl SlotState for Tagged {}
impl SlotState for Free {}
impl SlotState for Occupied {}

// #[repr(transparent)]
// pub struct Slot<S: SlotState, TData>(TData, PhantomData<S>);
//
// pub trait SlotState {}
// pub struct Tagged;
// pub struct Free;
// pub struct Occupied;
//
// impl SlotState for Tagged {}
// impl SlotState for Free {}
// impl SlotState for Occupied {}
