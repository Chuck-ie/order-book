#[cfg(test)]
mod tests {
    use order_book::{
        common::OrderIdU32,
        slot_map::{
            Linkable, SlotMap, TestableSlotMap, optimized::OptimizedSlotMap,
            standard::StandardSlotMap,
        },
    };

    macro_rules! test_slot_map {
        ($name:ident, $slot_map:ty) => {
            mod $name {
                use super::*;

                #[test]
                fn default_equiv_new_test() {
                    default_equiv_new::<$slot_map>();
                }

                #[test]
                fn init_slot_map_test() {
                    init_slot_map::<$slot_map>();
                }

                #[test]
                fn insert_single_test() {
                    insert_single::<$slot_map>();
                }

                #[test]
                fn insert_multiple_test() {
                    insert_multiple::<$slot_map>();
                }

                #[test]
                fn insert_middle_test() {
                    insert_middle::<$slot_map>();
                }

                #[test]
                fn remove_middle_test() {
                    remove_middle::<$slot_map>();
                }

                #[test]
                fn remove_head_test() {
                    remove_head::<$slot_map>();
                }

                #[test]
                fn remove_tail_test() {
                    remove_tail::<$slot_map>();
                }

                #[test]
                fn remove_last_test() {
                    remove_last::<$slot_map>();
                }
            }
        };
    }

    pub trait TestTrait:
        SlotMap<Data = u32, Id = OrderIdU32> + TestableSlotMap<Data = u32, Utype = u32> + Default
    {
    }

    impl<T> TestTrait for T where
        T: SlotMap<Data = u32, Id = OrderIdU32>
            + TestableSlotMap<Data = u32, Utype = u32>
            + Default
    {
    }

    test_slot_map!(sm_standard, StandardSlotMap<u32>);
    test_slot_map!(sm_optimized, OptimizedSlotMap<u32>);

    pub fn default_equiv_new<SM: TestTrait>() {
        let slot_map_new = SM::new();
        let slot_map_default = SM::default();

        assert_eq!(slot_map_default.head(), None, "Default head should be None");
        assert_eq!(
            slot_map_default.free_head(),
            None,
            "Default free_head should be None"
        );
        assert_eq!(slot_map_default.capacity(), 0);

        assert_eq!(slot_map_new.head(), slot_map_default.head());
        assert_eq!(slot_map_new.tail(), slot_map_default.tail());
        assert_eq!(slot_map_new.free_head(), slot_map_default.free_head());
    }

    pub fn init_slot_map<SM: TestTrait>() {
        let slot_map = SM::new();

        assert_eq!(None, slot_map.head());
        assert_eq!(None, slot_map.tail());
        assert_eq!(None, slot_map.free_head());
        assert_eq!(0, slot_map.total());
        assert_eq!(0, slot_map.capacity());
        assert!(slot_map.is_empty());
    }

    pub fn insert_single<SM: TestTrait>() {
        let mut slot_map = SM::new();
        let _idx = slot_map.insert(0);

        assert_eq!(Some(0), slot_map.head());
        assert_eq!(Some(0), slot_map.tail());
        assert_eq!(None, slot_map.free_head());
        assert_eq!(1, slot_map.total());
        assert_eq!(1, slot_map.capacity());
        assert!(!slot_map.is_empty());

        let link = slot_map.get_link(0);
        assert_links(link, None, None);

        assert!(slot_map.is_occupied(0, 0));
    }

    pub fn insert_multiple<SM: TestTrait>() {
        let mut slot_map = SM::new();
        let _idx1 = slot_map.insert(0);
        let _idx2 = slot_map.insert(1);
        let _idx3 = slot_map.insert(2);

        assert_eq!(Some(0), slot_map.head());
        assert_eq!(Some(2), slot_map.tail());
        assert_eq!(None, slot_map.free_head());
        assert_eq!(3, slot_map.total());
        assert_eq!(3, slot_map.capacity());
        assert!(!slot_map.is_empty());

        let link1 = slot_map.get_link(0);
        assert_links(link1, None, Some(1));

        let link2 = slot_map.get_link(1);
        assert_links(link2, Some(0), Some(2));

        let link3 = slot_map.get_link(2);
        assert_links(link3, Some(1), None);
    }

