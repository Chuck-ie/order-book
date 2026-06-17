use std::{
    cell::UnsafeCell,
    sync::{Arc, atomic::AtomicUsize},
};

use crate::spsc::{
    consumer::Consumer,
    producer::Producer,
    wrapper::{CacheLine, CachePadded},
};

mod consumer;
mod producer;
mod wrapper;

#[macro_export]
macro_rules! channel {
    ($ty:ty, $capacity:expr) => {{
        // TODO: This 64-byte magic number should ideally be determined based on the target architecture.
        const CACHE_LINE_SIZE: usize = 64;
        const ELEMENT_SIZE: usize = std::mem::size_of::<$ty>();

        // Validate type size constraints at compile time
        const _: () = {
            assert!(
                ELEMENT_SIZE <= CACHE_LINE_SIZE,
                "Compile Error: Type size cannot be greater than the cache line size (64 bytes)!"
            );
            assert!(
                ELEMENT_SIZE > 0,
                "Compile Error: Zero-Sized Types (ZSTs) are not allowed!"
            );
        };

        const ELEMENTS_PER_CACHE_LINE: usize = CACHE_LINE_SIZE / ELEMENT_SIZE;
        const TARGET_CAPACITY: usize = $capacity;

        // Validate capacity constraints at compile time
        const _: () = {
            assert!(
                TARGET_CAPACITY.is_power_of_two(),
                "Compile Error: Capacity must be a power of 2!"
            );
            assert!(
                TARGET_CAPACITY >= 4 * ELEMENTS_PER_CACHE_LINE,
                "Compile Error: Capacity is too small! It must be at least four times the elements per cache line."
            );
        };

        $crate::spsc::Buffer::<$ty, ELEMENTS_PER_CACHE_LINE>::with_capacity(TARGET_CAPACITY)
    }};
}

struct Buffer<T, const N: usize> {
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
    inner: UnsafeCell<Box<[CacheLine<T, N>]>>,
    // write_counts: Box<[AtomicUsize]>,
    write_counts: Box<[UnsafeCell<usize>]>,
    cl_mask: usize,
    capacity: usize,
}

unsafe impl<T: Send, const N: usize> Send for Buffer<T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for Buffer<T, N> {}

impl<T, const N: usize> Buffer<T, N> {
    pub fn with_capacity(capacity: usize) -> (Producer<T, N>, Consumer<T, N>) {
        let cache_lines = capacity / N;
        let inner: Box<[CacheLine<T, N>]> =
            (0..cache_lines).map(|_| CacheLine::default()).collect();

        let write_counts: Box<[UnsafeCell<usize>]> =
            (0..cache_lines).map(|_| UnsafeCell::new(0)).collect();

        let cache_line_mask = cache_lines - 1;

        let buffer = Arc::new(Self {
            head: CachePadded(AtomicUsize::new(0)),
            tail: CachePadded(AtomicUsize::new(cache_line_mask)),
            inner: UnsafeCell::new(inner),
            write_counts,
            cl_mask: cache_line_mask,
            capacity,
        });

        let producer = Producer::new(&buffer);
        let consumer = Consumer::new(&buffer);

        (producer, consumer)
    }

    // # Safety: the caller has to make sure that index is within bounds
    unsafe fn get_cache_line(&self, index: usize) -> &CacheLine<T, N> {
        unsafe { self.inner.get().as_ref_unchecked().get_unchecked(index) }
    }
}

#[cfg(test)]
mod spsc_tests {
    use std::{
        collections::VecDeque,
        sync::atomic::{AtomicUsize, Ordering},
        time::{Duration, Instant},
    };

    use crossbeam_channel::bounded;
    use picoring::create_spsc;
    use ringbuffer_spsc::ringbuffer;

    // #[test]
    fn test_single_threaded_vec() {
        const MESSAGES: usize = 5_000_000;

        let mut queue = VecDeque::with_capacity(MESSAGES);
        let mut sum: usize = 0;
        let start = std::time::Instant::now();

        for i in 0..MESSAGES {
            queue.push_back(i);
        }

        while let Some(val) = queue.pop_front() {
            sum += val;
        }

        let elapsed = start.elapsed();

        std::hint::black_box(sum);
        println!("sum single_threaded_vec: {sum}");
        println!("Time taken: {elapsed:?}");
        println!(
            "Throughput: {:.2} msg/s",
            MESSAGES as f64 / elapsed.as_secs_f64()
        );

        let expected_sum = (MESSAGES - 1) * (MESSAGES) / 2;
        assert_eq!(sum, expected_sum, "Sum does not match the expected value");
    }

