// use std::collections::{HashMap, hash_map};
//
// use csv::Reader;
// use order_book::{
//     arena_allocator::ArenaId,
//     common::{LimitOrderRequest, MatcherCommand},
//     engine::LimitOrder,
// };
// use serde::Deserialize;
//
// use crate::shared::bench_engine::BenchEngine;
//
// mod shared;
//
// fn main() {
//     todo!();
// }
//
// #[derive(Debug, Deserialize)]
// struct CsvOrder {
//     pub time: f32,
//     pub order_type: i8,
//     pub id: u32,
//     pub size: u64,
//     pub price: u64,
//     pub side: i8,
// }
//
// enum LobsterOrderCommand {
//     PlaceOrder(CsvOrder),
//     CancelOrder(u32),
// }
//
// impl From<CsvOrder> for MatcherCommand<LimitOrder, ArenaId> {
//     fn from(value: CsvOrder) -> Self {
//         MatcherCommand::PlaceOrder(LimitOrder {
//             limit: value.price as u32,
//             amount: value.size as u32,
//             side: value.side.into(),
//         })
//     }
// }
//
// impl<OrderId: Clone> From<CsvOrder> for MatcherCommand<LimitOrderRequest, OrderId> {
//     fn from(value: CsvOrder) -> Self {
//         MatcherCommand::PlaceOrder(LimitOrderRequest {
//             limit: value.price,
//             amount: value.size,
//             side: value.side.into(),
//         })
//     }
// }
//
// fn bench_lobster_data() {
//     fn bench_fn<Engine: BenchEngine>()
//     where
//         Engine::Command: From<CsvOrder>,
//     {
//         let tickers = vec!["AAPL", "AMZN", "GOOG", "INTC", "MSFT"];
//
//         for ticker in &tickers {
//             let mut engine = Engine::default();
//             let mut reader = Reader::from_path(format!("data/{ticker}.csv")).unwrap();
//             let commands: Vec<LobsterOrderCommand> = reader
//                 .deserialize()
//                 .filter_map(|row| {
//                     let row: CsvOrder = row.unwrap();
//
//                     if row.order_type == 1 {
//                         Some(LobsterOrderCommand::PlaceOrder(row))
//                     } else if row.order_type == 3 {
//                         Some(LobsterOrderCommand::CancelOrder(row.id))
//                     } else {
//                         None
//                     }
//                 })
//                 .collect();
//
//             let hashmap = HashMap::with_capacity(commands.len());
//
//             for cmd in commands {
//                 match cmd {
//                     LobsterOrderCommand::PlaceOrder(order) => {
//                         if let Some(order_id) = engine.process(std::hint::black_box(order.into())) {
//                                 hashmap.insert(order.id, order_id);
//                         }
//                     }
//                     LobsterOrderCommand::CancelOrder(id) {
//                         engine.process(Engine::Command::)
//                     }
//                 }
//
//                 engine.process(mapped_cmd);
//             }
//
//             let chunk_size = 1_000;
//
//             // for chunk_idx in rows.chunks_exact(chunk_size) {
//             //     for i in 0..chunk_size {
//             //         match commands {
//             //             LobsterOrderCommand::PlaceOrder(order) => engine.process(comm),
//             //         }
//             //     }
//             // }
//         }
//     }
// }
