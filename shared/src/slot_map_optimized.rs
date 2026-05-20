use crate::{Linkable, SlotMap, TestableSlotMap};

pub struct SlotMapOptimized<T> {
    pub head: u32,
    pub tail: u32,
    pub free_head: u32,
    slots: Vec<Slot<T>>,
    pub links: Vec<Option<Link>>,
    capacity: u32,
}

pub enum Slot<T> {
    Occupied(T),
    Free(u32),
}

pub struct Link {
    pub prev: u32,
    pub next: u32,
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
impl<T> SlotMapOptimized<T> {
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
            links: Vec::with_capacity(capacity),
            capacity: 0,
        }
    }
}

impl<T> Default for SlotMapOptimized<T> {
    fn default() -> Self {
        Self {
            head: u32::MAX,
            tail: u32::MAX,
            free_head: u32::MAX,
            slots: vec![],
            links: vec![],
            capacity: 0,
        }
    }
}

impl<T> SlotMap for SlotMapOptimized<T> {
    type Data = T;
    type Utype = u32;

    fn new() -> Self {
        Self {
            head: u32::MAX,
            tail: u32::MAX,
            free_head: u32::MAX,
            slots: vec![],
            links: vec![],
            capacity: 0,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn insert(&mut self, data: Self::Data) -> Self::Utype {
        let free_idx = self.free_head;

        let insert_idx = if free_idx == u32::MAX {
            self.total() as u32
        } else {
            let Slot::Free(next_free_idx) = self.slots[free_idx as usize] else {
                unreachable!("missing free slot");
            };

            self.free_head = next_free_idx;
            free_idx
        };

        let tail_idx = self.tail;

        if tail_idx != u32::MAX
            && let Some(tail_link) = &mut self.links[tail_idx as usize]
        {
            tail_link.next = insert_idx;
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

        if self.head == u32::MAX {
            self.head = insert_idx;
        }

        self.tail = insert_idx;
        self.capacity += 1;

        insert_idx
    }

    fn remove(&mut self, remove_idx: Self::Utype) {
        let Some(Some(curr_link)) = self.links.get_mut(remove_idx as usize) else {
            return;
        };

        let curr_prev = curr_link.prev;
        let curr_next = curr_link.next;

        if curr_prev != u32::MAX
            && let Some(Some(prev_link)) = self.links.get_mut(curr_prev as usize)
        {
            prev_link.next = curr_next;
        }

        if curr_next != u32::MAX
            && let Some(Some(next_link)) = self.links.get_mut(curr_next as usize)
        {
            next_link.prev = curr_prev;
        }

        self.slots[remove_idx as usize] = Slot::Free(self.free_head);
        self.links[remove_idx as usize] = None;
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

impl<T: PartialEq> TestableSlotMap for SlotMapOptimized<T> {
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
    pub const fn from_prev_tail(prev_tail: u32) -> Self {
        Self {
            prev: prev_tail,
            next: u32::MAX,
        }
    }
}

impl Linkable for Link {
    fn prev(&self) -> Option<usize> {
        if self.prev == u32::MAX {
            None
        } else {
            Some(self.prev as usize)
        }
    }
    fn next(&self) -> Option<usize> {
        if self.next == u32::MAX {
            None
        } else {
            Some(self.next as usize)
        }
    }
}

pub struct ArenaIter<'a, T> {
    arena: &'a SlotMapOptimized<T>,
    current: u32,
}

impl<'a, T> Iterator for ArenaIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current == u32::MAX {
            return None;
        }

        let index = self.current as usize;

        if let (Some(Slot::Occupied(data)), Some(Some(link))) =
            (self.arena.slots.get(index), self.arena.links.get(index))
        {
            self.current = link.next;
            return Some(data);
        }

        self.current = u32::MAX;
        None
    }
}

impl<'a, T> IntoIterator for &'a SlotMapOptimized<T> {
    type Item = &'a T;
    type IntoIter = ArenaIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        ArenaIter {
            arena: self,
            current: self.head,
        }
    }
}
