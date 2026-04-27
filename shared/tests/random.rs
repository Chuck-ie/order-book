#[cfg(test)]
mod tests {
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

    #[test]
    fn random() {
        println!("Slot-undefined: {}", std::mem::size_of::<Slot<Tagged>>());
        println!("Slot-free: {}", std::mem::size_of::<Slot<Free>>());
        println!("Slot-occupied: {}", std::mem::size_of::<Slot<Occupied>>());
    }
}
