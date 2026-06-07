use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem::MaybeUninit,
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::channel::spinlock::Spinlock;

mod spinlock;

#[repr(align(64))]
#[derive(Default)]
pub struct CacheLinePadded<T>(T);

impl<T> Deref for CacheLinePadded<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type BufferSlot<T> = CacheLinePadded<UnsafeCell<MaybeUninit<T>>>;

// TODO: check if const BUF_SIZE is preferred here. Originally I planned to use it for a fixed size,
// but heap allocated array, but that might cause problems so now its a dynamic size, but fixed at
// runtime array instead. The BUF_SIZE can still be used later though
/// see: `<https://en.wikipedia.org/wiki/Circular_buffer>` for implementation details
pub struct RingBuffer<T> {
    buffer: Box<[BufferSlot<T>]>,
    pub head: CacheLinePadded<AtomicUsize>,
    pub tail: CacheLinePadded<AtomicUsize>,
    capacity: usize,
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

pub type MpmcChannel<T> = Channel<T, MultiProducer<T>, MultiConsumer<T>>;

pub struct Channel<T, P, C>
where
    P: Producer<Item = T> + FromBuffer<Item = T>,
    C: Consumer<Item = T> + FromBuffer<Item = T>,
{
    buffer: Arc<RingBuffer<T>>,
    producer: P,
    consumer: C,
}

impl<T, P, C> Channel<T, P, C>
where
    P: Producer<Item = T> + FromBuffer<Item = T>,
    C: Consumer<Item = T> + FromBuffer<Item = T>,
{
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let buffer = Arc::new(RingBuffer::with_capacity(capacity));

        Self {
            buffer: buffer.clone(),
            producer: P::new(&buffer),
            consumer: C::new(&buffer),
        }
    }

    #[must_use]
    pub fn split(self) -> (P, C) {
        let Self {
            buffer: _,
            producer,
            consumer,
        } = self;
        (producer, consumer)
    }
}

pub trait Producer {
    type Item;

    fn try_write(&self, value: Self::Item) -> bool;
}

pub struct MultiProducer<T> {
    state: Arc<RingBuffer<T>>,
}

impl<T> Deref for MultiProducer<T> {
    type Target = Arc<RingBuffer<T>>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> Producer for MultiProducer<T> {
    type Item = T;

    fn try_write(&self, value: Self::Item) -> bool {
        let mut spinlock = Spinlock::new();
        let mut curr_write_index = self.head.load(Ordering::Relaxed);

        while spinlock.spin() {
            let curr_read_index = self.tail.load(Ordering::Relaxed);

            // this is a faster modulo operation, which only works under the assumption that N is a power of 2
            if (curr_write_index + 1) & self.capacity == curr_read_index {
                return false;
            }

            let next_write_index = if curr_write_index == self.capacity {
                0
            } else {
                curr_write_index + 1
            };

            let result = self.head.compare_exchange_weak(
                curr_write_index,
                next_write_index,
                Ordering::SeqCst,
                Ordering::Relaxed,
            );

            match result {
                Ok(_) => {
                    unsafe {
                        let write_ptr = self.buffer.get_unchecked(curr_write_index).get();
                        write_ptr.write(MaybeUninit::new(value));
                    }

                    return true;
                }
                Err(updated_write_index) => curr_write_index = updated_write_index,
            }
        }

        false
    }
}

pub trait FromBuffer {
    type Item;

    fn new(buffer: &Arc<RingBuffer<Self::Item>>) -> Self;
}

pub trait Consumer {
    type Item;

    fn try_read(&self) -> Option<Self::Item>;
}

pub struct Single;
pub struct Multi;

pub trait Mode {}
impl Mode for Single {}
impl Mode for Multi {}

pub struct BufferHandle<T, M: Mode> {
    state: Arc<RingBuffer<T>>,
    _mode: PhantomData<M>,
}

impl<T, M: Mode> BufferHandle<T, M> {
    #[must_use]
    pub fn new(buffer: &Arc<RingBuffer<T>>) -> Self {
        Self {
            state: buffer.clone(),
            _mode: PhantomData,
        }
    }
}

impl<T, M: Mode> Deref for BufferHandle<T, M> {
    type Target = Arc<RingBuffer<T>>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> Consumer for BufferHandle<T, Single> {
    type Item = T;

    fn try_read(&self) -> Option<Self::Item> {
        todo!()
    }
}

impl<T> Consumer for BufferHandle<T, Multi> {
    type Item = T;

