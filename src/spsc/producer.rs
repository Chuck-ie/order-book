use std::{
    cell::Cell,
    sync::{Arc, atomic::Ordering},
};

use crate::{channel::spinlock::Spinlock, spsc::Buffer};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    QueueFull,
    Timeout,
}

pub struct Producer<T, const N: usize> {
    inner_head: Cell<usize>,
    inner_cl_head: Cell<usize>,
    buffer: Arc<Buffer<T, N>>,
}

impl<T, const N: usize> Producer<T, N> {
    pub(crate) fn new(buffer: &Arc<Buffer<T, N>>) -> Self {
        Self {
            inner_head: Cell::new(0),
            inner_cl_head: Cell::new(0),
            buffer: buffer.clone(),
        }
    }

    pub fn send(&self, mut value: T) {
        let spinlock = Spinlock::new();

        loop {
            match self.try_send(value) {
                Ok(()) => break,
                Err((returned_value, _)) => {
                    value = returned_value;
                    spinlock.spin_heavy()
                }
            };
        }
    }

    pub fn try_send(&self, value: T) -> Result<(), (T, Error)> {
        let curr_head = self.inner_head.get();
        let curr_cl_head = self.inner_cl_head.get();

        // slow path when trying to wrap around at a cache line border
        // if we finished writing to a cache line in the previous send
        if curr_cl_head == N {
            // Calculate the index of the next cache line by wrapping around buffer bounds using
            // fast modulo since cache_lines is always a power of 2
            let next_head = (curr_head + 1) & self.buffer.cache_line_mask;

            // Sync with the reader's release when advancing its tail
            let curr_tail = self.buffer.tail.load(Ordering::Acquire);

            if next_head == curr_tail {
                return Err((value, Error::QueueFull));
            }

            // Safety: curr_head is exclusively owned by the writer and is within bounds
            let curr_cache_line = unsafe { self.buffer.inner.get_unchecked(curr_head) };
            curr_cache_line.write_count.store(N, Ordering::Release);

            // Safety: next_head is verified to not overlap with curr_tail and is within bounds
            let next_cache_line = unsafe { self.buffer.inner.get_unchecked(next_head) };
            unsafe { next_cache_line.write(0, value) };

            self.inner_head.set(next_head);
            self.inner_cl_head.set(1);

            // Sync the advancement with the read thread
            self.buffer.head.store(next_head, Ordering::Release);
        }
        // fast path for the currently exclusively owned cache line
        else {
            // Safety: curr_head is always within bounds and never overlaps with the read head
            let cache_line = unsafe { self.buffer.inner.get_unchecked(curr_head) };
            unsafe { cache_line.write(curr_cl_head, value) };

            self.inner_cl_head.set(curr_cl_head + 1);
        }

        Ok(())
    }

    pub fn flush(&self) -> Result<(), Error> {
        let curr_head = self.inner_head.get();
        let curr_cl_head = self.inner_cl_head.get();
        let next_head = (curr_head + 1) & self.buffer.cache_line_mask;

        // Sync with the reader's release when advancing its tail
        let curr_tail = self.buffer.tail.load(Ordering::Acquire);

        if next_head == curr_tail {
            return Err(Error::QueueFull);
        }

        self.inner_head.set(next_head);
        self.inner_cl_head.set(0);

        // case(curr_cl_head == 0): means 0 has not yet been written
        // case(curr_cl_head == 1): means 1 has not yet been written
        // case(curr_cl_head == N): means N has not yet been written
        // Safety: curr_head is exclusively owned by the writer and is within bounds
        let curr_cache_line = unsafe { self.buffer.inner.get_unchecked(curr_head) };

        curr_cache_line
            .write_count
            .store(curr_cl_head, Ordering::Release);

        self.buffer.head.store(next_head, Ordering::Release);

        Ok(())
    }
}
