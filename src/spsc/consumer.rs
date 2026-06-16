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
}

impl<T, const N: usize> Consumer<T, N> {
    pub(crate) fn new(buffer: &Arc<Buffer<T, N>>) -> Self {
        Self {
            inner_tail: Cell::new(buffer.cache_lines),
            inner_cl_tail: Cell::new(N),
            buffer: buffer.clone(),
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

        if curr_cl_tail == N {
            let next_tail = (curr_tail + 1) & self.buffer.cache_lines;
            let curr_head = self.buffer.head.load(Ordering::Acquire);

            if next_tail == curr_head {
                return Err(Error::QueueEmpty);
            }

            // Safety: curr_tail is always within bounds and never overlaps with the write head
            let cache_line = unsafe { self.buffer.inner.get_unchecked(next_tail) };
            let value = unsafe { cache_line.read(0) };

            self.inner_tail.set(next_tail);
            self.inner_cl_tail.set(1);
            self.buffer.tail.store(next_tail, Ordering::Release);

            Ok(value)
        } else {
            // Safety: curr_tail is always within bounds and never overlaps with the write head
            let cache_line = unsafe { self.buffer.inner.get_unchecked(curr_tail) };
            let value = unsafe { cache_line.read(curr_cl_tail) };

            self.inner_cl_tail.set(curr_cl_tail + 1);

            Ok(value)
        }
    }

    #[inline]
    fn recv_one(&self, mut curr_tail: usize) -> T {
        let mut curr_cl_tail = self.inner_cl_tail.get();

        // if we finished reading from the current cache line
        if curr_cl_tail == N {
            // calculate the index of the next cache line by wrapping around buffer bounds
            let next_tail = (curr_tail + 1) & self.buffer.cache_lines;

            curr_cl_tail = 0;
            curr_tail = next_tail;

            // sync it with the write thread
            self.buffer.tail.store(next_tail, Ordering::Release);
            self.inner_tail.set(next_tail);
        }

        // Safety: curr_tail is always within bounds and never overlaps with the write head
        let cache_line = unsafe { self.buffer.inner.get_unchecked(curr_tail) };
        let value = unsafe { cache_line.read(curr_cl_tail) };

        self.inner_cl_tail.set(curr_cl_tail + 1);

        value
    }

    pub fn flush_recv(&mut self) -> FlushIter<'_, T, N> {
        let until_head = self.buffer.head.load(Ordering::Acquire);

        let until_cl_head = unsafe {
            self.buffer
                .inner_cl_head_ptr
                .load(Ordering::Acquire)
                .cast_const()
                .read()
                .get()
        };

        // TODO: probably needs until_cl_head - 1 since the producer is lazy wrapping
        // actually probably fine since consumer is also lazy wrapping
        let until_pos = Buffer::<T, N>::get_pos(until_head, until_cl_head);

        FlushIter {
            consumer: self,
            until_pos,
        }
    }
}

pub struct FlushIter<'a, T, const N: usize> {
    consumer: &'a mut Consumer<T, N>,
    until_pos: usize,
}

impl<T, const N: usize> Iterator for FlushIter<'_, T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        let tail = self.consumer.inner_tail.get();
        let cl_tail = self.consumer.inner_cl_tail.get();
        let curr_pos = Buffer::<T, N>::get_pos(tail, cl_tail);

        if curr_pos == self.until_pos {
            return None;
        }

        let tail = self.consumer.inner_tail.get();
        let value = self.consumer.recv_one(tail);

        Some(value)
    }
}
