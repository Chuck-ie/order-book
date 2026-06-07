const MAX_SPINS: usize = 5;

pub struct Spinlock {
    spin_count: usize,
}

impl Spinlock {
    #[must_use]
    pub const fn new() -> Self {
        Self { spin_count: 0 }
    }

    /// returns true if the lock is still able to continue to spin
    /// returns false if the ``spin_count`` has reached it limit, signalling that it should be stopped
    pub fn spin(&mut self) -> bool {
        // spin just initialized, so we dont need to wait yet
        if self.spin_count == 0 {
            self.spin_count += 1;
            true
        }
        // wait for the current spinlock
        else {
            let spins = 1 << self.spin_count.min(MAX_SPINS);

            for _ in 0..spins {
                std::hint::spin_loop();
            }

            self.spin_count += 1;
            self.spin_count <= MAX_SPINS
        }
    }
}
