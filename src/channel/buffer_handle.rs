use std::{marker::PhantomData, sync::Arc};

use crate::channel::ring_buffer::RingBuffer;

pub trait FromBuffer<T> {
    fn new(buffer: &Arc<RingBuffer<T>>) -> Self;
}

pub struct BufferHandle<T, M: Mode> {
    state: Arc<RingBuffer<T>>,
    _mode: PhantomData<M>,
}

impl<T, M: Mode> BufferHandle<T, M> {
    #[inline]
    pub(crate) const fn inner(&self) -> &Arc<RingBuffer<T>> {
        &self.state
    }
}

impl<T, M: Mode> FromBuffer<T> for BufferHandle<T, M> {
    fn new(buffer: &Arc<RingBuffer<T>>) -> Self {
        Self {
            state: buffer.clone(),
            _mode: PhantomData,
        }
    }
}

pub trait Mode {}

/// Single Producer type state
pub struct SP;
impl Mode for SP {}
pub type SingleProducer<T> = BufferHandle<T, SP>;

/// Single Consumer type state
pub struct SC;
impl Mode for SC {}
pub type SingleConsumer<T> = BufferHandle<T, SC>;

/// Multi Producer type state
pub struct MP;
impl Mode for MP {}
pub type MultiProducer<T> = BufferHandle<T, MP>;

/// Multi Consumer type state
pub struct MC;
impl Mode for MC {}
pub type MultiConsumer<T> = BufferHandle<T, MC>;
