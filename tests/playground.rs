use order_book::engine::LimitOrder;

// #[repr(align(64))]
pub struct PaddedWrapper<T>(T);

#[test]
pub fn playground() {
    println!("size_of<LimitOrder>: {}", std::mem::size_of::<LimitOrder>());
    println!(
        "size_of<PaddedWrapper<LimitOrder>>: {}",
        std::mem::size_of::<PaddedWrapper<LimitOrder>>()
    );
}
