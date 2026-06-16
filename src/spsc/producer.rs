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
    pub(crate) inner_head: Cell<usize>,
    pub(crate) inner_cl_head: Cell<usize>,
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

    // pub fn try_send(&self, value: T) -> Result<(), (T, Error)> {
    //     // since there is only one writer, we don't need to sync reading the outer write head with other threads
    //     let mut curr_head = self.inner_head.get();
    //     let mut curr_cl_head = self.inner_cl_head.get();
    //
    //     // if we finished writing to the current cache line
    //     if curr_cl_head == N {
    //         // calculate the index of the next cache line by wrapping around buffer bounds
    //         let next_head = (curr_head + 1) & self.buffer.capacity;
    //
    //         // sync with the reader's release when advancing its tail
    //         let curr_tail = self.buffer.tail.load(Ordering::Acquire);
    //
    //         // and check if the next cache line is owned by the read head
    //         if next_head == curr_tail {
    //             return Err((value, Error::QueueFull));
    //         }
    //
    //         curr_cl_head = 0;
    //         curr_head = next_head;
    //
    //         // sync it with the read thread
    //         self.buffer.head.store(next_head, Ordering::Release);
    //         self.inner_head.set(next_head);
    //     }
    //
    //     // Safety: curr_head is always within bounds and never overlaps with the read head
    //     let cache_line = unsafe { self.buffer.inner.get_unchecked(curr_head) };
    //     unsafe { cache_line.write(curr_cl_head, value) };
    //
    //     let next_cl_head = curr_cl_head + 1;
    //     self.inner_cl_head.set(next_cl_head);
    //
    //     Ok(())
    // }

    pub fn try_send(&self, value: T) -> Result<(), (T, Error)> {
        let curr_head = self.inner_head.get();
        let curr_cl_head = self.inner_cl_head.get();

        // if we finished writing to a cache line in the previous send
        if curr_cl_head == N {
            // Calculate the index of the next cache line by wrapping around buffer bounds using
            // fast modulo since cache_lines is always a power of 2
            let next_head = (curr_head + 1) & self.buffer.cache_lines;

            // Sync with the reader's release when advancing its tail
            let curr_tail = self.buffer.tail.load(Ordering::Acquire);

            if next_head == curr_tail {
                return Err((value, Error::QueueFull));
            }

            // Safety: next_head is verified against curr_tail and is within bounds
            let cache_line = unsafe { self.buffer.inner.get_unchecked(next_head) };
            unsafe { cache_line.write(0, value) };

            self.inner_head.set(next_head);
            self.inner_cl_head.set(1);

            // Sync the advancement with the read thread
            self.buffer.head.store(next_head, Ordering::Release);
        } else {
            // Safety: curr_head is always within bounds and never overlaps with the read head
            let cache_line = unsafe { self.buffer.inner.get_unchecked(curr_head) };
            unsafe { cache_line.write(curr_cl_head, value) };

            self.inner_cl_head.set(curr_cl_head + 1);
        }

        Ok(())
    }
}
