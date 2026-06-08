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
        let curr_read_index = inner.tail.load(Ordering::Acquire);
        let curr_write_index = inner.head.load(Ordering::Relaxed);

        if curr_read_index == curr_write_index {
            return None;
        }

        let next_read_index = (curr_read_index + 1) & inner.capacity;
        let value = unsafe { inner.buffer.get_unchecked(curr_read_index).read() };
        inner.tail.store(next_read_index, Ordering::Release);

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

    // examples for when a thread is able to read or write
    /*  empty r==w:
     *  [0, 1, 2, 3]
     *   ^rw
     *  -> w CAN write, r can NOT read
     *
     *  full r-1==w
     *  [0, 1, 2, 3]
     *   ^r       ^w
     *  -> w can NOT write, r CAN read
     *
     *  else
     *  [0, 1, 2, 3]
     *   ^r    ^w
     *  -> w CAN write, r CAN read
     */

    /*
     *  [0, 1, 2, 3] -> write 2 values
     *   ^r12
     *   ^w12
     *
     *  case1:  - w1 and w2 both see widx=0;
     *          - both try CAS(0, 1); one fails, one succeeds;
     *          - failed one tries again
     *          - successful one needs to store slot_access at his widx to widx+1
     *
     */

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

                    return Some(value);
                }
                Err(updated_read_index) => {
                    curr_read_index = updated_read_index;

                    if !spinlock.spin() {
                        return None;
                    }
                }
            }
        }
    }
}
