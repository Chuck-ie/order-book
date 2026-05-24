use crate::{Linkable, SlotMap, TestableSlotMap, slot_map::NonMaxU32};

pub struct SlotMapStandard<T> {
    pub head: NonMaxU32,
    pub tail: NonMaxU32,
    pub free_head: NonMaxU32,
    slots: Vec<Slot<T>>,
    pub links: Vec<Option<Link>>,
    capacity: usize,
}

pub enum Slot<T> {
    Occupied(T),
    Free(NonMaxU32),
}

pub struct Link {
    pub prev: NonMaxU32,
    pub next: NonMaxU32,
}

impl<T> SlotMapStandard<T> {
    #[must_use]
    pub fn iter(&self) -> SlotMapIter<'_, T> {
        self.into_iter()
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            head: NonMaxU32::new_none(),
            tail: NonMaxU32::new_none(),
            free_head: NonMaxU32::new_none(),
            slots: Vec::with_capacity(capacity),
            links: Vec::with_capacity(capacity),
            capacity: 0,
        }
    }
}

impl<T> Default for SlotMapStandard<T> {
    fn default() -> Self {
        Self {
            head: NonMaxU32::new_none(),
            tail: NonMaxU32::new_none(),
            free_head: NonMaxU32::new_none(),
            slots: vec![],
            links: vec![],
            capacity: 0,
        }
    }
}

impl<T> SlotMap for SlotMapStandard<T> {
    type Data = T;
    type Id = u32;

    fn new() -> Self {
        Self {
            head: NonMaxU32::new_none(),
            tail: NonMaxU32::new_none(),
            free_head: NonMaxU32::new_none(),
            slots: vec![],
            links: vec![],
            capacity: 0,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn insert(&mut self, data: Self::Data) -> Self::Id {
        let free_idx = self.free_head;

        let insert_idx = if free_idx.is_none() {
            self.total() as u32
        } else {
            let Slot::Free(next_free_idx) = self.slots[free_idx.0 as usize] else {
                unreachable!("missing free slot");
            };

            self.free_head = next_free_idx;
            free_idx.0
        };

        let tail_idx = self.tail;

        if tail_idx.is_some()
            && let Some(tail_link) = &mut self.links[tail_idx.0 as usize]
        {
            tail_link.next.0 = insert_idx;
        }

        let new_slot = Slot::Occupied(data);
        let new_link = Some(Link::from_prev_tail(tail_idx));

        if insert_idx < self.total() as u32 {
            self.slots[insert_idx as usize] = new_slot;
            self.links[insert_idx as usize] = new_link;
        } else {
            self.slots.push(new_slot);
            self.links.push(new_link);
        }

        if self.head.is_none() {
            self.head.0 = insert_idx;
        }

        self.tail.0 = insert_idx;
        self.capacity += 1;

        insert_idx
    }

    fn remove(&mut self, remove_idx: Self::Id) {
        let Some(Some(curr_link)) = self.links.get_mut(remove_idx as usize) else {
            return;
        };

        let curr_prev = curr_link.prev;
        let curr_next = curr_link.next;

        if curr_prev.is_some()
            && let Some(Some(prev_link)) = self.links.get_mut(curr_prev.0 as usize)
        {
            prev_link.next = curr_next;
        }

        if curr_next.is_some()
            && let Some(Some(next_link)) = self.links.get_mut(curr_next.0 as usize)
        {
            next_link.prev = curr_prev;
        }

        self.slots[remove_idx as usize] = Slot::Free(self.free_head);
        self.links[remove_idx as usize] = None;
        self.free_head.0 = remove_idx;

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
        let Some(Slot::Occupied(data)) = self.slots.get(index) else {
            return None;
        };

        Some(data)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Data> {
        let Some(Slot::Occupied(data)) = self.slots.get_mut(index) else {
            return None;
        };

        Some(data)
    }
}

impl<T: PartialEq> TestableSlotMap for SlotMapStandard<T> {
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

    fn is_occupied(&self, index: usize, data: T) -> bool {
        let Some(slot) = self.slots.get(index) else {
            return false;
        };

        match slot {
            Slot::Free(_) => false,
            Slot::Occupied(curr_data) => curr_data == &data,
        }
    }

    fn get_link(&self, index: usize) -> Option<&impl Linkable> {
        self.links[index].as_ref()
    }
}

impl Link {
    #[must_use]
    pub const fn from_prev_tail(prev_tail: NonMaxU32) -> Self {
        Self {
            prev: prev_tail,
            next: NonMaxU32::new_none(),
        }
    }
}

impl Linkable for Link {
    fn prev(&self) -> Option<usize> {
        if self.prev.is_none() {
            None
        } else {
            Some(self.prev.0 as usize)
        }
    }
    fn next(&self) -> Option<usize> {
        if self.next.is_none() {
            None
        } else {
            Some(self.next.0 as usize)
        }
    }
}

pub struct SlotMapIter<'a, T> {
    slot_map: &'a SlotMapStandard<T>,
    current: u32,
}

impl<'a, T> Iterator for SlotMapIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == u32::MAX {
            return None;
        }

        let index = self.current as usize;

        if let (Some(Slot::Occupied(data)), Some(Some(link))) = (
            self.slot_map.slots.get(index),
            self.slot_map.links.get(index),
        ) {
            self.current = link.next.0;
            return Some(data);
        }

        self.current = u32::MAX;
        None
    }
}

impl<'a, T> IntoIterator for &'a SlotMapStandard<T> {
    type Item = &'a T;
    type IntoIter = SlotMapIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        SlotMapIter {
            slot_map: self,
            current: self.head.0,
        }
    }
}
