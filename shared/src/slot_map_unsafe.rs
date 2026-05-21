use std::fs::exists;

use crate::{Linkable, SlotMap, TestableSlotMap, slot_map_unsafe};

pub struct SlotMapUnsafe<T> {
    pub head: u32,
    pub tail: u32,
    pub free_head: u32,
    pub slots: Vec<Slot<T>>,
    capacity: u32,
}

pub enum Slot<T> {
    Free { next_free: u32 },
    Occupied { data: T, prev: u32, next: u32 },
}

impl<T> Slot<T> {
    // Safety: Caller must guarantee the slot is `Slot::Occupied`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    const unsafe fn as_occupied_unchecked(&self) -> (&T, &u32, &u32) {
        match self {
            Self::Occupied { data, prev, next } => (data, prev, next),
            Self::Free { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    // Safety: Caller must guarantee the slot is `Slot::Occupied`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    const unsafe fn as_occupied_unchecked_mut(&mut self) -> (&mut T, &mut u32, &mut u32) {
        match self {
            Self::Occupied { data, prev, next } => (data, prev, next),
            Self::Free { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    // Safety: Caller must guarantee the slot is `Slot::Occupied`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    const unsafe fn as_free_unchecked(&self) -> &u32 {
        match self {
            Self::Free { next_free } => next_free,
            Self::Occupied { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    // // Safety: Caller must guarantee the slot is `Slot::Occupied`.
    // #[allow(clippy::inline_always)]
    // #[inline(always)]
    // const unsafe fn as_free_unchecked_mut(&mut self) -> &mut u32 {
    //     match self {
    //         Self::Free { next_free } => next_free,
    //         Self::Occupied { .. } => unsafe { std::hint::unreachable_unchecked() },
    //     }
    // }
}

impl<T> SlotMapUnsafe<T> {
    #[must_use]
    pub fn iter(&self) -> ArenaIter<'_, T> {
        self.into_iter()
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            head: u32::MAX,
            tail: u32::MAX,
            free_head: u32::MAX,
            slots: Vec::with_capacity(capacity),
            capacity: 0,
        }
    }

    #[must_use]
    pub fn get_occupied_unchecked(&self, index: usize) -> &T {
        let (data, _, _) = unsafe { self.slots.get_unchecked(index).as_occupied_unchecked() };
        data
    }

    #[must_use]
    pub fn get_occupied_unchecked_mut(&mut self, index: usize) -> &mut T {
        let (data, _, _) = unsafe {
            self.slots
                .get_unchecked_mut(index)
                .as_occupied_unchecked_mut()
        };
        data
    }
}

impl<T> Default for SlotMapUnsafe<T> {
    fn default() -> Self {
        Self {
            head: u32::MAX,
            tail: u32::MAX,
            free_head: u32::MAX,
            slots: vec![],
            capacity: 0,
        }
    }
}

impl<T> SlotMap for SlotMapUnsafe<T> {
    type Data = T;
    type Utype = u32;

    fn new() -> Self {
        Self {
            head: u32::MAX,
            tail: u32::MAX,
            free_head: u32::MAX,
            slots: vec![],
            capacity: 0,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn insert(&mut self, data: Self::Data) -> Self::Utype {
        let free_idx = self.free_head;

        let insert_idx = if free_idx == u32::MAX {
            self.slots.len() as u32
        } else {
            // Safety: since we previously checked if free_idx == u32::MAX (meaning if its None)
            // we can safely do unsafe enum extraction. Any UB means there is a bug in defining
            // the free_idx/updating the self.free_head
            // TODO: update to use ptrs like for the slot assignment to make this zero cost.
            // currently blocked by: https://github.com/rust-lang/rust/issues/120141
            debug_assert!(
                (free_idx as usize) < self.slots.len(),
                "tail_idx out of bounds for self.slots"
            );

            let next_free_idx = unsafe {
                self.slots
                    .get_unchecked(free_idx as usize)
                    .as_free_unchecked()
            };

            self.free_head = *next_free_idx;
            free_idx
        };

        let tail_idx = self.tail;

        if tail_idx != u32::MAX {
            // Safety: since we previously checked if tail_idx != u32::MAX (meaning if its Some)
            // we can safely do unsafe enum extraction. Any UB means there is a bug in updating
            // the tail_idx/self.tail value somewhere else
            debug_assert!(
                (tail_idx as usize) < self.slots.len(),
                "tail_idx out of bounds for self.slots"
            );
            unsafe {
                let (_, _, next) = self
                    .slots
                    .get_unchecked_mut(tail_idx as usize)
                    .as_occupied_unchecked_mut();

                *next = insert_idx;
            }
        }

        let new_slot = Slot::Occupied {
            data,
            prev: tail_idx,
            next: u32::MAX,
        };

        if insert_idx < self.slots.len() as u32 {
            // Safety: previously did bounds checks via self.slots.len() already. We also guarantee
            // insert_idx to be either inside the array bounds or exactly 1 out of bounds with
            // index self.slots.len()
            unsafe {
                let insert_slot_ref = self.slots.get_unchecked_mut(insert_idx as usize);
                *insert_slot_ref = new_slot;
            }
        } else {
            self.slots.push(new_slot);
        }

        if self.head == u32::MAX {
            self.head = insert_idx;
        }

        self.tail = insert_idx;
        self.capacity += 1;

        insert_idx
    }

    fn remove(&mut self, remove_idx: Self::Utype) {
        // Safety: tbh there is no safety here protecting the api. I just want to pinky promise to
        // myself that im never doing anything like double frees which could corrupt the slotmap.
        // Since this is for a portfolio and testing optimizations, im gonna do it anyway.
        debug_assert!(
            self.slots
                .get(remove_idx as usize)
                .is_some_and(|s| matches!(s, Slot::Occupied { .. })),
            "Attempted to remove an empty or invalid slot, this might be a double free bug or something similar."
        );

        let (curr_prev, curr_next) = unsafe {
            let (_, prev, next) = self
                .slots
                .get_unchecked(remove_idx as usize)
                .as_occupied_unchecked();

            (*prev, *next)
        };

        if curr_prev != u32::MAX {
            unsafe {
                let (_, _, next) = self
                    .slots
                    .get_unchecked_mut(curr_prev as usize)
                    .as_occupied_unchecked_mut();
                *next = curr_next;
            }
        }

        if curr_next != u32::MAX {
            unsafe {
                let (_, prev, _) = self
                    .slots
                    .get_unchecked_mut(curr_next as usize)
                    .as_occupied_unchecked_mut();
                *prev = curr_prev;
            }
        }

        unsafe {
            let remove_slot_ref = self.slots.get_unchecked_mut(remove_idx as usize);
            *remove_slot_ref = Slot::Free {
                next_free: self.free_head,
            };
        };

        self.free_head = remove_idx;

        if curr_next == u32::MAX {
            self.tail = curr_prev;
        }

        if curr_prev == u32::MAX {
            self.head = curr_next;
        }

        self.capacity -= 1;
    }

    #[allow(clippy::cast_possible_truncation)]
    fn total(&self) -> usize {
        self.slots.len()
    }

    fn capacity(&self) -> usize {
        self.capacity as usize
    }

    fn is_empty(&self) -> bool {
        self.capacity == 0
    }

    fn get(&self, index: usize) -> Option<&Self::Data> {
        let slot_ref = unsafe { self.slots.get_unchecked(index) };

        if let Slot::Occupied { data, .. } = slot_ref {
            Some(data)
        } else {
            None
        }
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Data> {
        let slot_ref = unsafe { self.slots.get_unchecked_mut(index) };

        if let Slot::Occupied { data, .. } = slot_ref {
            Some(data)
        } else {
            None
        }
    }
}

impl<T> Linkable for Slot<T> {
    fn prev(&self) -> Option<usize> {
        if let Self::Occupied { prev, .. } = self
            && *prev != u32::MAX
        {
            Some(*prev as usize)
        } else {
            None
        }
    }

    fn next(&self) -> Option<usize> {
        if let Self::Occupied { next, .. } = self
            && *next != u32::MAX
        {
            Some(*next as usize)
        } else {
            None
        }
    }
}

impl<T: PartialEq> TestableSlotMap for SlotMapUnsafe<T> {
    type Data = T;
    type Utype = u32;

    fn head(&self) -> Option<Self::Utype> {
        if self.head == u32::MAX {
            None
        } else {
            Some(self.head)
        }
    }

    fn tail(&self) -> Option<Self::Utype> {
        if self.tail == u32::MAX {
            None
        } else {
            Some(self.tail)
        }
    }

    fn free_head(&self) -> Option<Self::Utype> {
        if self.free_head == u32::MAX {
            None
        } else {
            Some(self.free_head)
        }
    }

    fn is_occupied(&self, index: usize, check_data: T) -> bool {
        let Some(slot) = self.slots.get(index) else {
            return false;
        };

        match slot {
            Slot::Free { .. } => false,
            Slot::Occupied { data, .. } => *data == check_data,
        }
    }

    fn get_link(&self, index: usize) -> Option<&impl Linkable> {
        let slot = self.slots.get(index)?;

        if matches!(slot, Slot::Free { .. }) {
            return None;
        }

        Some(slot)
    }
}

pub struct ArenaIter<'a, T> {
    arena: &'a SlotMapUnsafe<T>,
    current: u32,
}

impl<'a, T> Iterator for ArenaIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == u32::MAX {
            return None;
        }

        let index = self.current as usize;

        if let Some(Slot::Occupied { data, next, .. }) = self.arena.slots.get(index) {
            self.current = *next;
            return Some(data);
        }

        self.current = u32::MAX;
        None
    }
}

impl<'a, T> IntoIterator for &'a SlotMapUnsafe<T> {
    type Item = &'a T;
    type IntoIter = ArenaIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        ArenaIter {
            arena: self,
            current: self.head,
        }
    }
}
