use charming::{
    Chart, HtmlRenderer,
    component::{Axis, Legend, Title},
    element::{AxisType, Tooltip},
    series::{Bar, Line},
};
use csv::Reader;
use serde::Deserialize;
use std::collections::HashSet;

use crate::shared::{
    LEVEL_SCALINGS, MEMORY_FOOTPRINT_PLACE_ORDERS_CSV_PATH, PersistentScalingOrderThroughputRow,
    THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_NARROW_CSV_PATH,
    THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_WIDE_CSV_PATH, smem_prof::SMemProfRow,
};

mod shared;

const MEMORY_FOOTPRINT_ALLOC_CHART_PATH: &str = "benches/results/memory_footprint_alloc.html";
const MEMORY_FOOTPRINT_GROW_CHART_PATH: &str = "benches/results/memory_footprint_grow.html";

const THROUGHPUT_PLACE_ORDERS_LEVEL_SCALING_CSV_PATH: &str =
    "benches/results/throughput_place_orders_level_scaling.csv";

const THROUGHPUT_PLACE_ORDERS_LEVEL_SCALING_CHART_PATH: &str =
    "benches/results/throughput_place_orders_level_scaling.html";

const THROUGHPUT_CANCEL_ORDERS_LEVEL_SCALING_CSV_PATH: &str =
    "benches/results/throughput_cancel_orders_level_scaling.csv";

const THROUGHPUT_CANCEL_ORDERS_LEVEL_SCALING_CHART_PATH: &str =
    "benches/results/throughput_cancel_orders_level_scaling.html";

const THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_NARROW_CHART_PATH: &str =
    "benches/results/throughput_place_orders_persistent_scaling_all_narrow.html";

const THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_WIDE_CHART_PATH: &str =
    "benches/results/throughput_place_orders_persistent_scaling_all_wide.html";

const CRITERION_RESULTS_CSV_PATH: &str = "benches/results/criterion_results.csv";

#[derive(Deserialize, Clone)]
pub struct CriterionResultRow {
    pub engine: String,
    pub command_type: String,
    pub order_strategy: String,
    pub levels: usize,
    pub orders_per_level: usize,
    pub m_orders_per_second: f64,
}

#[derive(Clone, Copy)]
enum ChartKind {
    Alloc,
    Grow,
}

#[derive(Deserialize)]
struct LevelScalingOrderThroughputRow {
    pub engine: String,
    pub total_levels: usize,
    pub orders_per_level: usize,
    pub m_orders_per_second: f64,
}

#[derive(Clone, Copy)]
enum OrderThroughputKind {
    PlaceOrders,
    CancelOrders,
}

