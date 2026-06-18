use std::{
    cell::Cell,
    ptr,
    sync::{Arc, atomic::Ordering},
};

use crate::{channel::spinlock::Spinlock, spsc::Buffer};

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    QueueFull,
    BatchTooLarge,
}

pub struct Producer<T, const N: usize> {
    cl_index: Cell<usize>,
    cl_offset: Cell<usize>,
    buffer: Arc<Buffer<T, N>>,
}

impl<T, const N: usize> Producer<T, N> {
    pub(crate) fn new(buffer: &Arc<Buffer<T, N>>) -> Self {
        Self {
            cl_index: Cell::new(0),
            cl_offset: Cell::new(0),
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
        let curr_head = self.cl_index.get();
        let curr_cl_head = self.cl_offset.get();

        // slow path when trying to wrap around at a cache line border
        // if we finished writing to a cache line in the previous send
        if curr_cl_head == N {
            // Calculate the index of the next cache line by wrapping around buffer bounds using
            // fast modulo since cache_lines is always a power of 2
            let next_head = (curr_head + 1) & self.buffer.cl_mask;

            // Sync with the reader's release when advancing its tail
            let curr_tail = self.buffer.tail.load(Ordering::Acquire);

            if next_head == curr_tail {
                return Err((value, Error::QueueFull));
            }

            // Safety: curr_head is exclusively owned by the writer and is within bounds
            unsafe {
                self.buffer
                    .write_counts
                    .get_unchecked(curr_head)
                    .get()
                    .write(N);
            };

            // Safety: next_head is verified to not overlap with curr_tail and is within bounds
            let next_cache_line = unsafe { self.buffer.get_cache_line(next_head) };
            unsafe { next_cache_line.write(0, value) };

            self.cl_index.set(next_head);
            self.cl_offset.set(1);

            // Sync the advancement with the read thread
            self.buffer.head.store(next_head, Ordering::Release);
        }
        // fast path for the currently exclusively owned cache line
        else {
            // Safety: curr_head is always within bounds and never overlaps with the read head
            let cache_line = unsafe { self.buffer.get_cache_line(curr_head) };
            unsafe { cache_line.write(curr_cl_head, value) };

            self.cl_offset.set(curr_cl_head + 1);
        }

        Ok(())
    }

    pub fn try_send_batch(&self, buf: &[T]) -> Result<usize, Error>
    where
        T: Copy,
    {
        let max_batch_size = self.buffer.capacity - N;
        let batch_size = buf.len().min(max_batch_size);
        let final_batch_size = batch_size.min(self.free_slots());

        if final_batch_size == 0 {
            return Err(Error::QueueFull);
        }

        Ok(unsafe { self.send_batch_exact_unchecked(&buf[0..final_batch_size]) })
    }

    pub fn try_send_batch_exact(&self, buf: &[T]) -> Result<usize, Error>
    where
        T: Copy,
    {
        let batch_size = buf.len();
        let max_batch_size = self.buffer.capacity - N;

        if batch_size > max_batch_size || batch_size > self.free_slots() {
            return Err(Error::BatchTooLarge);
        }

        Ok(unsafe { self.send_batch_exact_unchecked(buf) })
    }

    // # Safety: The caller has to make sure to validate that there are buf.len()
    // items free to write to the buffer
    unsafe fn send_batch_exact_unchecked(&self, buf: &[T]) -> usize
    where
        T: Copy,
    {
        let batch_size = buf.len();
        let curr_cl_index = self.cl_index.get();
        let curr_cl_offset = self.cl_offset.get();
        let last_abs_index = self.buffer.capacity;
        let from_abs_index = (curr_cl_index * N) + curr_cl_offset;
        let to_abs_index = from_abs_index + batch_size;

        if to_abs_index < last_abs_index {
            let s_ptr = unsafe { self.get_slice_ptr(curr_cl_index, curr_cl_offset) };
            unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), s_ptr, batch_size) };
        } else {
            let s1_len = last_abs_index - from_abs_index;
            let s1_ptr = unsafe { self.get_slice_ptr(curr_cl_index, curr_cl_offset) };
            unsafe { ptr::copy_nonoverlapping(buf.as_ptr(), s1_ptr, s1_len) };

            let s2_len = batch_size - s1_len;
            let s2_ptr = unsafe { self.get_slice_ptr(0, 0) };
            unsafe { ptr::copy_nonoverlapping(buf.as_ptr().add(s1_len), s2_ptr, s2_len) };
        }

        let final_abs_index = to_abs_index % self.buffer.capacity;
        let next_cl_index = (final_abs_index / N) & self.buffer.cl_mask;
        let next_cl_offset = final_abs_index % N;

        self.cl_index.set(next_cl_index);
        self.cl_offset.set(next_cl_offset);

        let mut i = curr_cl_index;

        while i != next_cl_index {
            unsafe { self.buffer.write_counts.get_unchecked(i).get().write(N) }
            i = (i + 1) & self.buffer.cl_mask;
        }

        self.buffer.head.store(next_cl_index, Ordering::Release);

        batch_size
    }

    pub fn try_reserve(&self, size: usize) -> Result<SendReservation<'_, T, N>, Error>
    where
        T: Copy,
    {
        let max_batch_size = self.buffer.capacity - N;
        let reservation_size = size.min(max_batch_size).min(self.free_slots());

        if reservation_size == 0 {
            return Err(Error::QueueFull);
        }

        Ok(unsafe { self.reserve_exact_unchecked(reservation_size) })
    }

    pub fn try_reserve_exact(&self, size: usize) -> Result<SendReservation<'_, T, N>, Error>
    where
        T: Copy,
    {
        let max_batch_size = self.buffer.capacity - N;

        if size > max_batch_size || size > self.free_slots() {
            return Err(Error::BatchTooLarge);
        }

        Ok(unsafe { self.reserve_exact_unchecked(size) })
    }

    unsafe fn reserve_exact_unchecked(&self, size: usize) -> SendReservation<'_, T, N>
    where
        T: Copy,
    {
        let curr_cl_index = self.cl_index.get();
        let curr_cl_offset = self.cl_offset.get();
        let last_abs_index = self.buffer.capacity;
        let from_abs_index = (curr_cl_index * N) + curr_cl_offset;
        let to_abs_index = from_abs_index + size;

        let (s1, s1_remaining, s2, s2_remaining) = if to_abs_index < last_abs_index {
            let s_ptr = unsafe { self.get_slice_ptr(curr_cl_index, curr_cl_offset) };
            (s_ptr, size, std::ptr::null_mut(), 0)
        } else {
            let s1_len = last_abs_index - from_abs_index;
            let s1_ptr = unsafe { self.get_slice_ptr(curr_cl_index, curr_cl_offset) };
            let s2_len = size - s1_len;
            let s2_ptr = unsafe { self.get_slice_ptr(0, 0) };

            (s1_ptr, s1_len, s2_ptr, s2_len)
        };

        SendReservation {
            tx: self,
            s1,
            s1_remaining,
            s2,
            s2_remaining,
            total_reserved: size,
            start_cl_index: curr_cl_index,
            start_cl_offset: curr_cl_offset,
        }
    }

    pub fn flush(&self) -> Result<(), Error> {
        let curr_cl_index = self.cl_index.get();
        let next_head = (curr_cl_index + 1) & self.buffer.cl_mask;

        // Sync with the reader's release when advancing its tail
        let curr_tail = self.buffer.tail.load(Ordering::Acquire);

        if next_head == curr_tail {
            return Err(Error::QueueFull);
        }

        self.cl_index.set(next_head);
        self.cl_offset.set(0);

        // case(curr_cl_head == 0): means 0 has not yet been written
        // case(curr_cl_head == 1): means 1 has not yet been written
        // case(curr_cl_head == N): means N has not yet been written
        // Safety: curr_head is exclusively owned by the writer and is within bounds
        unsafe {
            self.buffer
                .write_counts
                .get_unchecked(curr_cl_index)
                .get()
                .write(curr_cl_index);
        }

        self.buffer.head.store(next_head, Ordering::Release);

        Ok(())
    }

    #[inline]
    fn free_slots(&self) -> usize {
        let curr_cl_index = self.cl_index.get();
        let curr_cl_offset = self.cl_offset.get();
        let tail_cl_index = self.buffer.tail.load(Ordering::Acquire);

        let free_cache_lines = tail_cl_index.wrapping_sub(curr_cl_index) & self.buffer.cl_mask;
        (free_cache_lines * N).saturating_sub(N - curr_cl_offset)
    }

    // # Safety: the caller has to make sure that the index start_pos = (cl_index * N) + cl_offset
    // is within the ring buffers bounds
    #[inline]
    unsafe fn get_slice_ptr(&self, cl_index: usize, cl_offset: usize) -> *mut T {
        unsafe {
            (&*self.buffer.inner.get())
                .get_unchecked(cl_index)
                .get_item_ptr(cl_offset)
                .cast::<T>()
        }
    }
}

