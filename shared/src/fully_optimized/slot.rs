use std::marker::PhantomData;

#[repr(transparent)]
pub struct Slot<S: SlotState>(u64, PhantomData<S>);

pub trait SlotState {}
pub struct Tagged;
pub struct Free;
pub struct Occupied;

impl SlotState for Tagged {}
impl SlotState for Free {}
impl SlotState for Occupied {}

impl<S: SlotState> Slot<S> {
    #[must_use]
    // #[inline(always)]
    pub const fn value(&self) -> u64 {
        // ex(free): 0010 & !(1000) = 0010 & (0111) = 0010
        // ex(occu): 1010 & !(1000) = 1010 & (0111) = 0010
        self.0 & !(1 << 63)
    }

    pub fn set_value(&mut self, new_value: u64) {
        debug_assert!((new_value & (1 << 63)) == 0, "Value exceeds 63-bit range");
        let high_bit = self.0 & (1 << 63);
        self.0 = high_bit | new_value;
    }
}

impl Slot<Tagged> {
    #[must_use]
    // #[inline(always)]
    pub const fn is_free(&self) -> bool {
        // see is_occupied
        !self.is_occupied()
    }

    #[must_use]
    // #[inline(always)]
    pub const fn is_occupied(&self) -> bool {
        // ex(free): 0010 >> 3 = 0000 = 0
        // ex(occu): 1010 >> 3 = 0001 = 1
        // self.0 >> 63 == 1

        // first bit is interpreted as a sign bit (1 means negative)
        // so we just check if its a negative number for is_occupied
        self.0.cast_signed() < 0
    }

    #[must_use]
    pub const fn try_as_free(&self) -> Option<&Slot<Free>> {
        if self.is_free() {
            unsafe { Some(&*std::ptr::from_ref(self).cast::<Slot<Free>>()) }
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_free_unchecked_mut(&mut self) -> &mut Slot<Free> {
        debug_assert!(self.is_free(), "tried casting a non Free tagged slot to Free");
        // Safety: since the arena returns an index to an occupied slot on arena insert, the caller
        // usually has the knowledge of if a slot is free or occupied. The debug assert is there
        // to help during development in case the caller makes a mistake.
        unsafe { &mut *std::ptr::from_mut(self).cast::<Slot<Free>>() }
    }

    #[must_use]
    pub fn make_free_unchecked_mut(&mut self) -> &mut Slot<Free> {
        // ex(occu): 1001 & !(1000) = 1001 & 0111 = 0001
        // ex(free): 0001 & !(1000) = 0001 & 0111 = 0001
        self.0 &= !(1 << 63);
        unsafe { &mut *std::ptr::from_mut(self).cast::<Slot<Free>>() }
    }

    #[must_use]
    pub const fn try_as_occupied(&self) -> Option<&Slot<Occupied>> {
        if self.is_occupied() {
            unsafe { Some(&*std::ptr::from_ref(self).cast::<Slot<Occupied>>()) }
        } else {
            None
        }
    }

    #[must_use]
    pub fn as_occupied_unchecked_mut(&mut self) -> &mut Slot<Occupied> {
        debug_assert!(self.is_occupied(), "tried casting a non Occupied tagged slot to Occupied");
        // Safety: since the arena returns an index to an occupied slot on arena insert, the caller
        // usually has the knowledge of if a slot is free or occupied. The debug assert is there
        // to help during development in case the caller makes a mistake.
        unsafe { &mut *std::ptr::from_mut(self).cast::<Slot<Occupied>>() }
    }

    #[must_use]
    pub fn make_occupied_unchecked_mut(&mut self) -> &mut Slot<Occupied> {
        // ex(occu): 1001 | 1000 = 1001
        // ex(free): 0001 | 1001 = 1001
        self.0 |= 1 << 63;
        unsafe { &mut *std::ptr::from_mut(self).cast::<Slot<Occupied>>() }
    }
}

impl Slot<Free> {
    pub const NONE_VALUE: u64 = (1 << 63) - 1;

    #[must_use]
    pub const fn new(value: u64) -> Self {
        debug_assert!(
            value < Self::NONE_VALUE,
            "Value exceeds 63-bit range or collieds with None sentinel value"
        );

        Self(value, PhantomData)
    }

    #[must_use]
    pub const fn new_none() -> Self {
        Self(Self::NONE_VALUE, PhantomData)
    }

    #[must_use]
    pub const fn optional_value(&self) -> Option<u64> {
        let value = self.value();

        if value == Self::NONE_VALUE {
            None
        } else {
            Some(value)
        }
    }

    #[must_use]
    pub const fn to_tagged(self) -> Slot<Tagged> {
        Slot(self.0, PhantomData)
    }
}

impl Slot<Occupied> {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        // ex(success): 0111 & 1000 = 0000 = 0
        // ex(fail):    1000 & 1000 = 1000 != 0
        debug_assert!((value & (1 << 63)) == 0, "Value exceeds 63-bit range");

        // ex: 0111 | 1000 = 1111
        Self(value | (1 << 63), PhantomData)
    }

    #[must_use]
    pub const fn to_tagged(self) -> Slot<Tagged> {
        // ex: 0010 | 1000 = 1010
        // Slot(self.0 | (1 << 63), PhantomData)

        // simpler variant since Slot::<Occupied>::new() should guarantee
        // that the last bit is set to 1 so we just need to copy it
        Slot(self.0, PhantomData)
    }
}
