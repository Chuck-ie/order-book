use crate::final_ver::arena_slot_map::ArenaSlot;

pub struct ArenaSlotAllocator {
    slots: Vec<ArenaSlot>,
    free_stack: Vec<usize>,
    chunk_count: usize,
    chunk_size: usize,
}

#[repr(transparent)]
pub struct ArenaChunkIndex(pub usize);

// #[derive(Clone, Copy)]
// #[repr(transparent)]
// pub struct ArenaIndex(pub u32);

#[derive(Clone, Copy)]
pub struct ArenaIndex {
    pub generation: u32,
    pub index: u32,
}

impl ArenaSlotAllocator {
    #[must_use]
    pub fn new(chunk_count: usize, chunk_size: usize) -> Self {
        Self {
            slots: Vec::with_capacity(chunk_count * chunk_size),

            // reverse the indexes, so when we call .pop() we get index 0, not chunk_count first
            free_stack: (0..chunk_count).rev().collect(),
            chunk_count,
            chunk_size,
        }
    }

    /// # Safety
    ///
    /// the caller needs to make sure that the allocator allocates enough chunks upfront.
    /// this function does not make sure that there is another chunk left to claim
    pub unsafe fn claim_chunk(&mut self) -> ArenaChunkIndex {
        // TODO: add dynamic arena resizing, or maybe just claim more via virtual memory but commit less?
        debug_assert!(!self.free_stack.is_empty(), "ArenaSlotAllocator overflowed");

        let Some(chunk_index) = self.free_stack.pop() else {
            unsafe { std::hint::unreachable_unchecked() }
        };

        ArenaChunkIndex(chunk_index)
    }

    pub fn release_chunk(&mut self, chunk_index: &ArenaChunkIndex) {
        self.free_stack.push(chunk_index.0);
    }

    #[inline(always)]
    #[must_use]
    pub const fn chunk_count(&self) -> usize {
        self.chunk_count
    }

    #[inline(always)]
    #[must_use]
    pub const fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<&ArenaSlot> {
        self.slots.get(index)
    }

    #[must_use]
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ArenaSlot> {
        self.slots.get_mut(index)
    }

    /// # Safety
    ///
    /// the called needs to make sure that index is in bounds of the allocated space
    /// in my case this should be fine as I allocate more than enough slots and program
    /// start + my ``ArenaSlotMap`` data structure is supposed to manage indices correctly
    #[must_use]
    pub unsafe fn get_unchecked(&self, index: usize) -> &ArenaSlot {
        unsafe { self.slots.get_unchecked(index) }
    }

    /// # Safety
    ///
    /// the called needs to make sure that index is in bounds of the allocated space
    /// in my case this should be fine as I allocate more than enough slots and program
    /// start + my ``ArenaSlotMap`` data structure is supposed to manage indices correctly
    #[must_use]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut ArenaSlot {
        unsafe { self.slots.get_unchecked_mut(index) }
    }
}
