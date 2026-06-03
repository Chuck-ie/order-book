use std::cell::UnsafeCell;

use order_book::{
    arena_allocator::{ArenaAllocator, ArenaId},
    common::{LimitOrderRequest, MatcherCommand, OrderMatcherExt},
    engine::{LimitOrder, arena_order_matcher::ArenaOrderMatcherExt},
    slot_map::chunked::ArenaSlot,
};

use crate::shared::SyntheticOrder;

pub trait BenchEngine: Default {
    type Order: Clone;
    type OrderId: Clone;
    type Command: Clone + From<SyntheticOrder>;

    fn process(&mut self, cmd: Self::Command) -> Option<Self::OrderId>;
    fn new_cancel_order(order_id: Self::OrderId) -> Self::Command;
}

#[derive(Default)]
pub struct DefaultBenchEngine<Engine: OrderMatcherExt> {
    engine: Engine,
}

impl<Engine: Default + OrderMatcherExt> BenchEngine for DefaultBenchEngine<Engine> {
    type Order = LimitOrderRequest;
    type OrderId = Engine::OrderId;
    type Command = MatcherCommand<Self::Order, Self::OrderId>;

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn process(&mut self, cmd: Self::Command) -> Option<Self::OrderId> {
        self.engine.process(cmd)
    }

    fn new_cancel_order(order_id: Self::OrderId) -> Self::Command {
        MatcherCommand::CancelOrder(order_id)
    }
}

thread_local! {
    // static ARENA_ALLOCATOR: UnsafeCell<ArenaAllocator<ArenaSlot<LimitOrder>>> = UnsafeCell::new(ArenaAllocator::new(16384, 8192));
    static ARENA_ALLOCATOR: UnsafeCell<ArenaAllocator<ArenaSlot<LimitOrder>>> = UnsafeCell::new(ArenaAllocator::new(16384, 16384));
}

pub struct ArenaBenchEngine<Engine: ArenaOrderMatcherExt> {
    engine: Engine,
    arena: *mut ArenaAllocator<ArenaSlot<LimitOrder>>,
}

impl<Engine: Default + ArenaOrderMatcherExt> Default for ArenaBenchEngine<Engine> {
    fn default() -> Self {
        let arena = ARENA_ALLOCATOR.with(UnsafeCell::get);
        unsafe { (&mut *arena).clear() };

        Self {
            engine: Engine::default(),
            arena: arena.cast(),
        }
    }
}

impl<M: ArenaOrderMatcherExt + Default> BenchEngine for ArenaBenchEngine<M> {
    type Order = LimitOrder;
    type OrderId = ArenaId;
    type Command = MatcherCommand<Self::Order, Self::OrderId>;

    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn process(&mut self, cmd: Self::Command) -> Option<Self::OrderId> {
        let arena = unsafe { &mut *self.arena };
        self.engine.process(cmd, arena)
    }

    fn new_cancel_order(order_id: Self::OrderId) -> Self::Command {
        MatcherCommand::CancelOrder(order_id)
    }
}