    pub fn insert_middle<SM: TestTrait>() {
        let mut slot_map = SM::new();
        let _idx1 = slot_map.insert(0);
        let idx2 = slot_map.insert(1);
        let _idx3 = slot_map.insert(2);
        slot_map.remove(idx2);

        let _idx4 = slot_map.insert(3);

        assert_eq!(Some(0), slot_map.head());
        assert_eq!(Some(1), slot_map.tail());
        assert_eq!(None, slot_map.free_head());
        assert_eq!(3, slot_map.total());
        assert_eq!(3, slot_map.capacity());
        assert!(!slot_map.is_empty());

        let link1 = slot_map.get_link(0);
        assert_links(link1, None, Some(2));

        let link3 = slot_map.get_link(2);
        assert_links(link3, Some(0), Some(1));

        let link4 = slot_map.get_link(1);
        assert_links(link4, Some(2), None);
    }

    pub fn remove_middle<SM: TestTrait>() {
        let mut slot_map = SM::new();
        let _idx1 = slot_map.insert(0);
        let idx2 = slot_map.insert(1);
        let _idx3 = slot_map.insert(2);

        slot_map.remove(idx2);

        assert_eq!(Some(0), slot_map.head());
        assert_eq!(Some(2), slot_map.tail());
        assert_eq!(Some(1), slot_map.free_head());
        assert_eq!(3, slot_map.total());
        assert_eq!(2, slot_map.capacity());
        assert!(!slot_map.is_empty());

        let link1 = slot_map.get_link(0);
        assert_links(link1, None, Some(2));

        let link2 = slot_map.get_link(1);
        assert!(link2.is_none());

        let link3 = slot_map.get_link(2);
        assert_links(link3, Some(0), None);
    }

    pub fn remove_head<SM: TestTrait>() {
        let mut slot_map = SM::new();
        let idx1 = slot_map.insert(0);
        let _idx2 = slot_map.insert(1);
        let _idx3 = slot_map.insert(2);
        slot_map.remove(idx1);

        assert_eq!(Some(1), slot_map.head());
        assert_eq!(Some(2), slot_map.tail());
        assert_eq!(Some(0), slot_map.free_head());
        assert_eq!(3, slot_map.total());
        assert_eq!(2, slot_map.capacity());
        assert!(!slot_map.is_empty());

        let link1 = slot_map.get_link(0);
        assert!(link1.is_none());

        let link2 = slot_map.get_link(1);
        assert_links(link2, None, Some(2));

        let link3 = slot_map.get_link(2);
        assert_links(link3, Some(1), None);
    }

    pub fn remove_tail<SM: TestTrait>() {
        let mut slot_map = SM::new();
        let _idx1 = slot_map.insert(0);
        let _idx2 = slot_map.insert(1);
        let idx3 = slot_map.insert(2);
        slot_map.remove(idx3);

        assert_eq!(Some(0), slot_map.head());
        assert_eq!(Some(1), slot_map.tail());
        assert_eq!(Some(2), slot_map.free_head());
        assert_eq!(3, slot_map.total());
        assert_eq!(2, slot_map.capacity());
        assert!(!slot_map.is_empty());

        let link1 = slot_map.get_link(0);
        assert_links(link1, None, Some(1));

        let link2 = slot_map.get_link(1);
        assert_links(link2, Some(0), None);

        let link3 = slot_map.get_link(2);
        assert!(link3.is_none());
    }

    pub fn remove_last<SM: TestTrait>() {
        let mut slot_map = SM::new();
        let idx1 = slot_map.insert(0);
        slot_map.remove(idx1);

        assert!(slot_map.head().is_none());
        assert!(slot_map.tail().is_none());
        assert_eq!(Some(0), slot_map.free_head());
        assert_eq!(1, slot_map.total());
        assert_eq!(0, slot_map.capacity());
        assert!(slot_map.is_empty());

        let link1 = slot_map.get_link(0);
        assert!(link1.is_none());
    }

    fn assert_links<L: Linkable>(
        link: Option<&L>,
        expected_prev: Option<usize>,
        expected_next: Option<usize>,
    ) {
        assert!(link.is_some());
        assert_eq!(link.unwrap().prev(), expected_prev);
        assert_eq!(link.unwrap().next(), expected_next);
    }
}