fn main() {
    let mut reader = Reader::from_path(MEMORY_FOOTPRINT_PLACE_ORDERS_CSV_PATH).unwrap();
    let smem_prof_rows = reader
        .deserialize()
        .map(|row| row.unwrap())
        .collect::<Vec<SMemProfRow>>();

    let level_scaling_labels = LEVEL_SCALINGS
        .iter()
        .map(|(levels, orders)| format!("levels_{levels}/orders_{orders}"))
        .collect::<Vec<String>>();

    create_chart_memory_profiles(ChartKind::Alloc, &level_scaling_labels, &smem_prof_rows);
    create_chart_memory_profiles(ChartKind::Grow, &level_scaling_labels, &smem_prof_rows);

    let mut reader = Reader::from_path(THROUGHPUT_PLACE_ORDERS_LEVEL_SCALING_CSV_PATH).unwrap();
    let throughput_rows = reader
        .deserialize()
        .map(|row| row.unwrap())
        .collect::<Vec<LevelScalingOrderThroughputRow>>();

    create_chart_level_scaling_throughput(
        OrderThroughputKind::PlaceOrders,
        &level_scaling_labels,
        &throughput_rows,
    );

    let mut reader = Reader::from_path(THROUGHPUT_CANCEL_ORDERS_LEVEL_SCALING_CSV_PATH).unwrap();
    let throughput_rows = reader
        .deserialize()
        .map(|row| row.unwrap())
        .collect::<Vec<LevelScalingOrderThroughputRow>>();

    create_chart_level_scaling_throughput(
        OrderThroughputKind::CancelOrders,
        &level_scaling_labels,
        &throughput_rows,
    );

    let mut reader =
        Reader::from_path(THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_NARROW_CSV_PATH).unwrap();
    let throughput_rows = reader
        .deserialize()
        .map(|row| row.unwrap())
        .collect::<Vec<PersistentScalingOrderThroughputRow>>();

    create_chart_persistent_scaling_throughput(
        &throughput_rows,
        THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_NARROW_CHART_PATH,
    );

    let mut reader =
        Reader::from_path(THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_WIDE_CSV_PATH).unwrap();
    let throughput_rows = reader
        .deserialize()
        .map(|row| row.unwrap())
        .collect::<Vec<PersistentScalingOrderThroughputRow>>();

    create_chart_persistent_scaling_throughput(
        &throughput_rows,
        THROUGHPUT_PLACE_ORDERS_PERSISTENT_SCALING_ALL_WIDE_CHART_PATH,
    );

    create_criterion_result_charts();
}

#[allow(clippy::cast_precision_loss)]
fn create_chart_memory_profiles(
    chart_kind: ChartKind,
    level_scaling_labels: &[String],
    smem_prof_rows: &[SMemProfRow],
) {
    // 1_048_576 is just 1024^2. It therefore converts Bytes to MegaBytes
    let map_row = match chart_kind {
        ChartKind::Alloc => |row: &SMemProfRow| (row.alloc_bytes as f64) / 1_048_576.0,
        ChartKind::Grow => |row: &SMemProfRow| (row.grow_bytes as f64) / 1_048_576.0,
    };

    let chart_file_name = match chart_kind {
        ChartKind::Alloc => MEMORY_FOOTPRINT_ALLOC_CHART_PATH,
        ChartKind::Grow => MEMORY_FOOTPRINT_GROW_CHART_PATH,
    };

    let x_axis_name = match chart_kind {
        ChartKind::Alloc => "Allocations in MB",
        ChartKind::Grow => "Growth in MB",
    };

    let chart_title = match chart_kind {
        ChartKind::Alloc => "Bench Memory Allocations",
        ChartKind::Grow => "Bench Memory Growth",
    };

    let get_engine_data = |engine_name: &str| -> Vec<f64> {
        level_scaling_labels
            .iter()
            .map(|target_label| {
                smem_prof_rows
                    .iter()
                    .find(|row| {
                        row.engine == engine_name
                            && format!(
                                "levels_{}/orders_{}",
                                row.total_levels, row.orders_per_level
                            ) == *target_label
                    })
                    .map_or(0.0, map_row)
            })
            .collect()
    };

    let chart = Chart::new()
        .title(Title::new().text(chart_title))
        .tooltip(Tooltip::new())
        .legend(Legend::new())
        .x_axis(
            Axis::new()
                .name("Engines")
                .type_(AxisType::Category)
                .data(level_scaling_labels.to_vec()),
        )
        .y_axis(Axis::new().name(x_axis_name).type_(AxisType::Value))
        .series(
            Bar::new()
                .name("EngineV1")
                .data(get_engine_data("EngineV1")),
        )
        .series(
            Bar::new()
                .name("EngineV2")
                .data(get_engine_data("EngineV2")),
        )
        .series(
            Bar::new()
                .name("EngineV3")
                .data(get_engine_data("EngineV3")),
        )
        .series(
            Bar::new()
                .name("EngineV4")
                .data(get_engine_data("EngineV4")),
        )
        .series(
            Bar::new()
                .name("EngineV5")
                .data(get_engine_data("EngineV5")),
        );

    HtmlRenderer::new(chart_title, 1500, 600)
        .save(&chart, chart_file_name)
        .expect("Failed to save chart");
}

