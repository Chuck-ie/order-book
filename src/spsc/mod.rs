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
    use std::collections::VecDeque;

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

    #[test]
    fn test_custom_spsc_reservation_batch() {
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

                    if let Ok(mut reservation) = tx.try_reserve_exact(current_batch_size) {
                        while reservation.send(i).is_some() {
                            i += 1;
                        }
                    } else {
                        std::hint::spin_loop();
                    }
                }

                while tx.flush().is_err() {}
            });

            for i in 0..MESSAGES {
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
}
