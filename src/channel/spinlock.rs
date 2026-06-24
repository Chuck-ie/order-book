use std::cell::Cell;

const SOFT_LIMIT: usize = 6;
const HARD_LIMIT: usize = 12;

pub struct Spinlock {
    spin_count: Cell<usize>,
}

impl Spinlock {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            spin_count: Cell::new(0),
        }
    }

    #[inline]
    pub fn spin(&mut self) -> bool {
        let spins = 1 << self.spin_count.get().min(SOFT_LIMIT);

        for _ in 0..spins {
            std::hint::spin_loop();
        }

        if self.spin_count.get() <= SOFT_LIMIT {
            self.spin_count.set(self.spin_count.get() + 1);
        }

        self.spin_count.get() <= SOFT_LIMIT
    }

    #[inline]
    pub fn spin_heavy(&self) -> bool {
        let spins = 1 << self.spin_count.get().min(SOFT_LIMIT);

        if self.spin_count.get() <= SOFT_LIMIT {
            for _ in 0..spins {
                std::hint::spin_loop();
            }
        } else {
            for _ in 0..spins {
                std::hint::spin_loop();
            }

            std::thread::yield_now();
        }

        if self.spin_count.get() <= HARD_LIMIT {
            self.spin_count.set(self.spin_count.get() + 1);
        }

        self.spin_count.get() <= HARD_LIMIT
    }
}

impl Default for Spinlock {
    fn default() -> Self {
        Self::new()
    }
}
