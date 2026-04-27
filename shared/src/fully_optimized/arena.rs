use crate::fully_optimized::{
    slot::{Occupied, Slot, Tagged},
    types::NonZeroIndex,
};

pub struct Arena {
    pub slots: Vec<Slot<Tagged>>,
    // pub head: ArenaIndex,
    // pub tail: ArenaIndex,
    pub head: Option<NonZeroIndex>,
    pub tail: Option<NonZeroIndex>,
    pub next_free: Option<NonZeroIndex>,
    // pub next_free: ArenaIndex,
    pub links: Vec<Link>,
    // pub prev: Vec<ArenaIndex>,
    // pub next: Vec<ArenaIndex>,
    capacity: usize,
}

pub struct Link {
    pub prev: Option<NonZeroIndex>,
    pub next: Option<NonZeroIndex>,
}

impl Arena {
    #[must_use]
    pub const fn head(&self) -> &Option<NonZeroIndex> {
        &self.head
    }

    #[must_use]
    pub const fn tail(&self) -> &Option<NonZeroIndex> {
        &self.tail
    }

    #[must_use]
    pub const fn next_free(&self) -> &Option<NonZeroIndex> {
        &self.next_free
    }

    #[must_use]
    pub const fn total(&self) -> usize {
        self.slots.len()
    }

    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.capacity == 0
    }

    #[must_use]
    pub fn is_occupied_unchecked(&self, index: usize) -> bool {
        // version 1:
        // self.slots.get(index).is_some_and(Slot::is_occupied)

        // version 2:
        unsafe { self.slots.get_unchecked(index).is_occupied() }
    }

    #[must_use]
    pub fn is_free_unchecked(&self, index: usize) -> bool {
        // version 1:
        // self.slots.get(index).is_some_and(Slot::is_free)

        // version 2:
        unsafe { self.slots.get_unchecked(index).is_free() }
    }

    #[must_use]
    pub fn get_unchecked(&self, index: usize) -> &Slot<Tagged> {
        unsafe { self.slots.get_unchecked(index) }
    }

    #[must_use]
    pub fn get_unchecked_mut(&mut self, index: usize) -> &mut Slot<Tagged> {
        unsafe { self.slots.get_unchecked_mut(index) }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn insert(&mut self, order_id: u64) -> usize {
        let free_idx = self.next_free;

        let insert_idx = if let Some(free_idx) = free_idx {
            let free_idx = free_idx.to_raw();
            let slot = unsafe { self.slots.get_unchecked_mut(free_idx).as_free_unchecked_mut() };
            self.next_free = Some(NonZeroIndex::from_raw(slot.value() as usize));
            free_idx
        } else {
            self.total()
        };

        let tail_idx = self.tail;
        let some_insert_idx = Some(NonZeroIndex::from_raw(insert_idx));

        if let Some(tail_idx) = tail_idx {
            let tail_idx = tail_idx.to_raw();
            let tail_next_idx = unsafe { self.links.get_unchecked_mut(tail_idx) };
            tail_next_idx.next = some_insert_idx;
        }

        let new_link = Link::from_prev_tail(tail_idx);

        if insert_idx < self.total() {
            unsafe {
                self.slots
                    .get_unchecked_mut(insert_idx)
                    .make_occupied_unchecked_mut()
                    .set_value(order_id);

                *self.links.get_unchecked_mut(insert_idx) = new_link;
            }
        } else {
            self.slots.push(Slot::<Occupied>::new(order_id).to_tagged());
            self.links.push(new_link);
        }

        if self.head.is_none() {
            self.head = some_insert_idx;
        }

        self.tail = some_insert_idx;
        self.capacity += 1;
        insert_idx
    }

    pub fn remove(&mut self, index: usize) {
        let Some(node) = self.links.get_mut(index) else {
            return;
        };

        let prev = node.prev;
        let next = node.next;

        if let Some(prev_idx) = prev {
            unsafe {
                self.links.get_unchecked_mut(prev_idx.to_raw()).next = next;
            }
        }

        if let Some(next_idx) = next {
            unsafe {
                self.links.get_unchecked_mut(next_idx.to_raw()).prev = prev;
            }
        }

        unsafe {
            let removed_slot = self.slots.get_unchecked_mut(index).make_free_unchecked_mut();
            // removed_slot.set_value(self.next_free);
        }
    }

    //     self.slots[remove_idx as usize] = Slot::Free(self.free_head);
    //     self.links[remove_idx as usize] = None;
    //     self.free_head = remove_idx;
    //
    //     if curr_next == u32::MAX {
    //         self.tail = curr_prev;
    //     }
    //
    //     if curr_prev == u32::MAX {
    //         self.head = curr_next;
    //     }
    //
    //     self.capacity -= 1;
    // }
}

impl Link {
    #[must_use]
    pub const fn from_prev_tail(prev_tail: Option<NonZeroIndex>) -> Self {
        Self {
            prev: prev_tail,
            next: None,
        }
    }
}
