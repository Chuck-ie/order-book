use std::{marker::PhantomData, mem};

use memmap2::{MmapMut, MmapOptions};

use crate::final_ver::arena_slot_map::ArenaSlot;

// mmap_memory helps a little bit sometimes with dTLB misses and L1-dcache misses in my benchmark tests.
// i need to run the following command to not make it actively worse:
// ❯ sudo sh -c 'echo always > /sys/kernel/mm/transparent_hugepage/enabled'
//
// check for info of my own system
// ❯ cat /proc/meminfo | grep Huge
// AnonHugePages:     53248 kB
// ShmemHugePages:        0 kB
// FileHugePages:     83968 kB
// HugePages_Total:       0
// HugePages_Free:        0
// HugePages_Rsvd:        0
// HugePages_Surp:        0
// Hugepagesize:       2048 kB
// Hugetlb:               0 kB
//
// allows using 2MB hugepages to whatever i need/allocate in my arena
// ❯ sudo sysctl -w vm.nr_hugepages=256
// ...
// HugePages_Total:     256
// HugePages_Free:      256
// ...
// Hugetlb:          524288 kB
//
//
// for 1GB huge pages check this:
//
// set to 1 page
// sudo bash -c 'echo 1 > /sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages'
//
// set to 0 pages
// sudo bash -c 'echo 0 > /sys/kernel/mm/hugepages/hugepages-1048576kB/nr_hugepages'
//
// should say 1
// grep "" /sys/kernel/mm/hugepages/hugepages-1048576kB/*
//
// verify during runtime of the program if it uses the pages (should change from 1 to 0 and back once the program stops)
// cat /sys/kernel/mm/hugepages/hugepages-1048576kB/free_hugepages
pub struct ArenaSlotAllocator<T> {
    mmap_memory: MmapMut,
    pub free_stack: Vec<usize>,
    chunk_count: usize,
    chunk_size: usize,
    _marker: PhantomData<T>,
}

#[repr(transparent)]
pub struct ArenaChunkIndex(pub usize);

#[derive(Clone)]
pub struct ArenaId {
    pub generation: u32,
    pub index: u32,
}

impl<T> ArenaSlotAllocator<T> {
    /// # Panics
    ///
    /// panics if the mmap memory mapping files for the requested size
    #[must_use]
    pub fn new(chunk_count: usize, chunk_size: usize) -> Self {
        let total_bytes = chunk_count * chunk_size * mem::size_of::<ArenaSlot<T>>();
        // let total_bytes = 1 << 30;

        let mmap_memory = MmapOptions::new()
            .len(total_bytes)
            // from the memmap2 example, this is for 2MB huge pages like my system has
            .huge(Some(21))
            // .huge(Some(30))
            .populate()
            .map_anon()
            .expect("Failed to mmap anonymous memory");

        Self {
            mmap_memory,
            // reverse the indexes, so when we call .pop() we get index 0, not chunk_count first
            free_stack: (0..chunk_count).rev().collect(),
            chunk_count,
            chunk_size,
            _marker: PhantomData,
        }
    }

    /// # Safety
    ///
    /// the caller needs to make sure that the allocator allocates enough chunks upfront.
    /// this function does not make sure that there is another chunk left to claim
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub unsafe fn claim_chunk(&mut self) -> ArenaChunkIndex {
        // TODO: add dynamic arena resizing, or maybe just claim more via virtual memory but commit less?
        debug_assert!(!self.free_stack.is_empty(), "ArenaSlotAllocator overflowed");

        let Some(chunk_index) = self.free_stack.pop() else {
            unsafe { std::hint::unreachable_unchecked() }
        };

        ArenaChunkIndex(chunk_index)
    }

    // this is in fact not needless Mr. Clippy, but prevents double frees
    // (or well releases) that could lead to UB
    #[allow(clippy::needless_pass_by_value)]
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn release_chunk(&mut self, chunk_index: ArenaChunkIndex) {
        self.free_stack.push(chunk_index.0);
    }

    #[must_use]
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub const fn chunk_count(&self) -> usize {
        self.chunk_count
    }

    #[must_use]
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub const fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    #[must_use]
    pub const fn free_count(&self) -> usize {
        self.free_stack.len()
    }

    #[must_use]
    pub const fn slot_count(&self) -> usize {
        self.chunk_count * self.chunk_size
        // self.slots.len()
    }

    // #[must_use]
    // pub fn get(&self, index: usize) -> Option<&ArenaSlot<T>> {
    //     // self.slots.get(index)
    // }

    #[must_use]
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<&ArenaSlot<T>> {
        unsafe {
            let raw_ptr = self.mmap_memory.as_ptr().cast::<ArenaSlot<T>>();
            raw_ptr.add(index).as_ref()
        }
    }

    // #[must_use]
    // pub fn get_mut(&mut self, index: usize) -> Option<&mut ArenaSlot<T>> {
    //     self.slots.get_mut(index)
    // }

    /// # Safety
    ///
    /// the called needs to make sure that index is in bounds of the allocated space
    /// in my case this should be fine as I allocate more than enough slots and program
    /// start + my ``ArenaSlotMap`` data structure is supposed to manage indices correctly
    #[must_use]
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> &ArenaSlot<T> {
        // unsafe { self.slots.get_unchecked(index) }
        unsafe {
            let raw_ptr = self.mmap_memory.as_ptr().cast::<ArenaSlot<T>>();
            &*raw_ptr.add(index)
        }
    }

    /// # Safety
    ///
    /// the called needs to make sure that index is in bounds of the allocated space
    /// in my case this should be fine as I allocate more than enough slots and program
    /// start + my ``ArenaSlotMap`` data structure is supposed to manage indices correctly
    #[must_use]
    #[allow(clippy::inline_always)]
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut ArenaSlot<T> {
        // unsafe { self.slots.get_unchecked_mut(index) }
        unsafe {
            let raw_ptr = self.mmap_memory.as_mut_ptr().cast::<ArenaSlot<T>>();
            &mut *raw_ptr.add(index)
        }
    }
}
