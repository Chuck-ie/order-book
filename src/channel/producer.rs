use std::{mem::MaybeUninit, sync::atomic::Ordering};

use crate::channel::{
    buffer_handle::{BufferHandle, MP},
    spinlock::Spinlock,
};

pub trait Producer {
    type Item;

    fn try_write(&self, value: Self::Item) -> bool;
}

impl<T> Producer for BufferHandle<T, MP> {
    type Item = T;

    fn try_write(&self, value: Self::Item) -> bool {
        let mut spinlock = Spinlock::new();
        let mut curr_write_index = self.head.load(Ordering::Relaxed);

        loop {
            let curr_read_index = self.tail.load(Ordering::Relaxed);

            // this is a faster modulo operation, which only works under the assumption that N is a power of 2
            if (curr_write_index + 1) & self.capacity == curr_read_index {
                return false;
            }

            let next_write_index = (curr_write_index + 1) & self.capacity;

            match self.head.compare_exchange_weak(
                curr_write_index,
                next_write_index,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    unsafe {
                        let write_ptr = self.buffer.get_unchecked(curr_write_index).get();
                        write_ptr.write(MaybeUninit::new(value));
                    }

                    return true;
                }
                Err(updated_write_index) => {
                    curr_write_index = updated_write_index;

                    if !spinlock.spin() {
                        return false;
                    }
                }
            }
        }
    }
}