pub struct SendReservation<'a, T, const N: usize> {
    tx: &'a Producer<T, N>,
    s1: *mut T,
    s1_remaining: usize,
    s2: *mut T,
    s2_remaining: usize,
    total_reserved: usize,
    start_cl_index: usize,
    start_cl_offset: usize,
}

impl<T, const N: usize> SendReservation<'_, T, N> {
    pub fn send(&mut self, value: T) -> Option<()> {
        if self.s1_remaining > 0 {
            unsafe {
                self.s1.write(value);
                self.s1 = self.s1.add(1);
            }

            self.s1_remaining -= 1;
            Some(())
        } else if self.s2_remaining > 0 {
            unsafe {
                self.s2.write(value);
                self.s2 = self.s2.add(1);
            }

            self.s2_remaining -= 1;
            Some(())
        } else {
            None
        }
    }

    unsafe fn finalize_reservation(&self) {
        let total_remaining = self.s1_remaining + self.s2_remaining;
        let total_sent = self.total_reserved - total_remaining;

        if total_sent == 0 {
            return;
        }

        let from_abs_index = (self.start_cl_index * N) + self.start_cl_offset;
        let to_abs_index = from_abs_index + total_sent;

        let final_abs_index = to_abs_index % self.tx.buffer.capacity;
        let next_cl_index = (final_abs_index / N) & self.tx.buffer.cl_mask;
        let next_cl_offset = final_abs_index % N;

        self.tx.cl_index.set(next_cl_index);
        self.tx.cl_offset.set(next_cl_offset);

        let mut i = self.start_cl_index;

        while i != next_cl_index {
            unsafe { self.tx.buffer.write_counts.get_unchecked(i).get().write(N) }
            i = (i + 1) & self.tx.buffer.cl_mask;
        }

        self.tx.buffer.head.store(next_cl_index, Ordering::Release);
    }
}

impl<T, const N: usize> Drop for SendReservation<'_, T, N> {
    fn drop(&mut self) {
        unsafe { self.finalize_reservation() }
    }
}
