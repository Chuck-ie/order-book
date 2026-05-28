use crate::common::{
    BenchState, BenchStateKey, OrderProfile, SyntheticOrder, generate_synthetic_orders,
};
use charming::{
    Chart, HtmlRenderer,
    component::{Axis, DataZoom, DataZoomType, Legend, Title},
    element::{AxisType, ItemStyle, JsFunction, NameLocation, Tooltip, Trigger},
    series::Scatter,
};
use divan::{AllocProfiler, Bencher, counter::ItemsCount};
use mimalloc::MiMalloc;
use order_book::{common::OrderSide, engine::v1_vec_only};

#[path = "common.rs"]
mod common;

#[global_allocator]
static ALLOC: AllocProfiler<MiMalloc> = AllocProfiler::new(MiMalloc);

fn main() {
    // divan::main();

    macro_rules! plot_order_profile {
        ($name:expr, $order_profile:expr) => {
            let orders = generate_synthetic_orders(10_000, $order_profile);
            let chart = create_orderbook_scatter_chart($name, &orders);
            HtmlRenderer::new("Order Distribution", 1000, 800)
                .save(&chart, format!("./charts/{}.html", $name))
                .unwrap();
        };
    }

    plot_order_profile!("P1", &P1);
    plot_order_profile!("P2", &P2);
    plot_order_profile!("P3", &P3);
    plot_order_profile!("P4", &P4);
    plot_order_profile!("P5", &P5);
    plot_order_profile!("P6", &P6);
    plot_order_profile!("P7", &P7);
}

#[allow(clippy::cast_precision_loss)]
fn create_orderbook_scatter_chart(profile_name: &str, orders: &[SyntheticOrder]) -> Chart {
    let mut bid_data: Vec<Vec<f64>> = Vec::new();
    let mut ask_data: Vec<Vec<f64>> = Vec::new();

    for (index, order) in orders.iter().enumerate() {
        let base_size = (order.amount as f64).sqrt();
        let final_size = base_size.clamp(2.5, 30.0);
        let point = vec![
            index as f64,
            order.price as f64,
            order.amount as f64,
            final_size,
        ];

        match order.side {
            OrderSide::Bid => bid_data.push(point),
            OrderSide::Ask => ask_data.push(point),
        }
    }

    Chart::new()
        .title(
            Title::new()
                .text(format!("Order Distribution: {profile_name}"))
                .subtext("X: Sequence | Y: Price | Size: Quantity"),
        )
        .legend(Legend::new().data(vec!["Bids", "Asks"]).top("bottom"))
        .tooltip(
            Tooltip::new()
                .trigger(Trigger::Item)
                .formatter(JsFunction::new_with_args(
                    "params",
                    "return '<b>' + params.seriesName + '</b><br/>' +
                            'Seq (Time): ' + params.data[0] + '<br/>' +
                            'Price: ' + params.data[1] + '<br/>' +
                            'Qty: ' + params.data[2];",
                )),
        )
        .data_zoom(DataZoom::new().type_(DataZoomType::Inside).x_axis_index(0))
        .data_zoom(DataZoom::new().type_(DataZoomType::Slider).x_axis_index(0))
        .data_zoom(DataZoom::new().type_(DataZoomType::Inside).y_axis_index(0))
        .data_zoom(DataZoom::new().type_(DataZoomType::Slider).y_axis_index(0))
        .x_axis(
            Axis::new()
                .type_(AxisType::Value)
                .name("Order Sequence (Time)")
                .name_location(NameLocation::Middle)
                .name_gap(30.0),
        )
        .y_axis(
            Axis::new()
                .type_(AxisType::Value)
                .name("Price Ticks")
                .scale(true),
        )
        .series(
            Scatter::new()
                .name("Bids")
                .data(bid_data)
                .symbol_size(JsFunction::new_with_args("data", "return data[3];"))
                .item_style(ItemStyle::new().color("#10B981").opacity(0.5)),
        )
        .series(
            Scatter::new()
                .name("Asks")
                .data(ask_data)
                .symbol_size(JsFunction::new_with_args("data", "return data[3];"))
                .item_style(ItemStyle::new().color("#EF4444").opacity(0.5)),
        )
}

// fn create_orderbook_scatter_chart(orders: &[SyntheticOrder]) -> Chart {
//     let mut bid_data: Vec<Vec<f64>> = Vec::new();
//     let mut ask_data: Vec<Vec<f64>> = Vec::new();
//
//     for (index, order) in orders.iter().enumerate() {
//         let base_size = (order.amount as f64).sqrt();
//         let final_size = base_size.clamp(4.0, 30.0);
//         let point = vec![index as f64, order.price as f64, final_size];
//
//         match order.side {
//             OrderSide::Bid => bid_data.push(point),
//             OrderSide::Ask => ask_data.push(point),
//         }
//     }
//
//     Chart::new()
//         .title(Title::new().text("Order Distribution"))
//         .legend(Legend::new().data(vec!["Bids", "Asks"]))
//         .tooltip(Tooltip::new().trigger(Trigger::Axis))
//         .x_axis(
//             Axis::new()
//                 .type_(AxisType::Value)
//                 .name("Order Sequence (Time)"),
//         )
//         .y_axis(
//             Axis::new()
//                 .type_(AxisType::Value)
//                 .name("Price Ticks")
//                 .scale(true),
//         )
//         .series(
//             Scatter::new()
//                 .name("Bids")
//                 .data(bid_data)
//                 .symbol_size(JsFunction::new_with_args("data", "return data[2];"))
//                 .item_style(ItemStyle::new().color("#3B82F6").opacity(0.4)),
//         )
//         .series(
//             Scatter::new()
//                 .name("Asks")
//                 .data(ask_data)
//                 .symbol_size(JsFunction::new_with_args("data", "return data[2];"))
//                 .item_style(ItemStyle::new().color("#EF4444").opacity(0.4)),
//         )
// }

