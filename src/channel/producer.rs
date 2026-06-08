use std::{mem::MaybeUninit, sync::atomic::Ordering};

use crate::channel::{
    buffer_handle::{BufferHandle, FromBuffer, MP},
    spinlock::Spinlock,
};

#[derive(Debug, PartialEq, Eq)]
pub enum ProducerError {
    QueueFull,
    Timeout,
}

pub trait Producer {
    type Item;

    fn try_write(&self, value: Self::Item) -> Result<(), ProducerError>;
}

impl<T> Clone for BufferHandle<T, MP> {
    fn clone(&self) -> Self {
        Self::new(self.inner())
    }
}

impl<T> Producer for BufferHandle<T, MP> {
    type Item = T;

    /*
     *  (000 + 001) & (011) -> 001 & 011 = 001
     *  (001 + 001) & (011) -> 010 & 011 = 010
     *  (010 + 001) & (011) -> 011 & 011 = 011
     *  (011 + 001) & (011) -> 100 & 011 = 000
     *  (100 + 001) & (011) -> 101 & 011 = 001
     *  (101 + 001) & (011) -> 110 & 011 = 010
     *
     *
     *  division? !011 = 100
     *  0 000 & 100 = 000
     *  1 001 & 100 = 000
     *  2 010 & 100 = 000
     *  3 011 & 100 = 000
     *  4 100 & 100 = 100
     *  81000 & 100 = 000
     */

    fn try_write(&self, value: Self::Item) -> Result<(), ProducerError> {
        todo!();
        // let inner = self.inner();
        // let mut spinlock = Spinlock::new();
        // let mut curr_write_index = inner.head.load(Ordering::Relaxed);
        //
        // loop {
        //     let curr_read_index = inner.tail.load(Ordering::Relaxed);
        //
        //     // this is a faster modulo operation, which only works under the assumption that N is a power of 2
        //     let plus_one = curr_write_index + 1;
        //     let next_write_index = plus_one & inner.capacity;
        //
        //     if next_write_index == curr_read_index {
        //         return Err(ProducerError::QueueFull);
        //     }
        //
        //     let slot = unsafe { inner.buffer.get_unchecked(curr_write_index) };
        //     let ticket = slot.ticket.load(Ordering::Acquire);
        //
        //     match inner.head.compare_exchange_weak(
        //         curr_write_index,
        //         next_write_index,
        //         Ordering::SeqCst,
        //         Ordering::Relaxed,
        //     ) {
        //         Ok(_) => {
        //             // Safety
        //             //
        //             // we make sure above that we sync other threads by aquiring access and only
        //             // releasing it after the write happened
        //             unsafe {
        //                 slot.write(MaybeUninit::new(value));
        //             }
        //
        //             return Ok(());
        //         }
        //         Err(updated_write_index) => {
        //             curr_write_index = updated_write_index;
        //
        //             if !spinlock.spin() {
        //                 return Err(ProducerError::Timeout);
        //             }
        //         }
        //     }
        // }
    }
}
