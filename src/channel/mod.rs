use std::sync::Arc;

use crate::channel::{
    buffer_handle::{FromBuffer, MultiConsumer, MultiProducer, SingleConsumer, SingleProducer},
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
pub type SpscChannel<T> = Channel<T, SingleProducer<T>, SingleConsumer<T>>;

pub struct Channel<T, P, C>
where
    P: Producer<Item = T> + FromBuffer<T>,
    C: Consumer<Item = T> + FromBuffer<T>,
{
    buffer: Arc<RingBuffer<T>>,
    producer: P,
    consumer: C,
}

impl<T, P, C> Channel<T, P, C>
where
    P: Producer<Item = T> + FromBuffer<T>,
    C: Consumer<Item = T> + FromBuffer<T>,
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
mod channel_tests {
    use crate::channel::{
        Channel, Consumer, FromBuffer, MpmcChannel, MpscChannel, MultiConsumer, MultiProducer,
        Producer, SpscChannel,
        buffer_handle::{SingleConsumer, SingleProducer},
        producer::ProducerError,
    };
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Instant,
    };

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
    test_channel_impl!(spsc_channel, SpscChannel<u32>);

    trait TestableChannelExt {
        type P: Producer<Item = u32> + FromBuffer<u32>;
        type C: Consumer<Item = u32> + FromBuffer<u32>;

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
        type C = SingleConsumer<u32>;
    }

    impl TestableChannelExt for SpscChannel<u32> {
        type P = SingleProducer<u32>;
        type C = SingleConsumer<u32>;
    }

    fn init_valid<C: TestableChannelExt>() {
        let capacity = 2;
        let rb = C::with_capacity(capacity).buffer;

        assert_eq!(0, rb.head.load(Ordering::Relaxed));
        assert_eq!(0, rb.tail.load(Ordering::Relaxed));
        assert_eq!(capacity, rb.buffer.len());
    }

    fn init_buf_eq_zero<C: TestableChannelExt>() {
        std::hint::black_box(C::with_capacity(0));
    }

    fn init_buf_eq_one<C: TestableChannelExt>() {
        std::hint::black_box(C::with_capacity(1));
    }

    fn cant_write_past_read_tail<C: TestableChannelExt>() {
        let (p, c) = C::with_capacity(2).split();

        assert_eq!(Ok(()), p.try_write(1));
        assert_eq!(Err(ProducerError::QueueFull), p.try_write(1));

        assert_eq!(Some(1), c.try_read());
        assert_eq!(Ok(()), p.try_write(3));
        assert_eq!(Err(ProducerError::QueueFull), p.try_write(4));
    }

    fn write_index_advance_no_wrap<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let rb = channel.buffer;
        let p = channel.producer;
        assert_eq!(Ok(()), p.try_write(1));

        assert_eq!(1, rb.head.load(Ordering::Relaxed));
        assert_eq!(0, rb.tail.load(Ordering::Relaxed));
    }

    /// TODO: running this test with miri causes timeouts for writes, which is more a problem
    /// with miri rather than the tests or the implementations. should probably still add a
    /// helper method to write blocking just for testing
    fn write_index_advance_wrapping<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let rb = channel.buffer;
        let p = channel.producer;
        let c = channel.consumer;

        assert_eq!(Ok(()), p.try_write(1));
        assert_eq!(Err(ProducerError::QueueFull), p.try_write(2));

        assert_eq!(Some(1), c.try_read());
        assert_eq!(Ok(()), p.try_write(3));

        assert_eq!(0, rb.head.load(Ordering::Relaxed));
        assert_eq!(1, rb.tail.load(Ordering::Relaxed));
    }

    fn cant_read_past_write_head<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let p = channel.producer;
        let c = channel.consumer;
        assert_eq!(None, c.try_read());

        assert!(p.try_write(1).is_ok());
        assert_eq!(Some(1), c.try_read());
        assert_eq!(None, c.try_read());
    }

    fn read_index_advance_no_wrap<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let rb = channel.buffer;
        let p = channel.producer;
        let c = channel.consumer;

        assert!(p.try_write(1).is_ok());
        assert_eq!(Some(1), c.try_read());

        assert_eq!(1, rb.head.load(Ordering::Relaxed));
        assert_eq!(1, rb.tail.load(Ordering::Relaxed));
    }

    /// TODO: running this test with miri causes timeouts for writes, which is more a problem
    /// with miri rather than the tests or the implementations. should probably still add a
    /// helper method to write blocking just for testing
    fn read_index_advance_wrapping<C: TestableChannelExt>() {
        let channel = C::with_capacity(2);
        let rb = channel.buffer;
        let p = channel.producer;
        let c = channel.consumer;

        assert_eq!(Ok(()), p.try_write(1));
        assert_eq!(Some(1), c.try_read());
        assert_eq!(Ok(()), p.try_write(2));
        assert_eq!(Some(2), c.try_read());
        assert_eq!(Ok(()), p.try_write(3));
        assert_eq!(Some(3), c.try_read());

        assert_eq!(1, rb.head.load(Ordering::Acquire));
        assert_eq!(1, rb.tail.load(Ordering::Acquire));
    }

    // #[test]
    fn test_bench_spsc() {
        let items_to_write = 50_000_000;
        let (producer, consumer) = SpscChannel::<u32>::with_capacity(1024).split();

        let ready = Arc::new(AtomicBool::new(false));
        let ready_p = ready.clone();
        let ready_c = ready.clone();

        let producer_handle = std::thread::spawn(move || {
            while !ready_p.load(Ordering::Acquire) {
                std::thread::yield_now();
            }

            for i in 0..items_to_write {
                while producer.try_write(i).is_err() {}
            }
        });

        let consumer_handle = std::thread::spawn(move || {
            while !ready_c.load(Ordering::Acquire) {
                std::thread::yield_now();
            }

            for _ in 0..items_to_write {
                let mut val = None;
                while val.is_none() {
                    val = consumer.try_read();
                }
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(100));

        let start = Instant::now();
        ready.store(true, Ordering::Release);

        producer_handle.join().unwrap();
        consumer_handle.join().unwrap();

        let elapsed = start.elapsed();
        println!(
            "Total time: {:?}, Throughput: {} ops/sec",
            elapsed,
            (items_to_write as f64 / elapsed.as_secs_f64()) as u64
        );
    }

    // #[test]
    fn test_bench_mpmc_4p_4c() {
        let num_producers = 4;
        let num_consumers = 4;
        let total_items = 50_000_000;
        let items_per_producer = total_items / num_producers;

        let (producer, consumer) = MpmcChannel::<u32>::with_capacity(4096).split();

        let ready = Arc::new(AtomicBool::new(false));
        let mut producer_handles = vec![];
        for _ in 0..num_producers {
            let p = producer.clone();
            let ready_p = ready.clone();
            producer_handles.push(std::thread::spawn(move || {
                while !ready_p.load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }
                for i in 0..items_per_producer {
                    while p.try_write(i as u32).is_err() {
                        std::hint::spin_loop();
                    }
                }
            }));
        }

        let mut consumer_handles = vec![];
        for _ in 0..num_consumers {
            let c = consumer.clone();
            let ready_c = ready.clone();
            consumer_handles.push(std::thread::spawn(move || {
                while !ready_c.load(Ordering::Acquire) {
                    std::hint::spin_loop();
                }
                let mut count = 0;
                while count < items_per_producer {
                    if c.try_read().is_some() {
                        count += 1;
                    } else {
                        std::hint::spin_loop();
                    }
                }
            }));
        }

        std::thread::sleep(std::time::Duration::from_millis(100));

        let start = Instant::now();
        ready.store(true, Ordering::Release);

        for h in producer_handles {
            h.join().unwrap();
        }
        for h in consumer_handles {
            h.join().unwrap();
        }

        let elapsed = start.elapsed();
        println!(
            "Total time: {:?}, Throughput: {} ops/sec",
            elapsed,
            (total_items as f64 / elapsed.as_secs_f64()) as u64
        );
    }
}
