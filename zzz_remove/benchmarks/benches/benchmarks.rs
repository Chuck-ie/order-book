// use std::{hint::black_box, path::Path};
//
// use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
// use serde::Deserialize;
// use shared::{
//     LimitOrderRequest, MatcherCommand, OrderMatcherExt, OrderSide, ob_naive, ob_slot_map_optimized,
//     ob_slot_map_standard, ob_standard,
// };
//
// #[inline(always)]
// #[allow(clippy::inline_always)]
// fn run<ID, M>(mut matcher: M, commands: Vec<MatcherCommand<ID>>)
// where
//     M: OrderMatcherExt<OrderId = ID>,
// {
//     for cmd in commands {
//         black_box(matcher.process(black_box(cmd)));
//     }
// }
//
// macro_rules! bench_impl {
//     ($c:expr, $bench_name:expr, [ $( ($name:expr, $matcher:ty) ),* ], $setup:ident) => {
//         let mut group = $c.benchmark_group($bench_name);
//         // let sizes = [1_000, 10_000, 100_000];
//         let sizes = [100_000];
//
//         for n in sizes {
//             let (samples, measurement) = match n {
//                 100_000 => (10, 10),
//                 // 100_000 => (100, 10),
//                 10_000  => (250, 20),
//                 _       => (500, 30),
//             };
//
//             $(
//                 let bench_id = format!("{}/{}", $name, n);
//                 group.sample_size(samples);
//                 group.measurement_time(std::time::Duration::from_secs(measurement));
//                 group.throughput(Throughput::Elements(n as u64));
//                 group.warm_up_time(std::time::Duration::from_secs(2));
//                 group.noise_threshold(0.02);
//
//                 group.bench_function(&bench_id, |b| {
//                     b.iter_batched(
//                         || $setup::<$matcher>(n),
//                         |(matcher, commands)| run(matcher, commands),
//                         BatchSize::LargeInput
//                     )
//                 });
//             )*
//         }
//
//         group.finish()
//     };
// }
//
// fn bench_place_order_same_level(c: &mut Criterion) {
//     fn setup<M: OrderMatcherExt>(n: usize) -> (M, Vec<MatcherCommand<M::OrderId>>) {
//         let matcher = M::new();
//         let mut commands = Vec::with_capacity(n);
//
//         (0..n).for_each(|_| {
//             commands.push(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1));
//         });
//
//         (matcher, commands)
//     }
//
//     bench_impl!(
//         c,
//         "place_order_same_level",
//         [
//             ("ob_naive", ob_naive::OrderMatcher),
//             ("ob_standard", ob_standard::OrderMatcher),
//             ("ob_slot_map_standard", ob_slot_map_standard::OrderMatcher),
//             ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher)
//         ],
//         setup
//     );
// }
//
// fn bench_place_order_different_levels(c: &mut Criterion) {
//     fn setup<M: OrderMatcherExt>(n: usize) -> (M, Vec<MatcherCommand<M::OrderId>>) {
//         let matcher = M::new();
//         let mut commands = Vec::with_capacity(n);
//
//         #[allow(clippy::cast_possible_truncation)]
//         (0..n).for_each(|i| {
//             commands.push(MatcherCommand::new_limit_order(OrderSide::Bid, i as u64, 1));
//         });
//
//         (matcher, commands)
//     }
//
//     bench_impl!(
//         c,
//         "place_order_different_levels",
//         [
//             ("ob_naive", ob_naive::OrderMatcher),
//             ("ob_standard", ob_standard::OrderMatcher),
//             ("ob_slot_map_standard", ob_slot_map_standard::OrderMatcher),
//             ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher)
//         ],
//         setup
//     );
// }
//
// fn bench_cancel_order_same_level(c: &mut Criterion) {
//     fn setup<M: OrderMatcherExt>(n: usize) -> (M, Vec<MatcherCommand<M::OrderId>>) {
//         let mut matcher = M::new();
//         let mut commands = Vec::with_capacity(n);
//
//         (0..n).for_each(|_| {
//             let id = matcher
//                 .process(MatcherCommand::new_limit_order(OrderSide::Bid, 100, 1))
//                 .unwrap();
//
//             commands.push(MatcherCommand::CancelOrder(id));
//         });
//
//         (matcher, commands)
//     }
//
//     bench_impl!(
//         c,
//         "cancel_order_same_level",
//         [
//             ("ob_naive", ob_naive::OrderMatcher),
//             ("ob_standard", ob_standard::OrderMatcher),
//             ("ob_slot_map_standard", ob_slot_map_standard::OrderMatcher),
//             ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher)
//         ],
//         setup
//     );
// }
//
// fn bench_cancel_order_different_levels(c: &mut Criterion) {
//     fn setup<M: OrderMatcherExt>(n: usize) -> (M, Vec<MatcherCommand<M::OrderId>>) {
//         let mut matcher = M::new();
//         let mut commands = Vec::with_capacity(n);
//
//         #[allow(clippy::cast_possible_truncation)]
//         (0..n).for_each(|i| {
//             let id = matcher
//                 .process(MatcherCommand::new_limit_order(OrderSide::Bid, i as u64, 1))
//                 .unwrap();
//
//             commands.push(MatcherCommand::CancelOrder(id));
//         });
//
//         (matcher, commands)
//     }
//
//     bench_impl!(
//         c,
//         "cancel_order_different_levels",
//         [
//             ("ob_naive", ob_naive::OrderMatcher),
//             ("ob_standard", ob_standard::OrderMatcher),
//             ("ob_slot_map_standard", ob_slot_map_standard::OrderMatcher),
//             ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher)
//         ],
//         setup
//     );
// }
//
// fn bench_real_data(c: &mut Criterion) {
//     #[derive(Debug, Deserialize)]
//     struct CsvOrder {
//         pub time: f32,
//         pub order_type: i8,
//         pub id: u32,
//         pub size: u64,
//         pub price: u64,
//         pub side: i8,
//     }
//
//     fn load_order_commands<ID>(ticker: &str) -> Vec<MatcherCommand<ID>> {
//         let base_dir = env!("CARGO_MANIFEST_DIR");
//         let path = std::path::PathBuf::from(base_dir)
//             .join("data")
//             .join(format!("{ticker}.csv"));
//
//         if !Path::new(&path).exists() {
//             eprintln!("file doesnt exist");
//             return vec![];
//         }
//
//         csv::ReaderBuilder::new()
//             .has_headers(false)
//             .from_path(&path)
//             .expect("failed to open CSV")
//             .deserialize::<CsvOrder>()
//             .filter_map(|result| {
//                 let record: CsvOrder = result.expect("failed to deserialize CSV row");
//
//                 if record.order_type == 1 {
//                     Some(MatcherCommand::PlaceOrder(LimitOrderRequest {
//                         side: record.side.into(),
//                         amount: record.size,
//                         limit: record.price,
//                     }))
//                 } else {
//                     None
//                 }
//             })
//             .collect()
//     }
//
//     macro_rules! bench_real_data {
//         ($c:expr, $bench_name:expr, [ $( ($name:expr, $matcher:ty) ),* ], $setup:ident) => {
//             let mut group = $c.benchmark_group($bench_name);
//             group.sample_size(10);
//             group.measurement_time(std::time::Duration::from_secs(10));
//             group.warm_up_time(std::time::Duration::from_secs(2));
//             group.noise_threshold(0.02);
//
//             $(
//                 let tickers = ["AAPL", "AMZN", "GOOG", "INTC", "MSFT"];
//                 for ticker in tickers {
//
//                     // is down double setup which loads the csv's twice each, but since its
//                     // not part of the benchmark itself and the csv's aren't that large
//                     // i really dont mind for now
//                     let bench_id = format!("{}_{}", $name, ticker);
//                     let (_, commands) = $setup::<$matcher>(ticker);
//                     let command_count = commands.len() as u64;
//
//                     group.throughput(criterion::Throughput::Elements(command_count));
//                     group.bench_function(&bench_id, |b| {
//                         b.iter_batched(
//                             || $setup::<$matcher>(ticker),
//                             |(matcher, commands)| run(matcher, commands),
//                             BatchSize::LargeInput,
//                         )
//                     });
//                 }
//             )*
//
//             group.finish();
//         };
//     }
//
//     fn setup<M: OrderMatcherExt>(ticker: &str) -> (M, Vec<MatcherCommand<M::OrderId>>) {
//         let order_commands = load_order_commands::<M::OrderId>(ticker);
//         let matcher = M::new();
//
//         (matcher, order_commands)
//     }
//
//     bench_real_data!(
//         c,
//         "LOBSTER_real_data",
//         [
//             ("ob_naive", ob_naive::OrderMatcher),
//             ("ob_standard", ob_standard::OrderMatcher),
//             ("ob_slot_map_standard", ob_slot_map_standard::OrderMatcher),
//             ("ob_slot_map_optimized", ob_slot_map_optimized::OrderMatcher)
//         ],
//         setup
//     );
// }
//
// criterion_group!(
//     benches,
//     bench_real_data,
//     bench_place_order_same_level,
//     bench_place_order_different_levels,
//     bench_cancel_order_same_level,
//     bench_cancel_order_different_levels
// );
// criterion_main!(benches);