    fn try_read(&self) -> Option<Self::Item> {
        todo!()
    }
}

pub struct SingleConsumer<T> {
    state: Arc<RingBuffer<T>>,
}

impl<T> Deref for SingleConsumer<T> {
    type Target = Arc<RingBuffer<T>>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> Consumer for SingleConsumer<T> {
    type Item = T;

    fn try_read(&self) -> Option<Self::Item> {
        todo!()
    }
}

pub struct MultiConsumer<T> {
    state: Arc<RingBuffer<T>>,
}

impl<T> Deref for MultiConsumer<T> {
    type Target = Arc<RingBuffer<T>>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<T> Consumer for MultiConsumer<T> {
    type Item = T;

    fn try_read(&self) -> Option<Self::Item> {
        let mut spinlock = Spinlock::new();
        let mut curr_read_index = self.tail.load(Ordering::Relaxed);

        while spinlock.spin() {
            let curr_write_index = self.head.load(Ordering::Relaxed);

            if curr_read_index == curr_write_index {
                return None;
            }

            let next_read_index = if curr_read_index == self.capacity {
                0
            } else {
                curr_read_index + 1
            };

            let result = self.tail.compare_exchange_weak(
                curr_read_index,
                next_read_index,
                Ordering::SeqCst,
                Ordering::Relaxed,
            );

            match result {
                // Safety
                // we check that curr_read_index is allowed to read and always update it to be a valid
                // index inside our buffer, so we can always get the read_ptr and read from it
                Ok(_) => unsafe {
                    let read_ptr = self.buffer.get_unchecked(curr_read_index).get();
                    return Some(read_ptr.read().assume_init_read());
                },
                Err(updated_read_index) => {
                    curr_read_index = updated_read_index;
                }
            }
        }

        None
    }
}

// #[cfg(test)]
// mod ring_buffer_tests {
//     use crate::channel::{Channel, Consumer, MpmcChannel, MultiConsumer, MultiProducer, Producer};
//     use std::sync::atomic::Ordering;
//
//     macro_rules! test_channel_impl {
//         ($name:ident, $ty:ty) => {
//             mod $name {
//                 use super::*;
//
//                 #[test]
//                 fn test_init_valid() {
//                     super::init_valid::<$ty>();
//                 }
//
//                 #[test]
//                 #[should_panic(expected = "capacity must be greater than 1")]
//                 fn test_init_buf_eq_zero() {
//                     super::init_buf_eq_zero::<$ty>();
//                 }
//
//                 #[test]
//                 #[should_panic(expected = "capacity must be greater than 1")]
//                 fn test_init_buf_eq_one() {
//                     super::init_buf_eq_one::<$ty>();
//                 }
//
//                 #[test]
//                 fn test_cant_write_past_read_tail() {
//                     super::cant_write_past_read_tail::<$ty>();
//                 }
//
//                 #[test]
//                 fn test_write_index_advance_no_wrap() {
//                     super::write_index_advance_no_wrap::<$ty>();
//                 }
//
//                 #[test]
//                 fn test_write_index_advance_wrapping() {
//                     super::write_index_advance_wrapping::<$ty>();
//                 }
//
//                 #[test]
//                 fn test_cant_read_past_write_head() {
//                     super::cant_read_past_write_head::<$ty>();
//                 }
//
//                 #[test]
//                 fn test_read_index_advance_no_wrap() {
//                     super::read_index_advance_no_wrap::<$ty>();
//                 }
//
//                 #[test]
//                 fn test_read_index_advance_wrapping() {
//                     super::read_index_advance_wrapping::<$ty>();
//                 }
//             }
//         };
//     }
//
//     test_channel_impl!(mpmc_channel, MpmcChannel<u32>);
//
//     trait TestableChannelExt {
//         type P: Producer<Item = u32>;
//         type C: Consumer<Item = u32>;
//
//         fn with_capacity(capacity: usize) -> Channel<u32, Self::P, Self::C> {
//             Channel::with_capacity(capacity)
//         }
//     }
//
//     impl TestableChannelExt for MpmcChannel<u32> {
//         type P = MultiProducer<u32>;
//         type C = MultiConsumer<u32>;
//     }
//
//     fn init_valid<C: TestableChannelExt>() {
//         let capacity = 2;
//         let rb = C::with_capacity(capacity).buffer;
//
//         assert_eq!(0, rb.head.load(Ordering::Relaxed));
//         assert_eq!(0, rb.tail.load(Ordering::Relaxed));
//         assert_eq!(capacity, rb.buffer.len());
//     }
//
//     // we dont need to test < 0, since the size is a const of type usize which gets compile time checked already
//     fn init_buf_eq_zero<C: TestableChannelExt>() {
//         std::hint::black_box(C::with_capacity(0));
//     }
//
//     fn init_buf_eq_one<C: TestableChannelExt>() {
//         std::hint::black_box(C::with_capacity(1));
//     }
//
//     fn cant_write_past_read_tail<C: TestableChannelExt>() {
//         let (p, c) = C::with_capacity(2).split();
//
//         assert!(p.try_write(1));
//         assert!(!p.try_write(2));
//
//         c.try_read();
//         assert!(p.try_write(3));
//         assert!(!p.try_write(4));
//     }
//
//     fn write_index_advance_no_wrap<C: TestableChannelExt>() {
//         let channel = C::with_capacity(2);
//         let rb = channel.buffer;
//         let p = channel.producer;
//         assert!(p.try_write(1));
//
//         assert_eq!(1, rb.head.load(Ordering::Relaxed));
//         assert_eq!(0, rb.tail.load(Ordering::Relaxed));
//     }
//
//     fn write_index_advance_wrapping<C: TestableChannelExt>() {
//         let channel = C::with_capacity(2);
//         let rb = channel.buffer;
//         let p = channel.producer;
//         let c = channel.consumer;
//
//         p.try_write(1);
//         p.try_write(2);
//
//         c.try_read();
//         assert!(p.try_write(1));
//
//         assert_eq!(0, rb.head.load(Ordering::Relaxed));
//         assert_eq!(1, rb.tail.load(Ordering::Relaxed));
//     }
//
//     fn cant_read_past_write_head<C: TestableChannelExt>() {
//         let channel = C::with_capacity(2);
//         let p = channel.producer;
//         let c = channel.consumer;
//         assert_eq!(None, c.try_read());
//
//         p.try_write(1);
//         assert_eq!(Some(1), c.try_read());
//         assert_eq!(None, c.try_read());
//     }
//
//     fn read_index_advance_no_wrap<C: TestableChannelExt>() {
//         let channel = C::with_capacity(2);
//         let rb = channel.buffer;
//         let p = channel.producer;
//         let c = channel.consumer;
//
//         p.try_write(1);
//         assert_eq!(Some(1), c.try_read());
//
//         assert_eq!(1, rb.head.load(Ordering::Relaxed));
//         assert_eq!(1, rb.tail.load(Ordering::Relaxed));
//     }
//
//     fn read_index_advance_wrapping<C: TestableChannelExt>() {
//         let channel = C::with_capacity(2);
//         let rb = channel.buffer;
//         let p = channel.producer;
//         let c = channel.consumer;
//
//         p.try_write(1);
//         c.try_read();
//         p.try_write(2);
//         c.try_read();
//         p.try_write(3);
//         assert_eq!(Some(3), c.try_read());
//
//         assert_eq!(1, rb.head.load(Ordering::Relaxed));
//         assert_eq!(1, rb.tail.load(Ordering::Relaxed));
//     }
// }
