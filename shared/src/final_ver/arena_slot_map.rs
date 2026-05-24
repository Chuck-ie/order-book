use crate::{
    final_ver::{
        ArenaIndex,
        arena_slot_allocator::{ArenaChunkIndex, ArenaSlotAllocator},
        orderbook::LimitOrder,
    },
    slot_map::NonMaxU32,
};

pub struct ArenaSlotMap {
    pub head: NonMaxU32,
    pub tail: NonMaxU32,
    pub free_head: NonMaxU32,
    pub owned_chunks: Vec<ArenaChunkIndex>,
    // pub capacity: usize,
    pub next_unallocated: u32,
    pub total_capacity: usize,
    pub total_len: usize,
    // pub current_len: usize,
}

impl ArenaSlotMap {
    // TODO: check if with_capacity has any performance impact (positive or negative)
    pub fn from_arena(arena: &mut ArenaSlotAllocator) -> Self {
        let chunk_size = arena.chunk_size();
        let chunk_index = unsafe { arena.claim_chunk() };
        let mut owned_chunks = Vec::with_capacity(4);
        owned_chunks.push(chunk_index);

        Self {
            head: NonMaxU32::new_none(),
            tail: NonMaxU32::new_none(),
            free_head: NonMaxU32::new_none(),
            // owned_chunks: vec![chunk_index],
            owned_chunks,
            total_capacity: chunk_size,
            total_len: 0,
            // current_len: 0,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn insert(&mut self, data: LimitOrder, arena: &mut ArenaSlotAllocator) -> ArenaIndex {
        let free_index;

        if self.free_head.is_some() {
            free_index = self.free_head.0;

            unsafe {
                let (_, next_free) = arena.get_unchecked(free_index as usize).as_free_unchecked();
                self.free_head = *next_free;
            }
        } else {
            if self.current_len == arena.chunk_size() {
                // TODO: benchmark if hint::cold_path reduce branch misses even further
                // since resizing should happen rarely
                let new_chunk_index = unsafe { arena.claim_chunk() };
                self.owned_chunks.push(new_chunk_index);
                self.capacity += arena.chunk_size();
                self.current_len = 0;
            }

            let Some(free_chunk_index) = self.owned_chunks.last() else {
                unsafe { std::hint::unreachable_unchecked() };
            };

            free_index = (free_chunk_index.0 * arena.chunk_size() + self.current_len) as u32;
            self.current_len += 1;
        }

        let tail_index = self.tail;

        if tail_index.is_some() {
            unsafe {
                let (_, _, _, next) = arena
                    .get_unchecked_mut(tail_index.0 as usize)
                    .as_occupied_unchecked_mut();

                next.0 = free_index;
            }
        }

        let new_occupied = ArenaSlot::occupied_with_prev(data, tail_index);

        unsafe {
            let free_slot_ref = arena.get_unchecked_mut(free_index as usize);
            *free_slot_ref = new_occupied;
        };

        if self.head.is_none() {
            self.head.0 = free_index;
        }

        self.tail.0 = free_index;
        self.total_len += 1;

        ArenaIndex(free_index)
    }

    pub fn remove(&mut self, remove_index: &ArenaIndex, arena: &mut ArenaSlotAllocator) -> bool {
        // TODO: honestly, this should probably never be unsafe. Slots should have a version index and
        // then this initial fetch should check if slot is occupied and version of slot and index match
        let (_, _, curr_prev, curr_next) = unsafe {
            arena
                .get_unchecked(remove_index.0 as usize)
                .as_occupied_unchecked()
        };

        let curr_prev = *curr_prev;
        let curr_next = *curr_next;

        if curr_prev.is_some() {
            unsafe {
                let (_, _, _, next) = arena
                    .get_unchecked_mut(curr_prev.0 as usize)
                    .as_occupied_unchecked_mut();

                *next = curr_next;
            }
        }

        if curr_next.is_some() {
            unsafe {
                let (_, _, prev, _) = arena
                    .get_unchecked_mut(curr_next.0 as usize)
                    .as_occupied_unchecked_mut();
                *prev = curr_prev;
            }
        }

        unsafe {
            let remove_slot_ref = arena.get_unchecked_mut(remove_index.0 as usize);
            *remove_slot_ref = ArenaSlot::free_with_next(self.free_head);
        }

        self.free_head = NonMaxU32::from(remove_index.0);

        if curr_next.is_none() {
            self.tail = curr_prev;
        }

        if curr_prev.is_none() {
            self.head = curr_next;
        }

        self.total_len = self.total_len.saturating_sub(1);

        // return value should be used to release the claims on all chunks allocated by this slotmap
        self.total_len == 0
    }
}

pub enum ArenaSlot {
    Free {
        generation: u32,
        next_free: NonMaxU32,
    },
    Occupied {
        generation: u32,
        data: LimitOrder,
        prev: NonMaxU32,
        next: NonMaxU32,
    },
}

impl ArenaSlot {
    #[must_use]
    pub const fn occupied_with_prev(data: LimitOrder, prev: NonMaxU32) -> Self {
        Self::Occupied {
            generation: 0,
            data,
            prev,
            next: NonMaxU32::new_none(),
        }
    }

    #[must_use]
    pub const fn free_with_next(next_free: NonMaxU32) -> Self {
        Self::Free {
            generation: 0,
            next_free,
        }
    }

    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Occupied`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_occupied_unchecked(
        &self,
    ) -> (&u32, &LimitOrder, &NonMaxU32, &NonMaxU32) {
        match self {
            Self::Occupied {
                generation,
                data,
                prev,
                next,
            } => (generation, data, prev, next),
            Self::Free { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Occupied`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_occupied_unchecked_mut(
        &mut self,
    ) -> (&mut u32, &mut LimitOrder, &mut NonMaxU32, &mut NonMaxU32) {
        match self {
            Self::Occupied {
                generation,
                data,
                prev,
                next,
            } => (generation, data, prev, next),
            Self::Free { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Free`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_free_unchecked(&self) -> (&u32, &NonMaxU32) {
        match self {
            Self::Free {
                generation,
                next_free,
            } => (generation, next_free),
            Self::Occupied { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Free`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_free_unchecked_mut(&mut self) -> (&mut u32, &mut NonMaxU32) {
        match self {
            Self::Free {
                generation,
                next_free,
            } => (generation, next_free),
            Self::Occupied { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}
