use std::fmt::Debug;

pub mod optimized;
pub mod standard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct NonMaxU32(pub u32);

impl NonMaxU32 {
    const NONE_VALUE: u32 = u32::MAX;

    #[must_use]
    pub const fn new_none() -> Self {
        Self(Self::NONE_VALUE)
    }

    #[must_use]
    pub const fn is_none(&self) -> bool {
        self.0 == Self::NONE_VALUE
    }

    #[must_use]
    pub const fn is_some(&self) -> bool {
        self.0 != Self::NONE_VALUE
    }

    #[must_use]
    pub fn from(value: u32) -> Self {
        debug_assert!(
            value < Self::NONE_VALUE,
            "value out of range: {} is >= {}",
            value,
            Self::NONE_VALUE
        );

        Self(value)
    }
}

impl From<u32> for NonMaxU32 {
    fn from(value: u32) -> Self {
        Self::from(value)
    }
}

pub trait SlotMap {
    type Id;
    type Data;

    fn new() -> Self;
    fn insert(&mut self, data: Self::Data) -> Self::Id;
    fn remove(&mut self, remove_idx: Self::Id);

    fn total(&self) -> usize;
    fn capacity(&self) -> usize;
    fn is_empty(&self) -> bool;

    fn get(&self, index: usize) -> Option<&Self::Data>;
    fn get_mut(&mut self, index: usize) -> Option<&mut Self::Data>;
}

pub trait TestableSlotMap {
    type Data: PartialEq;
    type Utype: TryFrom<usize> + Debug + PartialEq + Copy;

    fn head(&self) -> Option<Self::Utype>;
    fn tail(&self) -> Option<Self::Utype>;
    fn free_head(&self) -> Option<Self::Utype>;
    fn is_occupied(&self, index: usize, data: Self::Data) -> bool;
    fn get_link(&self, index: usize) -> Option<&impl Linkable>;
}

pub trait Linkable {
    fn prev(&self) -> Option<usize>;
    fn next(&self) -> Option<usize>;
}
