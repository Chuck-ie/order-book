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
