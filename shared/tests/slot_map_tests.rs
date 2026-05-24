#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use shared::{
        Linkable, SlotMap, TestableSlotMap, slot_map::optimized::SlotMapOptimized,
        slot_map::standard::SlotMapStandard,
    };

    macro_rules! test_slot_map_impl {
        ($name:ident, $slot_map:ty, $utype:ty) => {
            mod $name {
                use super::*;

                #[test]
                fn default_equiv_new_test() {
                    super::default_equiv_new::<$slot_map, $utype>();
                }

                #[test]
                fn init_arena_test() {
                    super::init_arena::<$slot_map, $utype>();
                }

                #[test]
                fn insert_single_test() {
                    super::insert_single::<$slot_map, $utype>();
                }

                #[test]
                fn insert_multiple_test() {
                    super::insert_multiple::<$slot_map, $utype>();
                }

                #[test]
                fn insert_middle_test() {
                    super::insert_middle::<$slot_map, $utype>();
                }

                #[test]
                fn remove_middle_test() {
                    super::remove_middle::<$slot_map, $utype>();
                }

                #[test]
                fn remove_head_test() {
                    super::remove_head::<$slot_map, $utype>();
                }

                #[test]
                fn remove_tail_test() {
                    super::remove_tail::<$slot_map, $utype>();
                }

                #[test]
                fn remove_last_test() {
                    super::remove_last::<$slot_map, $utype>();
                }
            }
        };
    }

    test_slot_map_impl!(sm_standard, SlotMapStandard<u32>, u32);
    test_slot_map_impl!(sm_optimized, SlotMapOptimized<u32>, u32);

    pub trait TestSlotMap<U>:
        SlotMap<Data = u32, Id = U> + TestableSlotMap<Data = u32, Utype = U>
    {
    }

    impl<T, U> TestSlotMap<U> for T where
        T: SlotMap<Data = u32, Id = U> + TestableSlotMap<Data = u32, Utype = U>
    {
    }

    pub trait TestUtype: TryFrom<usize> + TryInto<usize> + Debug + PartialEq + Copy {}

    impl<T> TestUtype for T where T: TryFrom<usize> + TryInto<usize> + Debug + PartialEq + Copy {}

    pub fn default_equiv_new<SM, U>()
    where
        SM: TestSlotMap<U> + Default,
        U: TestUtype,
    {
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

    pub fn init_arena<SM, U>()
    where
        SM: TestSlotMap<U>,
        U: TestUtype,
    {
        let slot_map = SM::new();

        assert_eq!(None, slot_map.head());
        assert_eq!(None, slot_map.tail());
        assert_eq!(None, slot_map.free_head());
        assert_eq!(0, slot_map.total());
        assert_eq!(0, slot_map.capacity());
        assert!(slot_map.is_empty());
    }

    pub fn insert_single<SM, U>()
    where
        SM: TestSlotMap<U>,
        U: TestUtype,
    {
        let mut slot_map = SM::new();
        let idx = slot_map.insert(0);

        assert_eq!(Some(idx), slot_map.head());
        assert_eq!(Some(idx), slot_map.tail());
        assert_eq!(None, slot_map.free_head());
        assert_eq!(1, slot_map.total());
        assert_eq!(1, slot_map.capacity());
        assert!(!slot_map.is_empty());

        let link = slot_map.get_link(0);
        assert_links(link, None, None);

        assert!(slot_map.is_occupied(idx.try_into().ok().unwrap(), 0));
    }

    pub fn insert_multiple<SM, U>()
    where
        SM: TestSlotMap<U>,
        U: TestUtype,
    {
        let mut slot_map = SM::new();
        let idx1 = slot_map.insert(0);
        let _idx2 = slot_map.insert(1);
        let idx3 = slot_map.insert(2);

        assert_eq!(Some(idx1), slot_map.head());
        assert_eq!(Some(idx3), slot_map.tail());
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

    pub fn insert_middle<SM, U>()
    where
        SM: TestSlotMap<U>,
        U: TestUtype,
    {
        let mut slot_map = SM::new();
        let idx1 = slot_map.insert(0);
        let idx2 = slot_map.insert(1);
        let _idx3 = slot_map.insert(2);
        slot_map.remove(idx2);

        let idx4 = slot_map.insert(3);

        assert_eq!(Some(idx1), slot_map.head());
        assert_eq!(Some(idx4), slot_map.tail());
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

    pub fn remove_middle<SM, U>()
    where
        SM: TestSlotMap<U>,
        U: TestUtype,
    {
        let mut slot_map = SM::new();
        let idx1 = slot_map.insert(0);
        let idx2 = slot_map.insert(1);
        let idx3 = slot_map.insert(2);

        slot_map.remove(idx2);

        assert_eq!(Some(idx1), slot_map.head());
        assert_eq!(Some(idx3), slot_map.tail());
        assert_eq!(U::try_from(1).ok(), slot_map.free_head());
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

    pub fn remove_head<SM, U>()
    where
        SM: TestSlotMap<U>,
        U: TestUtype,
    {
        let mut slot_map = SM::new();
        let idx1 = slot_map.insert(0);
        let idx2 = slot_map.insert(1);
        let idx3 = slot_map.insert(2);
        slot_map.remove(idx1);

        assert_eq!(Some(idx2), slot_map.head());
        assert_eq!(Some(idx3), slot_map.tail());
        assert_eq!(U::try_from(0).ok(), slot_map.free_head());
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

    pub fn remove_tail<SM, U>()
    where
        SM: TestSlotMap<U>,
        U: TestUtype,
    {
        let mut slot_map = SM::new();
        let idx1 = slot_map.insert(0);
        let idx2 = slot_map.insert(1);
        let idx3 = slot_map.insert(2);
        slot_map.remove(idx3);

        assert_eq!(Some(idx1), slot_map.head());
        assert_eq!(Some(idx2), slot_map.tail());
        assert_eq!(U::try_from(2).ok(), slot_map.free_head());
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

    pub fn remove_last<SM, U>()
    where
        SM: TestSlotMap<U>,
        U: TestUtype,
    {
        let mut slot_map = SM::new();
        let idx1 = slot_map.insert(0);
        slot_map.remove(idx1);

        assert!(slot_map.head().is_none());
        assert!(slot_map.tail().is_none());
        assert_eq!(U::try_from(0).ok(), slot_map.free_head());
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
