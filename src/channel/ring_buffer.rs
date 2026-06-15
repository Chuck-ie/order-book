use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    ops::Deref,
    sync::atomic::{AtomicBool, AtomicUsize},
};

// 1: no align: Config: 4P/4C | Total time: 8.233921789s, Throughput: 6072440 ops/sec
// 2: align h+t: Config: 4P/4C | Total time: 10.315543863s, Throughput: 4847054 ops/sec
// 3: align slot: Config: 4P/4C | Total time: 9.503548084s, Throughput: 5261192 ops/sec
// 4: align both: Config: 4P/4C | Total time: 7.406188957s, Throughput: 6751110 ops/sec

#[repr(align(64))]
#[derive(Default)]
pub struct CacheLinePadded<T>(T);

impl<T> Deref for CacheLinePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// #[repr(align(64))]
pub struct BufferSlot<T> {
    cell: UnsafeCell<MaybeUninit<T>>,
    pub(crate) written: AtomicBool,
}

impl<T> BufferSlot<T> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            cell: UnsafeCell::new(MaybeUninit::uninit()),
            written: AtomicBool::new(false),
        }
    }

    /// # Safety
    ///
    /// The caller has to make sure that writing is allowed and wont cause any race conditions with other threads
    #[inline]
    pub const unsafe fn write(&self, value: MaybeUninit<T>) {
        unsafe { self.cell.get().write(value) };
    }

    /// # Safety
    ///
    /// The caller has to make sure that reading is allowed and wont cause any race conditions with other threads
    #[inline]
    pub const unsafe fn read(&self) -> T {
        unsafe { self.cell.get().read().assume_init_read() }
    }
}

// TODO: check if const BUF_SIZE is preferred here. Originally I planned to use it for a fixed size,
// but heap allocated array, but that might cause problems so now its a dynamic size, but fixed at
// runtime array instead. The BUF_SIZE can still be used later though
/// see: `<https://en.wikipedia.org/wiki/Circular_buffer>` for implementation details
pub struct RingBuffer<T> {
    pub(crate) buffer: Box<[BufferSlot<T>]>,
    pub(crate) head: CacheLinePadded<AtomicUsize>,
    pub(crate) tail: CacheLinePadded<AtomicUsize>,
    pub(crate) capacity: usize,
}

unsafe impl<T: Send> Send for RingBuffer<T> {}
unsafe impl<T: Sync> Sync for RingBuffer<T> {}

impl<T> RingBuffer<T> {
    /// # Panics
    ///
    /// panics if the capacity is less than 2 or if its not a power of 2
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 1, "capacity must be greater than 1");
        assert!(capacity.is_power_of_two(), "capacity must be a power of 2");

        let buffer = (0..capacity).map(|_| BufferSlot::new()).collect();

        Self {
            buffer,
            head: CacheLinePadded(AtomicUsize::new(0)),
            tail: CacheLinePadded(AtomicUsize::new(0)),
            capacity: capacity - 1,
        }
    }

    // examples for when a thread is able to read or write
    /*  empty r==w:
     *  [0, 1, 2, 3]
     *   ^rw
     *  -> w CAN write, r can NOT read
     *
     *  full r-1==w
     *  [0, 1, 2, 3]
     *   ^r       ^w
     *  -> w can NOT write, r CAN read
     *
     *  else
     *  [0, 1, 2, 3]
     *   ^r    ^w
     *  -> w CAN write, r CAN read
     */
}
