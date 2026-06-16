use std::{
    cell::{Cell, UnsafeCell},
    ptr,
    sync::{
        Arc,
        atomic::{AtomicPtr, AtomicUsize, Ordering},
    },
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
        // TODO: magic var 64 should be replaced with the size of a cacheline depending on the
        // system since not all systems have a fixed 64 byte sized cacheline
        const SIZE_OF_TY: usize = std::mem::size_of::<$ty>();

        const _: () = assert!(
            SIZE_OF_TY <= 64,
            "Compile Error: Type size cannot be greater than 64 bytes!"
        );

        const _: () = assert!(
            SIZE_OF_TY > 0,
            "Compile Error: Zero-Sized Types are not allowed!"
        );

        const _: () = {
            const CAP: usize = $capacity;
            assert!(CAP.is_power_of_two(), "capacity must be a power of 2");
        };

        $crate::spsc::Buffer::<$ty, { 64 / SIZE_OF_TY }>::with_capacity($capacity)
    }};
}

/*
 * spsc(u32, 1024): 16 u32 fit into 1 cache line, therefore the length of the array is 64
 * [CL(0), ..., CL(63)]: each CL contains up to 16 uninit u32
 * read and write should always be on different CL's with
 * queue is full if head == tail - 1 and CL internal index == N (64 / size_of(u32))
 */
struct Buffer<T, const N: usize> {
    inner: Box<[CacheLine<T, N>]>,
    head: CachePadded<AtomicUsize>,
    tail: CachePadded<AtomicUsize>,
    capacity: usize,
    inner_cl_head_ptr: AtomicPtr<Cell<usize>>,
}

unsafe impl<T: Send, const N: usize> Send for Buffer<T, N> {}
unsafe impl<T: Sync, const N: usize> Sync for Buffer<T, N> {}

impl<T, const N: usize> Buffer<T, N> {
    // TODO: consider implementing into for spsc -> (prod, cons)
    pub fn with_capacity(capacity: usize) -> (Box<Producer<T, N>>, Consumer<T, N>) {
        // pub fn with_capacity(capacity: usize) -> (Producer<T, N>, Consumer<T, N>) {
        let actual_capacity = capacity / N;
        let inner = (0..actual_capacity).map(|_| CacheLine::default()).collect();

        let buffer = Arc::new(Self {
            inner,
            head: CachePadded(AtomicUsize::new(0)),
            tail: CachePadded(AtomicUsize::new(0)),
            capacity: actual_capacity - 1,
            inner_cl_head_ptr: AtomicPtr::new(ptr::null_mut()),
        });

        // box the producer, so that the cl_head_ptr stays stable, even if the producer is moved
        // between threads or even just into a thread
        let producer = Box::new(Producer::new(&buffer));
        let consumer = Consumer::new(&buffer);

        buffer.inner_cl_head_ptr.store(
            ptr::addr_of!(producer.inner_cl_head).cast_mut(),
            Ordering::Release,
        );

        (producer, consumer)
    }

    #[inline]
    pub const fn get_pos(pos: usize, cl_pos: usize) -> usize {
        (pos * N) + cl_pos
    }
}

