#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use shared::{
        arena_naive::NaiveArena, arena_optimized::OptimizedArena, Arena, Linkable, TestableArena,
    };

    macro_rules! test_arena_impl {
        ($name:ident, $arena:ty, $utype:ty) => {
            mod $name {
                use super::*;

                #[test]
                fn default_equiv_new_test() {
                    super::default_equiv_new::<$arena, $utype>();
                }

                #[test]
                fn init_arena_test() {
                    super::init_arena::<$arena, $utype>();
                }

                #[test]
                fn insert_single_test() {
                    super::insert_single::<$arena, $utype>();
                }

                #[test]
                fn insert_multiple_test() {
                    super::insert_multiple::<$arena, $utype>();
                }

                #[test]
                fn insert_middle_test() {
                    super::insert_middle::<$arena, $utype>();
                }

                #[test]
                fn remove_middle_test() {
                    super::remove_middle::<$arena, $utype>();
                }

                #[test]
                fn remove_head_test() {
                    super::remove_head::<$arena, $utype>();
                }

                #[test]
                fn remove_tail_test() {
                    super::remove_tail::<$arena, $utype>();
                }

                #[test]
                fn remove_last_test() {
                    super::remove_last::<$arena, $utype>();
                }
            }
        };
    }

    test_arena_impl!(optimized, OptimizedArena<u32>, u32);
    test_arena_impl!(naive, NaiveArena<u32>, usize);

    pub trait TestArena<U>:
        Arena<Data = u32, Utype = U> + TestableArena<Data = u32, Utype = U>
    {
    }

    impl<T, U> TestArena<U> for T where
        T: Arena<Data = u32, Utype = U> + TestableArena<Data = u32, Utype = U>
    {
    }

    pub trait TestUtype: TryFrom<usize> + TryInto<usize> + Debug + PartialEq + Copy {}

    impl<T> TestUtype for T where T: TryFrom<usize> + TryInto<usize> + Debug + PartialEq + Copy {}

    pub fn default_equiv_new<A, U>()
    where
        A: TestArena<U> + Default,
        U: TestUtype,
    {
        let arena_new = A::new();
        let arena_default = A::default();

        assert_eq!(arena_default.head(), None, "Default head should be None");
        assert_eq!(arena_default.free_head(), None, "Default free_head should be None");
        assert_eq!(arena_default.capacity(), 0);

        assert_eq!(arena_new.head(), arena_default.head());
        assert_eq!(arena_new.tail(), arena_default.tail());
        assert_eq!(arena_new.free_head(), arena_default.free_head());
    }

    pub fn init_arena<A, U>()
    where
        A: TestArena<U>,
        U: TestUtype,
    {
        let arena = A::new();

        assert_eq!(None, arena.head());
        assert_eq!(None, arena.tail());
        assert_eq!(None, arena.free_head());
        assert_eq!(0, arena.total());
        assert_eq!(0, arena.capacity());
        assert!(arena.is_empty());
    }

    pub fn insert_single<A, U>()
    where
        A: TestArena<U>,
        U: TestUtype,
    {
        let mut arena = A::new();
        let idx = arena.insert(0);

        assert_eq!(Some(idx), arena.head());
        assert_eq!(Some(idx), arena.tail());
        assert_eq!(None, arena.free_head());
        assert_eq!(1, arena.total());
        assert_eq!(1, arena.capacity());
        assert!(!arena.is_empty());

        let link = arena.get_link(0);
        assert_links(link, None, None);

        assert!(arena.is_occupied(idx.try_into().ok().unwrap(), 0));
    }

    pub fn insert_multiple<A, U>()
    where
        A: TestArena<U>,
        U: TestUtype,
    {
        let mut arena = A::new();
        let idx1 = arena.insert(0);
        let _idx2 = arena.insert(1);
        let idx3 = arena.insert(2);

        assert_eq!(Some(idx1), arena.head());
        assert_eq!(Some(idx3), arena.tail());
        assert_eq!(None, arena.free_head());
        assert_eq!(3, arena.total());
        assert_eq!(3, arena.capacity());
        assert!(!arena.is_empty());

        let link1 = arena.get_link(0);
        assert_links(link1, None, Some(1));

        let link2 = arena.get_link(1);
        assert_links(link2, Some(0), Some(2));

        let link3 = arena.get_link(2);
        assert_links(link3, Some(1), None);
    }

    pub fn insert_middle<A, U>()
    where
        A: TestArena<U>,
        U: TestUtype,
    {
        let mut arena = A::new();
        let idx1 = arena.insert(0);
        let idx2 = arena.insert(1);
        let _idx3 = arena.insert(2);
        arena.remove(idx2);

        let idx4 = arena.insert(3);

        assert_eq!(Some(idx1), arena.head());
        assert_eq!(Some(idx4), arena.tail());
        assert_eq!(None, arena.free_head());
        assert_eq!(3, arena.total());
        assert_eq!(3, arena.capacity());
        assert!(!arena.is_empty());

        let link1 = arena.get_link(0);
        assert_links(link1, None, Some(2));

        let link3 = arena.get_link(2);
        assert_links(link3, Some(0), Some(1));

        let link4 = arena.get_link(1);
        assert_links(link4, Some(2), None);
    }

    pub fn remove_middle<A, U>()
    where
        A: TestArena<U>,
        U: TestUtype,
    {
        let mut arena = A::new();
        let idx1 = arena.insert(0);
        let idx2 = arena.insert(1);
        let idx3 = arena.insert(2);

        arena.remove(idx2);

        assert_eq!(Some(idx1), arena.head());
        assert_eq!(Some(idx3), arena.tail());
        assert_eq!(U::try_from(1).ok(), arena.free_head());
        assert_eq!(3, arena.total());
        assert_eq!(2, arena.capacity());
        assert!(!arena.is_empty());

        let link1 = arena.get_link(0);
        assert_links(link1, None, Some(2));

        let link2 = arena.get_link(1);
        assert!(link2.is_none());

        let link3 = arena.get_link(2);
        assert_links(link3, Some(0), None);
    }

    pub fn remove_head<A, U>()
    where
        A: TestArena<U>,
        U: TestUtype,
    {
        let mut arena = A::new();
        let idx1 = arena.insert(0);
        let idx2 = arena.insert(1);
        let idx3 = arena.insert(2);
        arena.remove(idx1);

        assert_eq!(Some(idx2), arena.head());
        assert_eq!(Some(idx3), arena.tail());
        assert_eq!(U::try_from(0).ok(), arena.free_head());
        assert_eq!(3, arena.total());
        assert_eq!(2, arena.capacity());
        assert!(!arena.is_empty());

        let link1 = arena.get_link(0);
        assert!(link1.is_none());

        let link2 = arena.get_link(1);
        assert_links(link2, None, Some(2));

        let link3 = arena.get_link(2);
        assert_links(link3, Some(1), None);
    }

    pub fn remove_tail<A, U>()
    where
        A: TestArena<U>,
        U: TestUtype,
    {
        let mut arena = A::new();
        let idx1 = arena.insert(0);
        let idx2 = arena.insert(1);
        let idx3 = arena.insert(2);
        arena.remove(idx3);

        assert_eq!(Some(idx1), arena.head());
        assert_eq!(Some(idx2), arena.tail());
        assert_eq!(U::try_from(2).ok(), arena.free_head());
        assert_eq!(3, arena.total());
        assert_eq!(2, arena.capacity());
        assert!(!arena.is_empty());

        let link1 = arena.get_link(0);
        assert_links(link1, None, Some(1));

        let link2 = arena.get_link(1);
        assert_links(link2, Some(0), None);

        let link3 = arena.get_link(2);
        assert!(link3.is_none());
    }

    pub fn remove_last<A, U>()
    where
        A: TestArena<U>,
        U: TestUtype,
    {
        let mut arena = A::new();
        let idx1 = arena.insert(0);
        arena.remove(idx1);

        assert!(arena.head().is_none());
        assert!(arena.tail().is_none());
        assert_eq!(U::try_from(0).ok(), arena.free_head());
        assert_eq!(1, arena.total());
        assert_eq!(0, arena.capacity());
        assert!(arena.is_empty());

        let link1 = arena.get_link(0);
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
