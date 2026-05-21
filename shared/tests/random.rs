#[cfg(test)]
mod tests {
    use shared::slot_map_unsafe::{Slot, SlotMapUnsafe};

    #[test]
    fn random() {
        let test_vec_1: Vec<u32> = vec![];
        let test_vec_2: Vec<u32> = Vec::with_capacity(16);
        // let test_vec_3: Vec<u32> = Vec::with(16);

        println!("test_vec_1: {}", test_vec_1.len());
        println!("test_vec_2: {}", test_vec_2.len());

        println!("size_of: {}", std::mem::size_of::<Slot<u32>>());
    }
}
