use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    fmt,
    sync::atomic::{AtomicUsize, Ordering::Relaxed},
};

use serde::{Deserialize, Serialize};

thread_local! {
    static IS_PROFILING: Cell<bool> = const { Cell::new(false) };
}

#[derive(Default)]
pub struct SMemProfGuard(());

impl SMemProfGuard {
    #[must_use]
    pub fn new() -> Self {
        SMEM_PROF.guard_count.fetch_add(1, Relaxed);
        IS_PROFILING.with(|cell| cell.set(true));
        Self(())
    }
}

impl Drop for SMemProfGuard {
    fn drop(&mut self) {
        IS_PROFILING.with(|cell| cell.set(false));
    }
}

#[global_allocator]
pub static SMEM_PROF: SMemProf = SMemProf::zeroed();

/// ``SMemProf`` stands for Simple-Memory-Profiler
#[derive(Default)]
pub struct SMemProf {
    pub guard_count: AtomicUsize,
    alloc_count: AtomicUsize,
    pub alloc_bytes: AtomicUsize,
    dealloc_count: AtomicUsize,
    pub dealloc_bytes: AtomicUsize,
    grow_count: AtomicUsize,
    pub grow_bytes: AtomicUsize,
}

impl SMemProf {
    #[must_use]
    pub const fn zeroed() -> Self {
        Self {
            guard_count: AtomicUsize::new(0),
            alloc_count: AtomicUsize::new(0),
            alloc_bytes: AtomicUsize::new(0),
            dealloc_count: AtomicUsize::new(0),
            dealloc_bytes: AtomicUsize::new(0),
            grow_count: AtomicUsize::new(0),
            grow_bytes: AtomicUsize::new(0),
        }
    }

    pub fn reset(&self) {
        self.guard_count.store(0, Relaxed);
        self.alloc_count.store(0, Relaxed);
        self.alloc_bytes.store(0, Relaxed);
        self.dealloc_count.store(0, Relaxed);
        self.dealloc_bytes.store(0, Relaxed);
        self.grow_count.store(0, Relaxed);
        self.grow_bytes.store(0, Relaxed);
    }

    #[must_use]
    pub fn as_row(
        &self,
        engine: &str,
        total_levels: usize,
        orders_per_level: usize,
    ) -> SMemProfRow {
        SMemProfRow {
            engine: engine.to_string(),
            total_levels,
            orders_per_level,
            alloc_count: self.alloc_count.load(Relaxed),
            alloc_bytes: self.alloc_bytes.load(Relaxed),
            dealloc_count: self.dealloc_count.load(Relaxed),
            dealloc_bytes: self.dealloc_bytes.load(Relaxed),
            grow_count: self.grow_count.load(Relaxed),
            grow_bytes: self.grow_bytes.load(Relaxed),
        }
    }
}

impl fmt::Debug for SMemProf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let alloc_bytes = self.alloc_bytes.load(Relaxed);
        let dealloc_bytes = self.dealloc_bytes.load(Relaxed);

        f.debug_struct("BenchStatAlloc")
            .field("guard_count", &self.guard_count.load(Relaxed))
            .field("alloc_count", &self.alloc_count.load(Relaxed))
            .field("alloc_bytes", &ReadbleBytes(alloc_bytes))
            .field("dealloc_count", &self.dealloc_count.load(Relaxed))
            .field("dealloc_bytes", &ReadbleBytes(dealloc_bytes))
            .field("grow_count", &self.grow_count.load(Relaxed))
            .field("grow_bytes", &ReadbleBytes(self.grow_bytes.load(Relaxed)))
            .finish()
    }
}

struct ReadbleBytes(usize);

impl fmt::Debug for ReadbleBytes {
    #[allow(clippy::cast_precision_loss)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0 as f64;
        let kb = 1024.0;
        let mb = kb * 1024.0;
        let gb = mb * 1024.0;

        if bytes >= gb {
            write!(f, "{:.2} GB", bytes / gb)
        } else if bytes >= mb {
            write!(f, "{:.2} MB", bytes / mb)
        } else if bytes >= kb {
            write!(f, "{:.2} KB", bytes / kb)
        } else {
            write!(f, "{} B", self.0)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SMemProfRow {
    pub engine: String,
    pub total_levels: usize,
    pub orders_per_level: usize,
    pub alloc_count: usize,
    pub alloc_bytes: usize,
    pub dealloc_count: usize,
    pub dealloc_bytes: usize,
    pub grow_count: usize,
    pub grow_bytes: usize,
}

unsafe impl GlobalAlloc for SMemProf {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if IS_PROFILING.with(Cell::get) {
            self.alloc_count.fetch_add(1, Relaxed);
            self.alloc_bytes.fetch_add(layout.size(), Relaxed);
        }

        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if IS_PROFILING.with(Cell::get) {
            self.dealloc_count.fetch_add(1, Relaxed);
            self.dealloc_bytes.fetch_add(layout.size(), Relaxed);
        }

        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if IS_PROFILING.with(Cell::get) && new_size > layout.size() {
            self.grow_count.fetch_add(1, Relaxed);
            self.grow_bytes.fetch_add(new_size - layout.size(), Relaxed);
        }

        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
