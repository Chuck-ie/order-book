use std::sync::atomic::Ordering;

use crate::channel::{
    buffer_handle::{BufferHandle, FromBuffer, MC, SC},
    spinlock::Spinlock,
};

pub trait Consumer {
    type Item;

    fn try_read(&self) -> Option<Self::Item>;
}

impl<T> Consumer for BufferHandle<T, SC> {
    type Item = T;

    fn try_read(&self) -> Option<Self::Item> {
        let inner = self.inner();
        let curr_read_index = inner.tail.load(Ordering::Relaxed);
        let curr_write_index = inner.head.load(Ordering::Acquire);

        if curr_read_index == curr_write_index {
            return None;
        }

        let slot = unsafe { inner.buffer.get_unchecked(curr_read_index) };
        let value = unsafe { slot.read() };
        let next_read_index = (curr_read_index + 1) & inner.capacity;

        inner.tail.store(next_read_index, Ordering::Release);
        slot.written.store(false, Ordering::Release);

        Some(value)
    }
}

impl<T> Clone for BufferHandle<T, MC> {
    fn clone(&self) -> Self {
        Self::new(self.inner())
    }
}

impl<T> Consumer for BufferHandle<T, MC> {
    type Item = T;

    fn try_read(&self) -> Option<Self::Item> {
        let inner = self.inner();
        let mut spinlock = Spinlock::new();
        let mut curr_read_index = inner.tail.load(Ordering::Relaxed);

        loop {
            let curr_write_index = inner.head.load(Ordering::Relaxed);

            if curr_read_index == curr_write_index {
                return None;
            }

            let next_read_index = (curr_read_index + 1) & inner.capacity;
            let slot = unsafe { inner.buffer.get_unchecked(curr_read_index) };
            let slot_written = slot.written.load(Ordering::Acquire);

            if slot_written {
                match inner.tail.compare_exchange_weak(
                    curr_read_index,
                    next_read_index,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Safety
                        //
                        // We check that curr_read_index is allowed to read and always update it to be a valid
                        // index inside our buffer, so we can always get the read_ptr and read from it.
                        let value = unsafe { slot.read() };
                        slot.written.store(false, Ordering::Release);

                        return Some(value);
                    }
                    Err(updated_read_index) => {
                        curr_read_index = updated_read_index;

                        if !spinlock.spin() {
                            return None;
                        }
                    }
                }
            } else if !spinlock.spin() {
                return None;
            } else {
                std::thread::yield_now();
            }
        }
    }
}
