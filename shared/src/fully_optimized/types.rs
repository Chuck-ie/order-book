use std::num::NonZeroUsize;

#[repr(transparent)]
#[derive(Copy, Clone)]
pub struct NonZeroIndex(NonZeroUsize);

impl NonZeroIndex {
    #[must_use]
    pub const fn from_raw(index: usize) -> Self {
        debug_assert!(index != 0, "only non 0 indices are allowed");
        unsafe { Self(NonZeroUsize::new(index + 1).unwrap_unchecked()) }
    }

    #[must_use]
    pub const fn to_raw(self) -> usize {
        self.0.get() - 1
    }
}

pub trait SentinelNoneValue {
    const NONE_VALUE: u64 = (1 << 63) - 1;

    #[doc(hidden)]
    fn as_64(&self) -> u64;

    fn as_optional(&self) -> Option<u64> {
        let value = self.as_64();

        if value == Self::NONE_VALUE {
            None
        } else {
            Some(value)
        }
    }
}

impl SentinelNoneValue for u64 {
    fn as_64(&self) -> u64 {
        *self
    }
}

impl SentinelNoneValue for NonZeroIndex {
    fn as_64(&self) -> u64 {
        self.0.get() as u64
    }
}