    // #[test]
    fn test_custom_spsc_batch() {
        const MESSAGES: usize = 5_000_000;
        const BATCH_SIZE: usize = 512;

        let (tx, rx) = channel!(usize, 1024 * 16);
        let mut sum: usize = 0;
        let start = std::time::Instant::now();

        std::thread::scope(|scope| {
            scope.spawn(move || {
                let mut i = 0;
                while i < MESSAGES {
                    let current_batch_size = std::cmp::min(BATCH_SIZE, MESSAGES - i);

                    let mut batch = [0; BATCH_SIZE];
                    for j in 0..current_batch_size {
                        batch[j] = i;
                        i += 1;
                    }

                    let batch_slice = &batch[..current_batch_size];

                    while tx.try_send_batch(batch_slice).is_err() {}
                }

                while tx.flush().is_err() {}
            });

            // Receiver Thread (Receives individual elements exactly like before)
            for _ in 0..MESSAGES {
                sum += rx.recv();
            }
        });

        let elapsed = start.elapsed();

        std::hint::black_box(sum);
        println!("sum custom_spsc_batch: {sum}");
        println!("Time taken: {elapsed:?}");
        println!(
            "Throughput: {:.2} msg/s",
            MESSAGES as f64 / elapsed.as_secs_f64()
        );

        let expected_sum = (MESSAGES - 1) * (MESSAGES) / 2;
        assert_eq!(sum, expected_sum, "Sum does not match the expected value");
    }

    // #[test]
    fn test_custom_spsc_reservation_batch() {
        const MESSAGES: usize = 5_000_000;
        const BATCH_SIZE: usize = 8192;

        let (tx, rx) = channel!(usize, 1024 * 16);
        let mut sum: usize = 0;
        let start = std::time::Instant::now();

        std::thread::scope(|scope| {
            scope.spawn(move || {
                let mut i = 0;
                while i < MESSAGES {
                    let current_batch_size = std::cmp::min(BATCH_SIZE, MESSAGES - i);

                    if let Ok(mut reservation) = tx.try_reserve_batch(current_batch_size) {
                        for _ in 0..current_batch_size {
                            let _ = reservation.send(i);
                            i += 1;
                        }
                    } else {
                        std::hint::spin_loop();
                    }
                }

                while tx.flush().is_err() {}
            });

            for _ in 0..MESSAGES {
                sum += rx.recv();
            }
        });

        let elapsed = start.elapsed();

        std::hint::black_box(sum);
        println!("sum custom_spsc_reservation_batch: {sum}");
        println!("Time taken: {elapsed:?}");
        println!(
            "Throughput: {:.2} msg/s",
            MESSAGES as f64 / elapsed.as_secs_f64()
        );

        let expected_sum = (MESSAGES - 1) * (MESSAGES) / 2;
        assert_eq!(sum, expected_sum, "Sum does not match the expected value");
    }

    // #[test]
    fn test_picoring_spsc_batch() {
        const MESSAGES: usize = 5_000_000;
        const BATCH_SIZE: usize = 512;

        // Allocate the mirror-backed SPSC channel
        let (mut tx, mut rx) = create_spsc::<usize>(1024 * 16).unwrap();
        let mut sum: usize = 0;
        let start = std::time::Instant::now();

        std::thread::scope(|scope| {
            // Sender Thread
            scope.spawn(move || {
                let mut i = 0;
                while i < MESSAGES {
                    let current_batch_size = std::cmp::min(BATCH_SIZE, MESSAGES - i);

                    // Get the raw writable slice directly from the buffer
                    let slice = tx.writable_slice();

                    if slice.len() >= current_batch_size {
                        for j in 0..current_batch_size {
                            slice[j] = i;
                            i += 1;
                        }
                        // Move the producer head forward
                        tx.advance_head(current_batch_size);
                    } else {
                        // Buffer is full, spin/yield until space clears up
                        std::hint::spin_loop();
                    }
                }
            });

            // Receiver Thread
            let mut messages_received = 0;
            while messages_received < MESSAGES {
                // Get the readable slice directly
                let slice = rx.readable_slice();

                if slice.is_empty() {
                    std::hint::spin_loop();
                } else {
                    let len = std::cmp::min(slice.len(), MESSAGES - messages_received);

                    for j in slice.iter().take(len) {
                        sum += *j;
                    }

                    messages_received += len;
                    rx.advance_tail(len);
                }
            }
        });

        let elapsed = start.elapsed();

        std::hint::black_box(sum);
        println!("sum picoring_spsc: {sum}");
        println!("Time taken: {elapsed:?}");
        println!(
            "Throughput: {:.2} msg/s",
            MESSAGES as f64 / elapsed.as_secs_f64()
        );

        let expected_sum = (MESSAGES - 1) * (MESSAGES) / 2;
        assert_eq!(sum, expected_sum, "Sum does not match the expected value");
    }

