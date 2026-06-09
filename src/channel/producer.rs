use crate::channel::{
    buffer_handle::{BufferHandle, FromBuffer, MP, SP},
    spinlock::Spinlock,
};
use std::{mem::MaybeUninit, sync::atomic::Ordering};

#[derive(Debug, PartialEq, Eq)]
pub enum ProducerError {
    QueueFull,
    Timeout,
}

pub trait Producer {
    type Item;

    fn try_write(&self, value: Self::Item) -> Result<(), ProducerError>;
}

impl<T> Producer for BufferHandle<T, SP> {
    type Item = T;

    fn try_write(&self, value: Self::Item) -> Result<(), ProducerError> {
        let inner = self.inner();
        let curr_write_index = inner.head.load(Ordering::Relaxed);
        let curr_read_index = inner.tail.load(Ordering::Acquire);
        let next_write_index = (curr_write_index + 1) & inner.capacity;

        if next_write_index == curr_read_index {
            return Err(ProducerError::QueueFull);
        }

        let slot = unsafe { inner.buffer.get_unchecked(curr_write_index) };

        unsafe {
            slot.write(MaybeUninit::new(value));
        };

        inner.head.store(next_write_index, Ordering::Release);
        slot.written.store(true, Ordering::Release);

        Ok(())
    }
}

impl<T> Clone for BufferHandle<T, MP> {
    fn clone(&self) -> Self {
        Self::new(self.inner())
    }
}

impl<T> Producer for BufferHandle<T, MP> {
    type Item = T;

    fn try_write(&self, value: Self::Item) -> Result<(), ProducerError> {
        let inner = self.inner();
        let mut spinlock = Spinlock::new();
        let mut curr_write_index = inner.head.load(Ordering::Relaxed);

        loop {
            let curr_read_index = inner.tail.load(Ordering::Relaxed);

            // this is a faster modulo operation, which only works under the assumption that N is a power of 2
            let next_write_index = (curr_write_index + 1) & inner.capacity;

            if next_write_index == curr_read_index {
                return Err(ProducerError::QueueFull);
            }

            let slot = unsafe { inner.buffer.get_unchecked(curr_write_index) };
            let slot_written = slot.written.load(Ordering::Relaxed);

            if !slot_written {
                match inner.head.compare_exchange_weak(
                    curr_write_index,
                    next_write_index,
                    Ordering::SeqCst,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        // Safety
                        //
                        // we make sure above that we sync other threads by aquiring access and only
                        // releasing it after the write happened
                        unsafe {
                            slot.write(MaybeUninit::new(value));
                        }

                        slot.written.store(true, Ordering::Release);

                        return Ok(());
                    }
                    Err(updated_write_index) => {
                        curr_write_index = updated_write_index;

                        if !spinlock.spin() {
                            return Err(ProducerError::Timeout);
                        }
                    }
                }
            } else if !spinlock.spin() {
                return Err(ProducerError::Timeout);
            }
        }
    }
}
