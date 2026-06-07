use std::{marker::PhantomData, ops::Deref, sync::Arc};

use crate::channel::ring_buffer::RingBuffer;

pub trait FromBuffer {
    type Item;

    fn new(buffer: &Arc<RingBuffer<Self::Item>>) -> Self;
}

pub struct BufferHandle<T, M: Mode> {
    state: Arc<RingBuffer<T>>,
    _mode: PhantomData<M>,
}

impl<T, M: Mode> FromBuffer for BufferHandle<T, M> {
    type Item = T;

    fn new(buffer: &Arc<RingBuffer<Self::Item>>) -> Self {
        Self {
            state: buffer.clone(),
            _mode: PhantomData,
        }
    }
}

impl<T, M: Mode> Deref for BufferHandle<T, M> {
    type Target = Arc<RingBuffer<T>>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

pub trait Mode {}

/// Single Producer type state
pub struct SP;
impl Mode for SP {}

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