fn create_chart_level_scaling_throughput(
    throughput_kind: OrderThroughputKind,
    level_scaling_labels: &[String],
    throughput_rows: &[LevelScalingOrderThroughputRow],
) {
    let chart_file_name = match throughput_kind {
        OrderThroughputKind::PlaceOrders => THROUGHPUT_PLACE_ORDERS_LEVEL_SCALING_CHART_PATH,
        OrderThroughputKind::CancelOrders => THROUGHPUT_CANCEL_ORDERS_LEVEL_SCALING_CHART_PATH,
    };

    let chart_title = match throughput_kind {
        OrderThroughputKind::PlaceOrders => "Bench Place Orders Throughput",
        OrderThroughputKind::CancelOrders => "Bench Cancel Orders Throughput",
    };

    let get_engine_data = |engine_name: &str| -> Vec<f64> {
        level_scaling_labels
            .iter()
            .map(|target_label| {
                throughput_rows
                    .iter()
                    .find(|row| {
                        row.engine == engine_name
                            && format!(
                                "levels_{}/orders_{}",
                                row.total_levels, row.orders_per_level
                            ) == *target_label
                    })
                    .map_or(0.0, |row| row.m_orders_per_second)
            })
            .collect()
    };

    let chart = Chart::new()
        .title(Title::new().text(chart_title))
        .tooltip(Tooltip::new())
        .legend(Legend::new())
        .x_axis(
            Axis::new()
                .name("Engines")
                .type_(AxisType::Category)
                .data(level_scaling_labels.to_vec()),
        )
        .y_axis(
            Axis::new()
                .name("Million Orders/second")
                .type_(AxisType::Value),
        )
        .series(
            Bar::new()
                .name("EngineV1")
                .data(get_engine_data("EngineV1")),
        )
        .series(
            Bar::new()
                .name("EngineV2")
                .data(get_engine_data("EngineV2")),
        )
        .series(
            Bar::new()
                .name("EngineV3")
                .data(get_engine_data("EngineV3")),
        )
        .series(
            Bar::new()
                .name("EngineV4")
                .data(get_engine_data("EngineV4")),
        )
        .series(
            Bar::new()
                .name("EngineV5")
                .data(get_engine_data("EngineV5")),
        );

    HtmlRenderer::new(chart_title, 1500, 600)
        .save(&chart, chart_file_name)
        .expect("Failed to save chart");
}

fn create_chart_persistent_scaling_throughput(
    throughput_rows: &[PersistentScalingOrderThroughputRow],
    chart_file_name: &str,
) {
    let chart_title = "Bench Order Throughput Persistent Scaling";
    let map_row = |row: &PersistentScalingOrderThroughputRow| row.m_orders_per_second;

    let mut persistent_scaling_labels = throughput_rows
        .iter()
        .map(|row| row.batch)
        .collect::<Vec<usize>>();

    persistent_scaling_labels.sort_unstable();
    persistent_scaling_labels.dedup();

    let unique_labels = persistent_scaling_labels
        .iter()
        .map(|batch_idx| format!("{batch_idx}"))
        .collect::<Vec<String>>();

    let get_engine_data = |engine_name: &str| -> Vec<f64> {
        unique_labels
            .iter()
            .map(|target_label| {
                throughput_rows
                    .iter()
                    .find(|row| row.engine == engine_name && row.batch.to_string() == *target_label)
                    .map_or(0.0, map_row)
            })
            .collect()
    };

    let chart = Chart::new()
        .title(Title::new().text(chart_title))
        .tooltip(Tooltip::new())
        .legend(Legend::new())
        .x_axis(
            Axis::new()
                .name("Batches of 1000 orders")
                .type_(AxisType::Category)
                .data(unique_labels.clone()),
        )
        .y_axis(
            Axis::new()
                .name("Million Orders/second")
                .type_(AxisType::Log),
        )
        .series(
            Line::new()
                .name("EngineV1")
                .data(get_engine_data("EngineV1")),
        )
        .series(
            Line::new()
                .name("EngineV2")
                .data(get_engine_data("EngineV2")),
        )
        .series(
            Line::new()
                .name("EngineV3")
                .data(get_engine_data("EngineV3")),
        )
        .series(
            Line::new()
                .name("EngineV4")
                .data(get_engine_data("EngineV4")),
        )
        .series(
            Line::new()
                .name("EngineV5")
                .data(get_engine_data("EngineV5")),
        );

    HtmlRenderer::new(chart_title, 1500, 600)
        .save(&chart, chart_file_name)
        .expect("Failed to save chart");
}

