use crate::{Linkable, SlotMap, TestableSlotMap};

#[derive(Default)]
pub struct SlotMapNaive<T> {
    slots: Vec<Slot<T>>,
    pub head: Option<usize>,
    pub tail: Option<usize>,
    pub free_head: Option<usize>,
    capacity: usize,
}

pub enum Slot<T> {
    Occupied {
        data: T,
        prev: Option<usize>,
        next: Option<usize>,
    },
    Free(Option<usize>),
}

// ex1: [] -> [id], head: Some(0), tail: Some(0), free: None -> 0
// ex2: [id] -> [id, id], head: Some(0), tail: Some(1), free: None -> 1
// ex3: [id, id] -> [id, id, id], head: Some(0), tail: Some(2), free: None -> 2
// ex4: [id, id, id] -> [id, free(None), id], head: Some(0), tail: Some(2), free: Some(1)
// ex5: [id, free(None), id] -> [id, free(None), free(1)], head: Some(0), tail: Some(0), free: Some(2)
// ex6: [id, id, id] -> [id, id, free(None)], head: Some(0), tail: Some(1), free: Some(2)
// ex7: [id, id, id] -> [free(None), id, id], head: Some(1), tail: Some(2), free: Some(0)
// ex8: [id, free(None), id], free: Some(1) -> [id, id, id], free: None
// ex9: [id, free(None), free(1)], free: Some(2) -> [id, free(None), id], free: Some(1)
impl<T> SlotMapNaive<T> {
    #[must_use]
    pub fn iter(&self) -> NaiveArenaIter<'_, T> {
        self.into_iter()
    }
}

impl<T> SlotMap for SlotMapNaive<T> {
    type Data = T;
    type Utype = usize;

    fn new() -> Self {
        Self {
            slots: vec![],
            head: None,
            tail: None,
            free_head: None,
            capacity: 0,
        }
    }

    fn insert(&mut self, data: T) -> Self::Utype {
        let insert_idx = if let Some(free_idx) = self.free_head {
            let Some(Slot::Free(next_free_idx)) = self.slots.get(free_idx) else {
                unreachable!("missing free slot");
            };

            self.free_head = *next_free_idx;
            free_idx
        } else {
            self.slots.len()
        };

        if let Some(tail_idx) = self.tail {
            let Some(Slot::Occupied {
                next: tail_next_idx,
                ..
            }) = self.slots.get_mut(tail_idx)
            else {
                unreachable!("missing tail slot");
            };

            *tail_next_idx = Some(insert_idx);
        }

        let new_slot = Slot::new_tail(data, self.tail);

        if let Some(slot) = self.slots.get_mut(insert_idx) {
            *slot = new_slot;
        } else {
            self.slots.push(new_slot);
        }

        if self.head.is_none() {
            self.head = Some(insert_idx);
        }

        self.tail = Some(insert_idx);
        self.capacity += 1;
        insert_idx
    }

    fn remove(&mut self, remove_idx: Self::Utype) {
        let Some(curr_slot) = self.slots.get_mut(remove_idx) else {
            return;
        };

        let Slot::Occupied { prev, next, .. } = curr_slot else {
            return;
        };

        let curr_prev = *prev;
        let curr_next = *next;

        if let Some(prev_idx) = curr_prev {
            let Some(Slot::Occupied {
                next: prev_next_idx,
                ..
            }) = self.slots.get_mut(prev_idx)
            else {
                unreachable!("FIXME: missing prev slot");
            };

            *prev_next_idx = curr_next;
        }

        if let Some(next_idx) = curr_next {
            let Some(Slot::Occupied {
                prev: next_prev_idx,
                ..
            }) = self.slots.get_mut(next_idx)
            else {
                unreachable!("FIXME: missing next slot");
            };

            *next_prev_idx = curr_prev;
        }

        let new_free = Some(remove_idx);
        self.slots[remove_idx] = Slot::Free(self.free_head);
        self.free_head = new_free;

        if curr_next.is_none() {
            self.tail = curr_prev;
        }

        if curr_prev.is_none() {
            self.head = curr_next;
        }

        self.capacity -= 1;
    }

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
        let Some(Slot::Occupied { data, .. }) = self.slots.get(index) else {
            return None;
        };

        Some(data)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Data> {
        let Some(Slot::Occupied { data, .. }) = self.slots.get_mut(index) else {
            return None;
        };

        Some(data)
    }
}

impl<T: PartialEq> TestableSlotMap for SlotMapNaive<T> {
    type Data = T;
    type Utype = usize;

    fn head(&self) -> Option<Self::Utype> {
        self.head
    }

    fn tail(&self) -> Option<Self::Utype> {
        self.tail
    }

    fn free_head(&self) -> Option<Self::Utype> {
        self.free_head
    }

    fn is_occupied(&self, index: Self::Utype, data: T) -> bool {
        let Some(slot) = self.slots.get(index) else {
            return false;
        };

        match slot {
            Slot::Free(_) => false,
            Slot::Occupied {
                data: curr_data, ..
            } => curr_data == &data,
        }
    }

    fn get_link(&self, index: usize) -> Option<&impl Linkable> {
        self.slots
            .get(index)
            .filter(|slot| matches!(slot, Slot::Occupied { .. }))
    }
}

impl<T> Slot<T> {
    pub const fn new_tail(data: T, prev_tail: Option<usize>) -> Self {
        Self::Occupied {
            data,
            prev: prev_tail,
            next: None,
        }
    }
}

impl<T> Linkable for Slot<T> {
    fn prev(&self) -> Option<usize> {
        match self {
            Self::Occupied { prev, .. } => *prev,
            Self::Free(_) => None,
        }
    }
    fn next(&self) -> Option<usize> {
        match self {
            Self::Occupied { next, .. } => *next,
            Self::Free(_) => None,
        }
    }
}

pub struct NaiveArenaIter<'a, T> {
    arena: &'a SlotMapNaive<T>,
    current: Option<usize>,
}

impl<'a, T> Iterator for NaiveArenaIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.current?;

        if let Slot::Occupied { data, next, .. } = &self.arena.slots[index] {
            self.current = *next;
            Some(data)
        } else {
            None
        }
    }
}

impl<'a, T> IntoIterator for &'a SlotMapNaive<T> {
    type Item = &'a T;
    type IntoIter = NaiveArenaIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        NaiveArenaIter {
            arena: self,
            current: self.head,
        }
    }
}
