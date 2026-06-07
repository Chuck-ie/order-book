use std::{cell::UnsafeCell, mem::MaybeUninit, ops::Deref, sync::atomic::AtomicUsize};

pub type BufferSlot<T> = CacheLinePadded<UnsafeCell<MaybeUninit<T>>>;

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

impl<T> RingBuffer<T> {
    /// # Panics
    ///
    /// panics if the ``BUF_SIZE`` is less than 2 or if its not a power of 2
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        assert!(capacity > 1, "capacity must be greater than 1");
        assert!(capacity.is_power_of_two(), "capacity must be a power of 2");

        let buffer = (0..capacity)
            .map(|_| CacheLinePadded(UnsafeCell::new(MaybeUninit::uninit())))
            .collect();

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

#[repr(align(64))]
#[derive(Default)]
pub struct CacheLinePadded<T>(T);

impl<T> Deref for CacheLinePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
