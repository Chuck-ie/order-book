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
    pub total_capacity: usize,
    pub total_len: usize,
    pub total_occupied: usize,
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
            total_occupied: 0,
        }
    }

    // ex1: insert into empty

    #[allow(clippy::cast_possible_truncation)]
    pub fn insert(&mut self, data: LimitOrder, arena: &mut ArenaSlotAllocator) -> ArenaIndex {
        let free_index = self.free_head;
        // we have a slot to recycle
        let insert_index = if free_index.is_some() {
            let (_, next_free) = unsafe {
                arena
                    .get_unchecked(free_index.0 as usize)
                    .as_free_unchecked()
            };

            self.free_head = *next_free;
            free_index.0
        // we need to check if we need to allocate a new chunk of slots
        // case1: allocate slot + write to index 0
        // case2: allocate to the latest index
        } else {
            // 1024 means we need a new chunk
            let chunk_offset = self.total_len % arena.chunk_size();

            if chunk_offset == 0 {
                let new_chunk_index = unsafe { arena.claim_chunk() };
                self.owned_chunks.push(new_chunk_index);
                self.total_capacity += arena.chunk_size();
            }

            let Some(last_chunk_index) = self.owned_chunks.last() else {
                unsafe { std::hint::unreachable_unchecked() };
            };

            self.total_len += 1;
            (last_chunk_index.0 * arena.chunk_size() + chunk_offset) as u32
        };

        let tail_index = self.tail;

        if tail_index.is_some() {
            let (_, _, _, next) = unsafe {
                arena
                    .get_unchecked_mut(tail_index.0 as usize)
                    .as_occupied_unchecked_mut()
            };

            next.0 = insert_index;
        }

        let insert_slot_ref = unsafe { arena.get_unchecked_mut(insert_index as usize) };
        *insert_slot_ref = ArenaSlot::occupied_with_prev(data, tail_index);

        if self.head.is_none() {
            self.head.0 = insert_index;
        }

        self.tail.0 = insert_index;
        self.total_occupied += 1;

        ArenaIndex {
            generation: 0,
            index: insert_index,
        }
    }

    pub fn remove(&mut self, remove_index: &ArenaIndex, arena: &mut ArenaSlotAllocator) -> bool {
        let (generation, curr_prev, curr_next) = {
            let Some(ArenaSlot::Occupied {
                generation,
                prev,
                next,
                ..
            }) = arena.get(remove_index.index as usize)
            else {
                return false;
            };

            if *generation != remove_index.generation {
                return false;
            }

            (*generation, *prev, *next)
        };

        if curr_prev.is_some() {
            let (_, _, _, next) = unsafe {
                arena
                    .get_unchecked_mut(curr_prev.0 as usize)
                    .as_occupied_unchecked_mut()
            };

            *next = curr_next;
        } else {
            self.head = curr_next;
        }

        if curr_next.is_some() {
            let (_, _, prev, _) = unsafe {
                arena
                    .get_unchecked_mut(curr_next.0 as usize)
                    .as_occupied_unchecked_mut()
            };

            *prev = curr_prev;
        } else {
            self.tail = curr_prev;
        }

        unsafe {
            let remove_slot_ref = arena.get_unchecked_mut(remove_index.index as usize);
            *remove_slot_ref = ArenaSlot::Free {
                generation,
                next_free: self.free_head,
            }
        }

        self.free_head.0 = remove_index.index;
        self.total_occupied -= 1;
        self.total_occupied == 0
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