    #[test]
    fn test_custom_spsc() {
        const MESSAGES: usize = 5_000_000;
        let (tx, rx) = channel!(usize, 1024);
        let mut sum: usize = 0;
        let start = std::time::Instant::now();

        std::thread::scope(|scope| {
            scope.spawn(move || {
                for i in 0..MESSAGES {
                    tx.send(i);

                    // while tx.flush().is_err() {}
                }
                while tx.flush().is_err() {}
            });

            for _ in 0..MESSAGES {
                sum += rx.recv();
            }
        });

        let elapsed = start.elapsed();

        std::hint::black_box(sum);
        println!("sum custom_spsc: {sum}");
        println!("Time taken: {elapsed:?}");
        println!(
            "Throughput: {:.2} msg/s",
            MESSAGES as f64 / elapsed.as_secs_f64()
        );

        let expected_sum = (MESSAGES - 1) * (MESSAGES) / 2;
        assert_eq!(sum, expected_sum, "Sum does not match the expected value");
    }

    // #[test]
    fn bench_spsc_impls() {
        // TODO: benches are not 100% fair right now i guess since i use a spinglook and other spsc
        // impls just get a yield_now forced onto them
        // custom_spsc_benchmark();
        // ringbuffer_spsc_benchmark();
        // crossbeam_bounded_benchmark();
        // std_mpsc_sync_benchmark();
    }

    fn custom_spsc_benchmark() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        static STEP: Duration = Duration::from_secs(1);
        let (tx, rx) = channel!(usize, 1024);

        std::thread::spawn(move || {
            let mut i: usize = 0;
            loop {
                tx.send(1);
                i = i.wrapping_add(1);
            }
        });

        std::thread::spawn(move || {
            loop {
                rx.recv();
                COUNTER.fetch_add(1, Ordering::Relaxed);
            }
        });

        let start = Instant::now();
        for i in 1..=u32::MAX {
            let target_time = start + i * STEP;
            let now = Instant::now();
            if target_time > now {
                std::thread::sleep(target_time - now);
            }

            println!("custom_spsc: {} elem/s", COUNTER.swap(0, Ordering::Relaxed));
        }
    }

    fn ringbuffer_spsc_benchmark() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        static STEP: Duration = Duration::from_secs(1);
        let (mut tx, mut rx) = ringbuffer::<usize>(2_usize.pow(10));

        std::thread::spawn(move || {
            loop {
                if tx.push(1).is_some() {
                    std::thread::yield_now();
                }
            }
        });

        std::thread::spawn(move || {
            loop {
                if rx.pull().is_some() {
                    COUNTER.fetch_add(1, Ordering::Relaxed);
                } else {
                    std::thread::yield_now();
                }
            }
        });

        let start = Instant::now();
        for i in 1..=u32::MAX {
            std::thread::sleep(start + i * STEP - Instant::now());
            println!("{} elem/s", COUNTER.swap(0, Ordering::Relaxed));
        }
    }

    fn crossbeam_bounded_benchmark() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        static STEP: Duration = Duration::from_secs(1);
        let (producer, consumer) = bounded::<usize>(1024);

        std::thread::spawn(move || {
            let mut i = 0;
            loop {
                if producer.send(i).is_err() {
                    break;
                }
                i = i.wrapping_add(1);
            }
        });

        std::thread::spawn(move || {
            loop {
                if consumer.recv().is_ok() {
                    COUNTER.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        let start = Instant::now();
        for i in 1..=u32::MAX {
            let target_time = start + i * STEP;
            let now = Instant::now();
            if target_time > now {
                std::thread::sleep(target_time - now);
            }

            println!(
                "crossbeam_bounded: {} elem/s",
                COUNTER.swap(0, Ordering::Relaxed)
            );
        }
    }

    fn std_mpsc_sync_benchmark() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        static STEP: Duration = Duration::from_secs(1);
        let (producer, consumer) = std::sync::mpsc::sync_channel::<usize>(1024);

        std::thread::spawn(move || {
            let mut i = 0;
            loop {
                if producer.send(i).is_err() {
                    break;
                }
                i = i.wrapping_add(1);
            }
        });

        std::thread::spawn(move || {
            loop {
                if consumer.recv().is_ok() {
                    COUNTER.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        let start = Instant::now();
        for i in 1..=u32::MAX {
            let target_time = start + i * STEP;
            let now = Instant::now();
            if target_time > now {
                std::thread::sleep(target_time - now);
            }

            println!(
                "std_mpsc_sync: {} elem/s",
                COUNTER.swap(0, Ordering::Relaxed)
            );
        }
    }
}