fn create_criterion_result_charts() {
    let mut reader = Reader::from_path(CRITERION_RESULTS_CSV_PATH).expect("Failed to read file");

    let criterion_result_rows = reader
        .deserialize()
        .map(|row| row.unwrap())
        .collect::<Vec<CriterionResultRow>>();

    let command_types: HashSet<String> = criterion_result_rows
        .iter()
        .map(|row| row.command_type.clone())
        .collect();

    let order_strategies: HashSet<String> = criterion_result_rows
        .iter()
        .map(|row| row.order_strategy.clone())
        .collect();

    for cmd_type in &command_types {
        for strategy in &order_strategies {
            let chart_name = format!("{cmd_type} Orders/{strategy} Order");
            let chart_file_name = format!("{cmd_type}_orders_{strategy}_order");

            let filtered_rows: Vec<CriterionResultRow> = criterion_result_rows
                .iter()
                .filter(|row| row.command_type == *cmd_type && row.order_strategy == *strategy)
                .cloned()
                .collect();

            let mut x_axis_labels = Vec::new();
            for row in &filtered_rows {
                let label = format!("{} Levels/{} Orders", row.levels, row.orders_per_level);
                if !x_axis_labels.contains(&label) {
                    x_axis_labels.push(label);
                }
            }

            let get_engine_data = |engine_name: &str| -> Vec<f64> {
                x_axis_labels
                    .iter()
                    .map(|target_label| {
                        filtered_rows
                            .iter()
                            .find(|row| {
                                row.engine == engine_name
                                    && format!(
                                        "{} Levels/{} Orders",
                                        row.levels, row.orders_per_level
                                    ) == *target_label
                            })
                            .map_or(0.0, |row| row.m_orders_per_second)
                    })
                    .collect()
            };

            let chart = Chart::new()
                .title(Title::new().text(&chart_name))
                .tooltip(Tooltip::new())
                .legend(Legend::new())
                .x_axis(
                    Axis::new()
                        .name("Levels/Orders per Level")
                        .type_(AxisType::Category)
                        .data(x_axis_labels.clone()),
                )
                .y_axis(
                    Axis::new()
                        .name("Million Orders/second")
                        .type_(AxisType::Value),
                )
                .series(
                    Bar::new()
                        .name("EngineV1")
                        .data(get_engine_data("EngineV1")),
                )
                .series(
                    Bar::new()
                        .name("EngineV2")
                        .data(get_engine_data("EngineV2")),
                )
                .series(
                    Bar::new()
                        .name("EngineV3")
                        .data(get_engine_data("EngineV3")),
                )
                .series(
                    Bar::new()
                        .name("EngineV4")
                        .data(get_engine_data("EngineV4")),
                )
                .series(
                    Bar::new()
                        .name("EngineV5")
                        .data(get_engine_data("EngineV5")),
                );

            HtmlRenderer::new(chart_name.clone(), 1500, 600)
                .save(&chart, format!("benches/results/{chart_file_name}.html"))
                .expect("Failed to save chart");
        }
    }
}
