use crate::{
    common::OrderIdU32,
    slot_map::{Linkable, NonMaxU32, SlotMap, TestableSlotMap},
};

pub struct SlotMapOptimized<T> {
    head: NonMaxU32,
    tail: NonMaxU32,
    free_head: NonMaxU32,
    slots: Vec<Slot<T>>,
    capacity: usize,
}

pub enum Slot<T> {
    Free {
        next_free: NonMaxU32,
    },
    Occupied {
        data: T,
        prev: NonMaxU32,
        next: NonMaxU32,
    },
}

impl<T> Slot<T> {
    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Occupied`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub const unsafe fn as_occupied_unchecked(&self) -> (&T, &NonMaxU32, &NonMaxU32) {
        match self {
            Self::Occupied { data, prev, next } => (data, prev, next),
            Self::Free { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Occupied`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub const unsafe fn as_occupied_unchecked_mut(
        &mut self,
    ) -> (&mut T, &mut NonMaxU32, &mut NonMaxU32) {
        match self {
            Self::Occupied { data, prev, next } => (data, prev, next),
            Self::Free { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Free`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub const unsafe fn as_free_unchecked(&self) -> &NonMaxU32 {
        match self {
            Self::Free { next_free } => next_free,
            Self::Occupied { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }

    /// # Safety
    ///
    /// Caller must guarantee the slot is `Slot::Free`.
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub const unsafe fn as_free_unchecked_mut(&mut self) -> &mut NonMaxU32 {
        match self {
            Self::Free { next_free } => next_free,
            Self::Occupied { .. } => unsafe { std::hint::unreachable_unchecked() },
        }
    }
}

impl<T> SlotMapOptimized<T> {
    #[must_use]
    pub fn iter(&self) -> SlotMapIter<'_, T> {
        self.into_iter()
    }

    #[must_use]
    pub fn iter_mut(&mut self) -> SlotMapIterMut<'_, T> {
        SlotMapIterMut {
            iter: self.slots.iter_mut(),
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            head: NonMaxU32::new_none(),
            tail: NonMaxU32::new_none(),
            free_head: NonMaxU32::new_none(),
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

impl<T> Default for SlotMapOptimized<T> {
    fn default() -> Self {
        Self {
            head: NonMaxU32::new_none(),
            tail: NonMaxU32::new_none(),
            free_head: NonMaxU32::new_none(),
            slots: vec![],
            capacity: 0,
        }
    }
}

impl<T> SlotMap for SlotMapOptimized<T> {
    type Id = OrderIdU32;
    type Data = T;

    fn new() -> Self {
        Self {
            head: NonMaxU32::new_none(),
            tail: NonMaxU32::new_none(),
            free_head: NonMaxU32::new_none(),
            slots: vec![],
            capacity: 0,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn insert(&mut self, data: Self::Data) -> Self::Id {
        let free_idx = self.free_head;

        let insert_idx = if free_idx.is_none() {
            self.slots.len() as u32
        } else {
            // Safety: since we previously checked if free_idx == u32::MAX (meaning if its None)
            // we can safely do unsafe enum extraction. Any UB means there is a bug in defining
            // the free_idx/updating the self.free_head
            debug_assert!(
                (free_idx.0 as usize) < self.slots.len(),
                "tail_idx out of bounds for self.slots"
            );

            let next_free_idx = unsafe {
                self.slots
                    .get_unchecked(free_idx.0 as usize)
                    .as_free_unchecked()
            };

            self.free_head = *next_free_idx;
            free_idx.0
        };

        let tail_idx = self.tail;

        if tail_idx.is_some() {
            // Safety: since we previously checked if tail_idx != u32::MAX (meaning if its Some)
            // we can safely do unsafe enum extraction. Any UB means there is a bug in updating
            // the tail_idx/self.tail value somewhere else
            debug_assert!(
                (tail_idx.0 as usize) < self.slots.len(),
                "tail_idx out of bounds for self.slots"
            );
            unsafe {
                let (_, _, next) = self
                    .slots
                    .get_unchecked_mut(tail_idx.0 as usize)
                    .as_occupied_unchecked_mut();

                next.0 = insert_idx;
            }
        }

        let new_slot = Slot::Occupied {
            data,
            prev: tail_idx,
            next: NonMaxU32::new_none(),
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

        if self.head.is_none() {
            self.head.0 = insert_idx;
        }

        self.tail.0 = insert_idx;
        self.capacity += 1;

        OrderIdU32(insert_idx)
    }

    fn remove(&mut self, remove_idx: Self::Id) {
        // Safety: tbh there is no safety here protecting the api. I just want to pinky promise to
        // myself that im never doing anything like double frees which could corrupt the slotmap.
        // Since this is for a portfolio and testing optimizations, im gonna do it anyway.
        debug_assert!(
            self.slots
                .get(remove_idx.0 as usize)
                .is_some_and(|s| matches!(s, Slot::Occupied { .. })),
            "Attempted to remove an empty or invalid slot, this might be a double free bug or something similar."
        );

        let (curr_prev, curr_next) = unsafe {
            let (_, prev, next) = self
                .slots
                .get_unchecked(remove_idx.0 as usize)
                .as_occupied_unchecked();

            (*prev, *next)
        };

        if curr_prev.is_some() {
            unsafe {
                let (_, _, next) = self
                    .slots
                    .get_unchecked_mut(curr_prev.0 as usize)
                    .as_occupied_unchecked_mut();
                *next = curr_next;
            }
        }

        if curr_next.is_some() {
            unsafe {
                let (_, prev, _) = self
                    .slots
                    .get_unchecked_mut(curr_next.0 as usize)
                    .as_occupied_unchecked_mut();
                *prev = curr_prev;
            }
        }

        unsafe {
            let remove_slot_ref = self.slots.get_unchecked_mut(remove_idx.0 as usize);
            *remove_slot_ref = Slot::Free {
                next_free: self.free_head,
            };
        };

        self.free_head.0 = remove_idx.0;

        if curr_next.is_none() {
            self.tail = curr_prev;
        }

        if curr_prev.is_none() {
            self.head = curr_next;
        }

        self.capacity -= 1;
    }

    #[allow(clippy::cast_possible_truncation)]
    fn total(&self) -> usize {
        self.slots.len()
    }

    fn capacity(&self) -> usize {
        self.capacity
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
            && prev.is_some()
        {
            Some(prev.0 as usize)
        } else {
            None
        }
    }

    fn next(&self) -> Option<usize> {
        if let Self::Occupied { next, .. } = self
            && next.is_some()
        {
            Some(next.0 as usize)
        } else {
            None
        }
    }
}

impl<T: PartialEq> TestableSlotMap for SlotMapOptimized<T> {
    type Data = T;
    type Utype = u32;

    fn head(&self) -> Option<Self::Utype> {
        if self.head.is_none() {
            None
        } else {
            Some(self.head.0)
        }
    }

    fn tail(&self) -> Option<Self::Utype> {
        if self.tail.is_none() {
            None
        } else {
            Some(self.tail.0)
        }
    }

    fn free_head(&self) -> Option<Self::Utype> {
        if self.free_head.is_none() {
            None
        } else {
            Some(self.free_head.0)
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

pub struct SlotMapIter<'a, T> {
    slot_map: &'a SlotMapOptimized<T>,
    current: u32,
}

impl<'a, T> Iterator for SlotMapIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == u32::MAX {
            return None;
        }

        if let Some(Slot::Occupied { data, next, .. }) =
            self.slot_map.slots.get(self.current as usize)
        {
            self.current = next.0;
            return Some(data);
        }

        self.current = u32::MAX;
        None
    }
}

impl<'a, T> IntoIterator for &'a SlotMapOptimized<T> {
    type Item = &'a T;
    type IntoIter = SlotMapIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        SlotMapIter {
            slot_map: self,
            current: self.head.0,
        }
    }
}

pub struct SlotMapIterMut<'a, T> {
    iter: std::slice::IterMut<'a, Slot<T>>,
}

impl<'a, T> Iterator for SlotMapIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        for slot in self.iter.by_ref() {
            if let Slot::Occupied { data, .. } = slot {
                return Some(data);
            }
        }
        None
    }
}

impl<'a, T> IntoIterator for &'a mut SlotMapOptimized<T> {
    type Item = &'a mut T;
    type IntoIter = SlotMapIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
