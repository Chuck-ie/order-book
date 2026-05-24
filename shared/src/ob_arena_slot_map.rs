use crate::{
    LimitOrder, LimitOrderRequest, OrderBookExt, OrderMatcherExt, OrderSide, SlotMap,
    slot_map::optimized::SlotMapOptimized,
};
use std::{cmp::Reverse, collections::BTreeMap};

pub struct SlotArena<T> {
    pub chunks: Vec<SlotMapOptimized<T>>,
    pub free_chunks: Vec<usize>,
    pub chunk_count: usize,
    pub chunk_size: usize,
}

impl<T> SlotArena<T> {
    #[must_use]
    pub fn with_capacity(chunk_count: usize, chunk_size: usize) -> Self {
        let chunks = (0..chunk_count)
            .map(|_| SlotMapOptimized::with_capacity(chunk_size))
            .collect();

        let free_chunks = (0..chunk_count).collect();

        Self {
            chunks,
            free_chunks,
            chunk_count,
            chunk_size,
        }
    }

    pub fn claim(&mut self) -> usize {
        debug_assert!(!self.free_chunks.is_empty(), "SlotArena out of chunks");

        // TODO: replace unsafe code with safe code that allocates more chunks with initial capacity
        unsafe { self.free_chunks.pop().unwrap_unchecked() }
    }

    pub fn release(&mut self, chunk_index: usize) {
        self.free_chunks.push(chunk_index);
    }

    #[must_use]
    pub fn get(&self, arena_id: ArenaId) -> Option<&T> {
        self.chunks
            .get(arena_id.chunk_id as usize)
            .and_then(|chunk| chunk.get(arena_id.slot_id as usize))
    }

    #[must_use]
    pub fn get_unchecked(&self, arena_id: ArenaId) -> &T {
        unsafe {
            self.chunks
                .get_unchecked(arena_id.chunk_id as usize)
                .get_occupied_unchecked(arena_id.slot_id as usize)
        }
    }

    #[must_use]
    pub fn get_unchecked_mut(&mut self, arena_id: ArenaId) -> &mut T {
        unsafe {
            self.chunks
                .get_unchecked_mut(arena_id.chunk_id as usize)
                .get_occupied_unchecked_mut(arena_id.slot_id as usize)
        }
    }
}

#[derive(Clone, Copy)]
pub struct ArenaId {
    pub chunk_id: u16,
    pub slot_id: u16,
}

impl ArenaId {
    #[must_use]
    pub const fn new(chunk_id: u16, slot_id: u16) -> Self {
        Self { chunk_id, slot_id }
    }
}

pub struct PriceLevel {
    pub price: u64,
    pub chunk_ids: Vec<usize>,
}