macro_rules! bench_engine {
    ($group:ident, $bench_state:ty, $order_profile:expr) => {
        mod $group {
            use super::*;
            use std::cell::{LazyCell, UnsafeCell};

            thread_local! {
                static BENCH_STATE: UnsafeCell<LazyCell<$bench_state>> = const { UnsafeCell::new(LazyCell::new(<$bench_state>::default)) };
            }

            #[rustfmt::skip]
            #[divan::bench(sample_count = 100, args = [200_000])]
            fn bench_place_orders_persistent(bencher: Bencher, per_batch_orders: usize) {
                run_bench_place_orders_persistent(&BENCH_STATE, bencher, per_batch_orders, $order_profile);
            }
        }
    };
}

fn run_bench_place_orders_persistent<S: BenchState + Default>(
    state_key: BenchStateKey<S>,
    bencher: Bencher,
    per_batch_orders: usize,
    order_profile: &OrderProfile,
) {
    bencher
        .with_inputs(|| {
            state_key.with(|cell| unsafe {
                let state = &mut **cell.get();
                state.generate_input(per_batch_orders, order_profile)
            })
        })
        .input_counter(move |_| ItemsCount::new(per_batch_orders))
        .bench_local_values(|commands| {
            state_key.with(|cell| {
                let state = unsafe { &mut **cell.get() };

                for cmd in commands {
                    state.process(cmd);
                }
            });
        });
}

static P1: OrderProfile = OrderProfile::p1();
static P2: OrderProfile = OrderProfile::p2();
static P3: OrderProfile = OrderProfile::p3();
static P4: OrderProfile = OrderProfile::p4();
static P5: OrderProfile = OrderProfile::p5();
static P6: OrderProfile = OrderProfile::p6();
static P7: OrderProfile = OrderProfile::p7();

#[rustfmt::skip]
mod benches {
    use order_book::engine::{v2_btree, v3_slot_map, v4_slot_map_arena};
    use crate::common::{ArenaBenchState, DefaultBenchState};
    use super::*;

    // v1
    bench_engine!(v1_vec_only_benches_p1, DefaultBenchState<v1_vec_only::matcher::OrderMatcher>, &P1);
    bench_engine!(v1_vec_only_benches_p2, DefaultBenchState<v1_vec_only::matcher::OrderMatcher>, &P2);
    bench_engine!(v1_vec_only_benches_p3, DefaultBenchState<v1_vec_only::matcher::OrderMatcher>, &P3);
    bench_engine!(v1_vec_only_benches_p4, DefaultBenchState<v1_vec_only::matcher::OrderMatcher>, &P4);
    bench_engine!(v1_vec_only_benches_p6, DefaultBenchState<v1_vec_only::matcher::OrderMatcher>, &P6);

    // v2
    bench_engine!(v2_btree_benches_p1, DefaultBenchState<v2_btree::matcher::OrderMatcher>, &P1);
    bench_engine!(v2_btree_benches_p2, DefaultBenchState<v2_btree::matcher::OrderMatcher>, &P2);
    bench_engine!(v2_btree_benches_p3, DefaultBenchState<v2_btree::matcher::OrderMatcher>, &P3);
    bench_engine!(v2_btree_benches_p4, DefaultBenchState<v2_btree::matcher::OrderMatcher>, &P4);
    bench_engine!(v2_btree_benches_p6, DefaultBenchState<v2_btree::matcher::OrderMatcher>, &P6);
    
    // v3
    bench_engine!(v3_slot_map_benches_p1, DefaultBenchState<v3_slot_map::matcher::OrderMatcher>, &P1);
    bench_engine!(v3_slot_map_benches_p2, DefaultBenchState<v3_slot_map::matcher::OrderMatcher>, &P2);
    bench_engine!(v3_slot_map_benches_p3, DefaultBenchState<v3_slot_map::matcher::OrderMatcher>, &P3);
    bench_engine!(v3_slot_map_benches_p4, DefaultBenchState<v3_slot_map::matcher::OrderMatcher>, &P4);
    bench_engine!(v3_slot_map_benches_p6, DefaultBenchState<v3_slot_map::matcher::OrderMatcher>, &P6);

    // v4
    bench_engine!(v4_slot_map_arena_p1, ArenaBenchState<v4_slot_map_arena::matcher::OrderMatcher>, &P1);
    bench_engine!(v4_slot_map_arena_p2, ArenaBenchState<v4_slot_map_arena::matcher::OrderMatcher>, &P2);
    bench_engine!(v4_slot_map_arena_p3, ArenaBenchState<v4_slot_map_arena::matcher::OrderMatcher>, &P3);
    bench_engine!(v4_slot_map_arena_p4, ArenaBenchState<v4_slot_map_arena::matcher::OrderMatcher>, &P4);
    bench_engine!(v4_slot_map_arena_p6, ArenaBenchState<v4_slot_map_arena::matcher::OrderMatcher>, &P6);
}
