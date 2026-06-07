use std::sync::Arc;

use crate::channel::{
    buffer_handle::{FromBuffer, MultiConsumer, MultiProducer, SingleConsumer},
    consumer::Consumer,
    producer::Producer,
    ring_buffer::RingBuffer,
};

mod buffer_handle;
mod consumer;
mod producer;
mod ring_buffer;
mod spinlock;

pub type MpmcChannel<T> = Channel<T, MultiProducer<T>, MultiConsumer<T>>;
pub type MpscChannel<T> = Channel<T, MultiProducer<T>, SingleConsumer<T>>;

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

#[cfg(test)]
mod tests {
    use crate::channel::{
        Channel, Consumer, FromBuffer, MpmcChannel, MpscChannel, MultiConsumer, MultiProducer,
        Producer,
    };
    use std::sync::atomic::Ordering;

    macro_rules! test_channel_impl {
        ($name:ident, $ty:ty) => {
            mod $name {
                use super::*;

                #[test]
                fn test_init_valid() {
                    super::init_valid::<$ty>();
                }

                #[test]
                #[should_panic(expected = "capacity must be greater than 1")]
                fn test_init_buf_eq_zero() {
                    super::init_buf_eq_zero::<$ty>();
                }

                #[test]
                #[should_panic(expected = "capacity must be greater than 1")]
                fn test_init_buf_eq_one() {
                    super::init_buf_eq_one::<$ty>();
                }

                #[test]
                fn test_cant_write_past_read_tail() {
                    super::cant_write_past_read_tail::<$ty>();
                }

                #[test]
                fn test_write_index_advance_no_wrap() {
                    super::write_index_advance_no_wrap::<$ty>();
                }

                #[test]
                fn test_write_index_advance_wrapping() {
                    super::write_index_advance_wrapping::<$ty>();
                }

                #[test]
                fn test_cant_read_past_write_head() {
                    super::cant_read_past_write_head::<$ty>();
                }

                #[test]
                fn test_read_index_advance_no_wrap() {
                    super::read_index_advance_no_wrap::<$ty>();
                }

                #[test]
                fn test_read_index_advance_wrapping() {
                    super::read_index_advance_wrapping::<$ty>();
                }
            }
        };
    }

    test_channel_impl!(mpmc_channel, MpmcChannel<u32>);
    test_channel_impl!(mpsc_channel, MpscChannel<u32>);

    trait TestableChannelExt {
        type P: Producer<Item = u32> + FromBuffer<Item = u32>;
        type C: Consumer<Item = u32> + FromBuffer<Item = u32>;

        fn with_capacity(capacity: usize) -> Channel<u32, Self::P, Self::C> {
            Channel::with_capacity(capacity)
        }
    }

    impl TestableChannelExt for MpmcChannel<u32> {
        type P = MultiProducer<u32>;
        type C = MultiConsumer<u32>;
    }

    impl TestableChannelExt for MpscChannel<u32> {
        type P = MultiProducer<u32>;
        type C = MultiConsumer<u32>;
    }

    fn init_valid<C: TestableChannelExt>() {
        let capacity = 2;
        let rb = C::with_capacity(capacity).buffer;

        assert_eq!(0, rb.head.load(Ordering::Relaxed));
        assert_eq!(0, rb.tail.load(Ordering::Relaxed));
        assert_eq!(capacity, rb.buffer.len());
    }

    // we dont need to test < 0, since the size is a const of type usize which gets compile time checked already
    fn init_buf_eq_zero<C: TestableChannelExt>() {
        std::hint::black_box(C::with_capacity(0));
    }

    fn init_buf_eq_one<C: TestableChannelExt>() {
        std::hint::black_box(C::with_capacity(1));
    }

    fn cant_write_past_read_tail<C: TestableChannelExt>() {
        let (p, c) = C::with_capacity(2).split();

        assert!(p.try_write(1));
        assert!(!p.try_write(2));

        c.try_read();
        assert!(p.try_write(3));
        assert!(!p.try_write(4));
    }

    fn write_index_advance_no_wrap<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let rb = channel.buffer;
        let p = channel.producer;
        assert!(p.try_write(1));

        assert_eq!(1, rb.head.load(Ordering::Relaxed));
        assert_eq!(0, rb.tail.load(Ordering::Relaxed));
    }

    fn write_index_advance_wrapping<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let rb = channel.buffer;
        let p = channel.producer;
        let c = channel.consumer;

        p.try_write(1);
        p.try_write(2);

        c.try_read();
        assert!(p.try_write(1));

        assert_eq!(0, rb.head.load(Ordering::Relaxed));
        assert_eq!(1, rb.tail.load(Ordering::Relaxed));
    }

    fn cant_read_past_write_head<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let p = channel.producer;
        let c = channel.consumer;
        assert_eq!(None, c.try_read());

        p.try_write(1);
        assert_eq!(Some(1), c.try_read());
        assert_eq!(None, c.try_read());
    }

    fn read_index_advance_no_wrap<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let rb = channel.buffer;
        let p = channel.producer;
        let c = channel.consumer;

        p.try_write(1);
        assert_eq!(Some(1), c.try_read());

        assert_eq!(1, rb.head.load(Ordering::Relaxed));
        assert_eq!(1, rb.tail.load(Ordering::Relaxed));
    }

    fn read_index_advance_wrapping<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let rb = channel.buffer;
        let p = channel.producer;
        let c = channel.consumer;

        p.try_write(1);
        c.try_read();
        p.try_write(2);
        c.try_read();
        p.try_write(3);
        assert_eq!(Some(3), c.try_read());

        assert_eq!(1, rb.head.load(Ordering::Relaxed));
        assert_eq!(1, rb.tail.load(Ordering::Relaxed));
    }
}
