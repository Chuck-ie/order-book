use std::{
    cell::Cell,
    sync::{Arc, atomic::Ordering},
};

use crate::{channel::spinlock::Spinlock, spsc::Buffer};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    QueueEmpty,
    Timeout,
}

pub struct Consumer<T, const N: usize> {
    inner_tail: Cell<usize>,
    inner_cl_tail: Cell<usize>,
    buffer: Arc<Buffer<T, N>>,
    cl_write_count: Cell<usize>,
}

/* [CL0(0, 1), CL1(2, 3), CL2(4, 5), CL3(6, 7)],
 * consumer init state: inner_tail = 3, inner_cl_tail = N(2)
 * producer init state: inner_head = 0, inner_cl_tail = 0
 *
 * producer writes until N, then at the N + 1 messages, it does the following:
 * CL(0).write_count = N, write to CL(1)[0], update inner_head = 1, inner_cl_tail = 1
 *
 *
 */

impl<T, const N: usize> Consumer<T, N> {
    pub(crate) fn new(buffer: &Arc<Buffer<T, N>>) -> Self {
        Self {
            inner_tail: Cell::new(buffer.cache_line_mask),
            inner_cl_tail: Cell::new(N),
            buffer: buffer.clone(),
            cl_write_count: Cell::new(N),
        }
    }

    pub fn recv(&self) -> T {
        let spinlock = Spinlock::new();

        loop {
            match self.try_recv() {
                Ok(value) => return value,
                Err(_) => spinlock.spin_heavy(),
            };
        }
    }

    pub fn try_recv(&self) -> Result<T, Error> {
        let curr_tail = self.inner_tail.get();
        let curr_cl_tail = self.inner_cl_tail.get();
        let curr_cl_write_count = self.cl_write_count.get();

        // If we finished reading from a cache line in the previous recv
        if curr_cl_tail == curr_cl_write_count {
            // Calculate the index of the next cache line by wrapping around buffer bounds using
            // fast modulo since cache_lines is always a power of 2
            let next_tail = (curr_tail + 1) & self.buffer.cache_line_mask;

            // Sync with the writer's release when advancing its head
            let curr_head = self.buffer.head.load(Ordering::Acquire);

            if next_tail == curr_head {
                return Err(Error::QueueEmpty);
            }

            // Safety: next_tail is verified against curr_head and is within bounds
            // let curr_cache_line = unsafe { self.buffer.inner.get_unchecked(curr_tail) };
            // curr_cache_line.write_count.store(0, Ordering::Release);
            let curr_write_count = unsafe { self.buffer.write_counts.get_unchecked(curr_tail) };
            curr_write_count.store(0, Ordering::Release);

            // Safety: next_tail is verified against curr_head and is within bounds
            let next_cache_line = unsafe { self.buffer.inner.get_unchecked(next_tail) };
            let value = unsafe { next_cache_line.read(0) };

            // let next_cl_write_count = next_cache_line.write_count.load(Ordering::Acquire);
            // curr_write_count.store(0, Ordering::Release);
            let next_write_count = unsafe { self.buffer.write_counts.get_unchecked(next_tail) };
            self.cl_write_count
                .set(next_write_count.load(Ordering::Acquire));

            self.inner_tail.set(next_tail);
            self.inner_cl_tail.set(1);

            // Sync the advancement with the write thread
            self.buffer.tail.store(next_tail, Ordering::Release);

            Ok(value)
        } else {
            // Safety: curr_tail is always within bounds and guaranteed not to
            // reach the write head because we checked for empty state previously.
            let cache_line = unsafe { self.buffer.inner.get_unchecked(curr_tail) };
            let value = unsafe { cache_line.read(curr_cl_tail) };
            self.inner_cl_tail.set(curr_cl_tail + 1);

            Ok(value)
        }
    }
}
