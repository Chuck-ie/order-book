use charming::{
    Chart, HtmlRenderer,
    component::{Axis, DataZoom, DataZoomType, Legend, Title},
    element::{AxisType, ItemStyle, JsFunction, NameLocation, Tooltip, Trigger},
    series::{Bar, Scatter},
};
use order_book::common::OrderSide;
use std::sync::{Mutex, OnceLock};

use crate::shared::SyntheticOrder;

// fn main() {
//     macro_rules! plot_order_profile {
//         ($name:expr, $order_profile:expr) => {
//             let orders = generate_synthetic_orders(1_000, $order_profile);
//             let chart = create_orderbook_scatter_chart($name, &orders);
//             HtmlRenderer::new("Order Distribution", 1000, 800)
//                 .save(&chart, format!("charts/{}.html", $name))
//                 .unwrap();
//         };
//     }
//
//     plot_order_profile!("NARROW", &NARROW);
//     plot_order_profile!("WIDE", &WIDE);
//     plot_order_profile!("P1", &P1);
//     plot_order_profile!("P2", &P2);
//     plot_order_profile!("P3", &P3);
//     plot_order_profile!("P4", &P4);
//     plot_order_profile!("P5", &P5);
//     plot_order_profile!("P6", &P6);
//     plot_order_profile!("P7", &P7);
// }

#[allow(dead_code)]
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn create_orderbook_scatter_chart(profile_name: &str, orders: &[SyntheticOrder]) -> Chart {
    let mut bid_data: Vec<Vec<f64>> = Vec::new();
    let mut ask_data: Vec<Vec<f64>> = Vec::new();

    for (index, order) in orders.iter().enumerate() {
        let base_size = (order.amount as f64).sqrt();
        let final_size = base_size.clamp(2.5, 30.0);
        let point = vec![
            index as f64,
            order.limit as f64,
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
                .item_style(ItemStyle::new().color("#3B82F6").opacity(0.4)),
        )
        .series(
            Scatter::new()
                .name("Asks")
                .data(ask_data)
                .symbol_size(JsFunction::new_with_args("data", "return data[3];"))
                .item_style(ItemStyle::new().color("#EF4444").opacity(0.4)),
        )
}

#[derive(Debug, Clone)]
pub struct SMemProfSnapshot {
    pub name: String,
    pub id: String,
    pub alloc_bytes_mb: f64,
    pub dealloc_bytes_mb: f64,
    pub grow_bytes_mb: f64,
}

pub static BENCH_RESULTS: OnceLock<Mutex<Vec<SMemProfSnapshot>>> = OnceLock::new();

pub fn get_results_registry() -> &'static Mutex<Vec<SMemProfSnapshot>> {
    BENCH_RESULTS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn update_shared_memory_chart(file_name: &str) {
    let filtered_results = get_results_registry()
        .lock()
        .unwrap()
        .clone()
        .into_iter()
        .filter(|r| r.name == file_name)
        .collect::<Vec<_>>();

    if filtered_results.is_empty() {
        return;
    }

    let ids: Vec<String> = filtered_results.iter().map(|r| r.id.clone()).collect();
    let alloc_data: Vec<f64> = filtered_results.iter().map(|r| r.alloc_bytes_mb).collect();
    let dealloc_data: Vec<f64> = filtered_results
        .iter()
        .map(|r| r.dealloc_bytes_mb)
        .collect();

    let grow_data: Vec<f64> = filtered_results.iter().map(|r| r.grow_bytes_mb).collect();

    let chart = Chart::new()
        .title(Title::new().text(format!("Memory Footprint: {file_name}")))
        .tooltip(Tooltip::new())
        .legend(Legend::new().data(vec![
            "Alloc Bytes (MB)",
            "Dealloc Bytes (MB)",
            "Grow Bytes (MB)",
        ]))
        .x_axis(Axis::new().type_(AxisType::Category).data(ids))
        .y_axis(Axis::new().type_(AxisType::Value).name("Memory (MB)"))
        .series(Bar::new().name("Alloc Bytes (MB)").data(alloc_data))
        .series(Bar::new().name("Dealloc Bytes (MB)").data(dealloc_data))
        .series(Bar::new().name("Grow Bytes (MB)").data(grow_data));

    HtmlRenderer::new("Memory Benchmark Chart", 1800, 600)
        .save(&chart, format!("charts/{file_name}"))
        .expect("Failed to save memory chart");
}
