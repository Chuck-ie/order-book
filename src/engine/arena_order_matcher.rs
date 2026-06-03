use crate::{
    arena_allocator::{ArenaAllocator, ArenaId},
    common::{MatcherCommand, OrderSide},
    engine::LimitOrder,
    slot_map::chunked::ArenaSlot,
};

pub struct ArenaOrderMatcher<M: ArenaOrderMatcherExt> {
    pub arena: ArenaAllocator<ArenaSlot<LimitOrder>>,
    pub matcher: M,
}

pub trait ArenaOrderMatcherExt {
    fn new() -> Self;

    fn process(
        &mut self,
        command: MatcherCommand<LimitOrder, ArenaId>,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) -> Option<ArenaId>;

    fn process_bid(
        &mut self,
        order: &mut LimitOrder,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    );

    fn process_ask(
        &mut self,
        order: &mut LimitOrder,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    );

    fn best_bid(&self) -> Option<usize>;
    fn best_ask(&self) -> Option<usize>;

    fn total_volume_at(
        &mut self,
        side: OrderSide,
        price: u32,
        arena: &mut ArenaAllocator<ArenaSlot<LimitOrder>>,
    ) -> u32;
}

impl<M: ArenaOrderMatcherExt> ArenaOrderMatcher<M> {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn process(&mut self, command: MatcherCommand<LimitOrder, ArenaId>) -> Option<ArenaId> {
        self.matcher.process(command, &mut self.arena)
    }

    pub fn total_volume_at(&mut self, side: OrderSide, price: u32) -> u32 {
        self.matcher.total_volume_at(side, price, &mut self.arena)
    }

    pub fn get_order(&self, arena_id: &ArenaId) -> Option<&LimitOrder> {
        if let ArenaSlot::Occupied {
            data,
            generation: _,
            prev: _,
            next: _,
        } = unsafe { self.arena.get_unchecked(arena_id.index as usize) }
        {
            Some(data)
        } else {
            None
        }
    }
}