#[cfg(test)]
mod spsc_tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    use crossbeam_channel::bounded;

    const LEN: usize = 1;

    #[derive(Debug, Clone, Copy)]
    pub struct Message(#[allow(dead_code)] [usize; LEN]);

    #[inline]
    pub fn new(num: usize) -> Message {
        Message([num; LEN])
    }

    fn custom_spsc_bench() {
        let items_to_write = 5_000_001;
        let (producer, consumer) = channel!(Message, 1024);

        let ready = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ready_p = ready.clone();
        let ready_c = ready.clone();

        std::thread::scope(|scope| {
            let producer_handle = scope.spawn(move || {
                while !ready_p.load(std::sync::atomic::Ordering::Acquire) {
                    std::thread::yield_now();
                }

                for i in 0..items_to_write {
                    producer.send(new(i));
                }
            });

            let consumer_handle = scope.spawn(move || {
                while !ready_c.load(std::sync::atomic::Ordering::Acquire) {
                    std::thread::yield_now();
                }

                for _ in 0..items_to_write {
                    std::hint::black_box(consumer.recv());
                }
            });

            // Small delay to allow threads to hit the yield loop
            std::thread::sleep(std::time::Duration::from_millis(100));
            ready.store(true, std::sync::atomic::Ordering::Release);

            producer_handle.join().unwrap();
            consumer_handle.join().unwrap();
        });
    }

    const MESSAGES: usize = 5_000_001;

    fn custom_spsc() {
        let (tx, rx) = channel!(Message, 1024);

        let mut sum: usize = 0;
        std::thread::scope(|scope| {
            scope.spawn(move || {
                for i in 0..MESSAGES {
                    tx.send(new(i));
                }
            });

            for _ in 0..MESSAGES {
                let msg = rx.recv();
                sum = sum.wrapping_add(std::hint::black_box(unsafe { *msg.0.get_unchecked(0) }));
            }
        });
        std::hint::black_box(sum);
        println!("sum custom_spsc: {sum}");
    }

    fn crossbeam_spsc() {
        let (tx, rx) = bounded::<Message>(1024);

        let mut sum: usize = 0;
        std::thread::scope(|scope| {
            scope.spawn(move || {
                for i in 0..MESSAGES {
                    tx.send(new(i)).unwrap();
                }
            });

            for _ in 0..MESSAGES {
                let msg = rx.recv().unwrap();
                sum = sum.wrapping_add(std::hint::black_box(unsafe { *msg.0.get_unchecked(0) }));
            }
        });
        std::hint::black_box(sum);
        println!("sum crossbeam_spsc: {sum}");
    }

    #[test]
    fn test() {
        macro_rules! run {
            ($name:expr, $f:expr) => {
                let now = ::std::time::Instant::now();
                $f;
                let elapsed = now.elapsed();
                println!(
                    "{:25} {:15} {:7.3} sec",
                    $name,
                    "Rust crossbeam-channel",
                    elapsed.as_secs() as f64 + elapsed.subsec_nanos() as f64 / 1e9
                );
            };
        }

        run!("custom_spsc", custom_spsc());
        run!("crossbeam_spsc", crossbeam_spsc());
    }

    // #[test]
    // fn test_bench_spsc() {
    //     let items_to_write = 5_000_000;
    //     let (producer, mut consumer) = channel!(Message, 1024);
    //
    //     let ready = Arc::new(AtomicBool::new(false));
    //     let ready_p = ready.clone();
    //     let ready_c = ready.clone();
    //     let with_delay = false;
    //
    //     let producer_handle = std::thread::spawn(move || {
    //         while !ready_p.load(Ordering::Acquire) {
    //             std::thread::yield_now();
    //         }
    //
    //         for i in 0..items_to_write {
    //             producer.send(new(i));
    //             std::hint::black_box(());
    //             if with_delay {
    //                 let end = Instant::now() + Duration::from_nanos(100);
    //                 while Instant::now() < end {}
    //             }
    //         }
    //     });
    //
    //     let consumer_handle = std::thread::spawn(move || {
    //         while !ready_c.load(Ordering::Acquire) {
    //             std::thread::yield_now();
    //         }
    //
    //         for i in 0..items_to_write {
    //             if i > (items_to_write - 16) {
    //                 for _ in std::hint::black_box(consumer.flush_recv()) {}
    //             } else {
    //                 std::hint::black_box(consumer.recv());
    //             }
    //
    //             // std::hint::black_box(consumer.recv());
    //
    //             if with_delay {
    //                 let end = Instant::now() + Duration::from_nanos(100);
    //                 while Instant::now() < end {}
    //             }
    //         }
    //     });
    //
    //     std::thread::sleep(std::time::Duration::from_millis(100));
    //
    //     let start = Instant::now();
    //     ready.store(true, Ordering::Release);
    //
    //     producer_handle.join().unwrap();
    //     consumer_handle.join().unwrap();
    //
    //     let elapsed = start.elapsed();
    //     println!(
    //         "test_bench_spsc: Total time: {:?}, Throughput: {} ops/sec",
    //         elapsed,
    //         (items_to_write as f64 / elapsed.as_secs_f64()) as u64
    //     );
    // }

    // #[test]
    fn test_bench_crossbeam() {
        let items_to_write = 5_000_00;
        let (producer, consumer) = bounded(1024);

        let ready = Arc::new(AtomicBool::new(false));
        let ready_p = ready.clone();
        let ready_c = ready.clone();
        let with_delay = false;

        let producer_handle = std::thread::spawn(move || {
            while !ready_p.load(Ordering::Acquire) {
                std::thread::yield_now();
            }

            for i in 0..items_to_write {
                producer.send(i).unwrap();
                std::hint::black_box(());

                if with_delay {
                    let end = Instant::now() + Duration::from_nanos(100);
                    while Instant::now() < end {}
                }
            }
        });

        let consumer_handle = std::thread::spawn(move || {
            while !ready_c.load(Ordering::Acquire) {
                std::thread::yield_now();
            }

            for _ in 0..items_to_write {
                consumer.recv().unwrap();
                std::hint::black_box(());

                if with_delay {
                    let end = Instant::now() + Duration::from_nanos(100);
                    while Instant::now() < end {}
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
            "test_bench_crossbeam: Total time: {:?}, Throughput: {} ops/sec",
            elapsed,
            (items_to_write as f64 / elapsed.as_secs_f64()) as u64
        );
    }

    // #[test]
    fn test_bench_std() {
        let items_to_write = 5_000_00;
        let (producer, consumer) = std::sync::mpsc::sync_channel(1024);

        let ready = Arc::new(AtomicBool::new(false));
        let ready_p = ready.clone();
        let ready_c = ready.clone();

        let producer_handle = std::thread::spawn(move || {
            while !ready_p.load(Ordering::Acquire) {
                std::thread::yield_now();
            }

            for i in 0..items_to_write {
                let _ = producer.send(i);
                std::hint::black_box(());
            }
        });

        let consumer_handle = std::thread::spawn(move || {
            while !ready_c.load(Ordering::Acquire) {
                std::thread::yield_now();
            }

            for _ in 0..items_to_write {
                let _ = std::hint::black_box(consumer.recv());
            }
        });

        std::thread::sleep(std::time::Duration::from_millis(100));

        let start = Instant::now();
        ready.store(true, Ordering::Release);

        producer_handle.join().unwrap();
        consumer_handle.join().unwrap();

        let elapsed = start.elapsed();
        println!(
            "test_bench_std: Total time: {:?}, Throughput: {} ops/sec",
            elapsed,
            (items_to_write as f64 / elapsed.as_secs_f64()) as u64
        );
    }
}