impl PriceLevel {
    pub fn new<T>(price: u64, order_arena: &mut SlotArena<T>) -> Self {
        Self {
            price,
            chunk_ids: vec![order_arena.claim()],
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn insert<T>(&mut self, order_arena: &mut SlotArena<T>, data: T) -> ArenaId {
        let mut active_chunk_id =
            unsafe { *self.chunk_ids.get_unchecked(self.chunk_ids.len() - 1) };

        let mut active_chunk = unsafe { order_arena.chunks.get_unchecked_mut(active_chunk_id) };

        if active_chunk.free_head.is_none()
            && active_chunk.slots.len() == active_chunk.slots.capacity()
        {
            active_chunk_id = order_arena.claim();
            active_chunk = unsafe { order_arena.chunks.get_unchecked_mut(active_chunk_id) };
            self.chunk_ids.push(active_chunk_id);
        }

        ArenaId::new(active_chunk_id as u16, active_chunk.insert(data) as u16)
    }

    pub fn remove<T>(&mut self, order_arena: &mut SlotArena<T>, order_id: ArenaId) {
        let remove_chunk_id = order_id.chunk_id as usize;
        let chunk = unsafe { order_arena.chunks.get_unchecked_mut(remove_chunk_id) };

        chunk.remove(order_id.slot_id.into());

        if chunk.capacity() == 0 {
            self.chunk_ids.retain(|&id| id != remove_chunk_id);
            order_arena.release(remove_chunk_id);
        }
    }

    #[must_use]
    pub fn is_empty<T>(&self, order_arena: &SlotArena<T>) -> bool {
        self.chunk_ids
            .iter()
            .all(|&id| order_arena.chunks[id].capacity() == 0)
    }
}

pub struct OrderBook {
    pub bids: BTreeMap<Reverse<u64>, PriceLevel>,
    pub asks: BTreeMap<u64, PriceLevel>,
    pub order_arena: SlotArena<LimitOrder<ArenaId>>,
}

impl OrderBookExt for OrderBook {
    type OrderId = ArenaId;
    type Order = LimitOrder<Self::OrderId>;

    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            // 4096 price levels * 1024 orders per price level = 4_194_304 orders = 128 MB
            order_arena: SlotArena::with_capacity(4096, 1024),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        let LimitOrderRequest {
            side,
            limit: price,
            amount,
        } = request;

        let new_order = LimitOrder::new(ArenaId::new(0, 0), side, price, amount);
        let new_order_id = match side {
            OrderSide::Bid => self
                .bids
                .entry(Reverse(price))
                .or_insert_with(|| PriceLevel::new(price, &mut self.order_arena))
                .insert(&mut self.order_arena, new_order),
            OrderSide::Ask => self
                .asks
                .entry(price)
                .or_insert_with(|| PriceLevel::new(price, &mut self.order_arena))
                .insert(&mut self.order_arena, new_order),
        };

        let inserted_order = self.order_arena.get_unchecked_mut(new_order_id);
        inserted_order.id = new_order_id;
        new_order_id
    }

    #[allow(clippy::cast_possible_truncation)]
    fn cancel_order(&mut self, order_id: Self::OrderId) {
        let (price, side) = match self.order_arena.get(order_id) {
            Some(order) => (order.limit, order.side),
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

            level.remove(&mut self.order_arena, order_id);
            level.is_empty(&self.order_arena)
        };

        if level_is_empty {
            match side {
                OrderSide::Bid => self.bids.remove(&Reverse(price)),
                OrderSide::Ask => self.asks.remove(&price),
            };
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn get_order(&self, order_id: Self::OrderId) -> Option<&Self::Order> {
        self.order_arena.get(order_id)
    }

    fn capacity(&self) -> usize {
        let mut capacity = 0;

        for chunk in &self.order_arena.chunks {
            capacity += chunk.capacity();
        }

        capacity
    }
}

pub struct OrderMatcher {
    pub order_book: OrderBook,
    cancelation_buffer: Vec<ArenaId>,
}

impl OrderMatcherExt for OrderMatcher {
    type OrderId = ArenaId;
    type OrderBook = OrderBook;

    fn new() -> Self {
        Self {
            order_book: OrderBook::new(),
            cancelation_buffer: Vec::with_capacity(1024),
        }
    }

    fn place_order(&mut self, request: LimitOrderRequest) -> Self::OrderId {
        self.order_book.place_order(request)
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
                for (price, level) in $iter {
                    if !(*price $op limit) || remaining_amount == 0 {
                        break;
                    }

                    for chunk_id in &level.chunk_ids {
                        let chunk = unsafe {
                            self.order_book
                                .order_arena
                                .chunks
                                .get_unchecked_mut(*chunk_id)
                        };

                        for current_order in chunk.iter_mut() {
                            let fill_amount = current_order.amount.min(remaining_amount);

                            current_order.amount -= fill_amount;
                            remaining_amount -= fill_amount;

                            if current_order.amount == 0 {
                                self.cancelation_buffer.push(current_order.id);
                            }

                            if remaining_amount == 0 {
                                break;
                            }
                        }
                    }
                    //
                    // for order_id in self.cancelation_buffer.drain(..) {
                    //     // self.order_book.cancel_order(order_id);
                    //     level.remove(&mut self.order_book.order_arena, order_id);
                    // }
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

        // let buf = std::mem::take(&mut self.cancelation_buffer);
        //
        // for order_id in buf {
        //     self.cancel_order(order_id);
        // }

        for i in 0..self.cancelation_buffer.len() {
            self.cancel_order(self.cancelation_buffer[i]);
        }

        self.cancelation_buffer.clear();

        request.amount = remaining_amount;
        request
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_bid(&self) -> Option<usize> {
        if let Some((price, level)) = self.order_book.bids.last_key_value()
            && !level.is_empty(&self.order_book.order_arena)
        {
            Some(price.0 as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn best_ask(&self) -> Option<usize> {
        if let Some((price, level)) = self.order_book.asks.first_key_value()
            && !level.is_empty(&self.order_book.order_arena)
        {
            Some(*price as usize)
        } else {
            None
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn total_volume_at(&self, side: OrderSide, price: usize) -> usize {
        let Some(level) = (match side {
            OrderSide::Bid => self.order_book.bids.get(&Reverse(price as u64)),
            OrderSide::Ask => self.order_book.asks.get(&(price as u64)),
        }) else {
            return 0;
        };

        level
            .chunk_ids
            .iter()
            .map(|chunk_id| {
                let chunk = unsafe { self.order_book.order_arena.chunks.get_unchecked(*chunk_id) };
                chunk.iter().map(|order| order.amount).sum::<u64>() as usize
            })
            .sum()
    }

    fn order_book(&self) -> &Self::OrderBook {
        &self.order_book
    }
}
