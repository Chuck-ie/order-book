pub struct SlotMap<S: Slot> {
    pub slots: Vec<S>,
    pub head: u32,
    pub tail: u32,
    pub next_free: u32,
    capacity: usize,
}

pub trait Slot {}

impl<S: Slot> SlotMap<S> {
    #[must_use]
    pub const fn head(&self) -> u32 {
        self.head
    }

    #[must_use]
    pub const fn tail(&self) -> u32 {
        self.tail
    }

    #[must_use]
    pub const fn next_free(&self) -> u32 {
        self.next_free
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

    pub fn insert(&mut self, order_id: u32) -> usize {
        let free_idx = self.next_free;

        let insert_idx = if free_idx != u32::MAX {
            // let free_idx = free_idx;

            let slot = unsafe { self.slots.get_unchecked_mut(free_idx as usize) };

            // self.next_free = slot.
            free_idx as usize
        } else {
            self.total()
        };

        // let insert_idx = if let Some(free_idx) = free_idx {
        //     let free_idx = free_idx.to_raw();
        //     let slot = unsafe {
        //         self.slots
        //             .get_unchecked_mut(free_idx)
        //             .as_free_unchecked_mut()
        //     };
        //     self.next_free = Some(NonZeroIndex::from_raw(slot.value() as usize));
        //     free_idx
        // } else {
        //     self.total()
        // };

        0
    }

    // #[must_use]
    // #[allow(clippy::cast_possible_truncation)]
    // pub fn insert(&mut self, order_id: u64) -> usize {
    //     let free_idx = self.next_free;
    //
    //     let insert_idx = if let Some(free_idx) = free_idx {
    //         let free_idx = free_idx.to_raw();
    //         let slot = unsafe {
    //             self.slots
    //                 .get_unchecked_mut(free_idx)
    //                 .as_free_unchecked_mut()
    //         };
    //         self.next_free = Some(NonZeroIndex::from_raw(slot.value() as usize));
    //         free_idx
    //     } else {
    //         self.total()
    //     };
    //
    //     let tail_idx = self.tail;
    //     let some_insert_idx = Some(NonZeroIndex::from_raw(insert_idx));
    //
    //     if let Some(tail_idx) = tail_idx {
    //         let tail_idx = tail_idx.to_raw();
    //         let tail_next_idx = unsafe { self.links.get_unchecked_mut(tail_idx) };
    //         tail_next_idx.next = some_insert_idx;
    //     }
    //
    //     let new_link = Link::from_prev_tail(tail_idx);
    //
    //     if insert_idx < self.total() {
    //         unsafe {
    //             self.slots
    //                 .get_unchecked_mut(insert_idx)
    //                 .make_occupied_unchecked_mut()
    //                 .set_value(order_id);
    //
    //             *self.links.get_unchecked_mut(insert_idx) = new_link;
    //         }
    //     } else {
    //         self.slots.push(Slot::<Occupied>::new(order_id).to_tagged());
    //         self.links.push(new_link);
    //     }
    //
    //     if self.head.is_none() {
    //         self.head = some_insert_idx;
    //     }
    //
    //     self.tail = some_insert_idx;
    //     self.capacity += 1;
    //     insert_idx
    // }
}
