use std::marker::PhantomData;

use crate::{
    final_ver::{
        ArenaId,
        arena_slot_allocator::{ArenaChunkIndex, ArenaSlotAllocator},
    },
    slot_map::NonMaxU32,
};

pub struct ArenaSlotMap<T> {
    pub head: NonMaxU32,
    pub tail: NonMaxU32,
    pub free_head: NonMaxU32,
    pub owned_chunks: Vec<ArenaChunkIndex>,
    pub total_capacity: usize,
    pub total_len: usize,
    pub total_occupied: usize,
    _slot: PhantomData<T>,
}

pub struct ArenaSlotMapWalker<'a, T> {
    arena: &'a mut ArenaSlotAllocator<T>,
    curr: NonMaxU32,
}

impl<T> ArenaSlotMapWalker<'_, T> {
    pub fn next_pair(&mut self) -> Option<(ArenaId, &mut T)> {
        if self.curr.is_none() {
            return None;
        }

        let index = self.curr.0 as usize;

        let (generation, data, _, next) = unsafe {
            self.arena
                .get_unchecked_mut(index)
                .as_occupied_unchecked_mut()
        };

        let arena_id = ArenaId {
            generation: *generation,
            index: self.curr.0,
        };

        self.curr = *next;
        Some((arena_id, data))
    }
}

impl<T> ArenaSlotMap<T> {
    #[must_use]
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub const fn walk<'a>(
        &mut self,
        arena: &'a mut ArenaSlotAllocator<T>,
    ) -> ArenaSlotMapWalker<'a, T> {
        ArenaSlotMapWalker {
            arena,
            curr: self.head,
        }
    }

    // TODO: check if with_capacity has any performance impact (positive or negative)
    // TODO: check if smallvec could help too
    pub fn from_arena(arena: &mut ArenaSlotAllocator<T>) -> Self {
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
            _slot: PhantomData,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub fn insert(&mut self, data: T, arena: &mut ArenaSlotAllocator<T>) -> ArenaId {
        let free_index = self.free_head;
        // we have a slot to recycle
        let (generation, insert_index) = if free_index.is_some() {
            let (generation, next_free) = unsafe {
                arena
                    .get_unchecked(free_index.0 as usize)
                    .as_free_unchecked()
            };

            self.free_head = *next_free;
            (*generation, free_index.0)
        // we need to check if we need to allocate a new chunk of slots
        // case1: allocate slot + write to index 0
        // case2: allocate to the latest index
        } else {
            // 1024 means we need a new chunk
            let chunk_offset = self.total_len % arena.chunk_size();

            if self.total_len == self.total_capacity {
                let new_chunk_index = unsafe { arena.claim_chunk() };
                self.owned_chunks.push(new_chunk_index);
                self.total_capacity += arena.chunk_size();
            }

            let Some(last_chunk_index) = self.owned_chunks.last() else {
                unsafe { std::hint::unreachable_unchecked() };
            };

            self.total_len += 1;
            (
                0_u32,
                (last_chunk_index.0 * arena.chunk_size() + chunk_offset) as u32,
            )
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
        *insert_slot_ref = ArenaSlot::occupied_with_prev(data, tail_index, generation);

        if self.head.is_none() {
            self.head.0 = insert_index;
        }

        self.tail.0 = insert_index;
        self.total_occupied += 1;

        ArenaId {
            generation,
            index: insert_index,
        }
    }

    pub fn remove(&mut self, remove_id: &ArenaId, arena: &mut ArenaSlotAllocator<T>) -> bool {
        let (generation, curr_prev, curr_next) = {
            let Some(ArenaSlot::Occupied {
                generation,
                prev,
                next,
                ..
            }) = arena.get(remove_id.index as usize)
            else {
                return false;
            };

            if *generation != remove_id.generation {
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
            let remove_slot_ref = arena.get_unchecked_mut(remove_id.index as usize);
            *remove_slot_ref = ArenaSlot::Free {
                generation: generation + 1,
                next_free: self.free_head,
            }
        }

        self.free_head.0 = remove_id.index;
        self.total_occupied -= 1;
        self.total_occupied == 0
    }
}

#[derive(Debug, PartialEq, Eq)]
// #[repr(C, align(64))]
#[repr(C, align(32))]
pub enum ArenaSlot<T> {
    Free {
        generation: u32,
        next_free: NonMaxU32,
    },
    Occupied {
        data: T,
        generation: u32,
        prev: NonMaxU32,
        next: NonMaxU32,
    },
}

impl<T> Default for ArenaSlot<T> {
    fn default() -> Self {
        Self::Free {
            generation: 0,
            next_free: NonMaxU32::new_none(),
        }
    }
}

impl<T> ArenaSlot<T> {
    #[must_use]
    pub const fn occupied_with_prev(data: T, prev: NonMaxU32, generation: u32) -> Self {
        Self::Occupied {
            generation,
            data,
            prev,
            next: NonMaxU32::new_none(),
        }
    }

    #[must_use]
    pub const fn free_with_next(next_free: NonMaxU32, generation: u32) -> Self {
        Self::Free {
            generation,
            next_free,
        }
    }

    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Occupied`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    #[must_use]
    pub const unsafe fn as_occupied_unchecked(&self) -> (&u32, &T, &NonMaxU32, &NonMaxU32) {
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
    ) -> (&mut u32, &mut T, &mut NonMaxU32, &mut NonMaxU32) {
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

// TODO: According to AI this is very bad to do, because someone could get multiple mut references
// to the next return values at the same time, which the current walker version doesnt allow
//
// pub struct ArenaSlotMapPairMut<'b, T> {
//     arena: &'b mut ArenaSlotAllocator<T>,
//     curr: NonMaxU32,
// }
//
// impl<'b, T> Iterator for ArenaSlotMapPairMut<'b, T> {
//     type Item = (ArenaIndex, &'b mut T);
//
//     fn next(&mut self) -> Option<Self::Item> {
//         if self.curr.is_none() {
//             return None;
//         }
//
//         let (generation, data_ref, _, next) = unsafe {
//             self.arena
//                 .get_unchecked_mut(self.curr.0 as usize)
//                 .as_occupied_unchecked_mut()
//         };
//
//         let arena_index = ArenaIndex {
//             generation: *generation,
//             index: self.curr.0,
//         };
//
//         let data = unsafe { &mut *std::ptr::from_mut::<T>(data_ref) };
//
//         self.curr = *next;
//         Some((arena_index, data))
//     }
// }
