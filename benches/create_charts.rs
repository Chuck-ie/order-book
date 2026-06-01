use charming::{
    Chart, HtmlRenderer,
    component::{Axis, Legend, Title},
    element::{AxisType, Tooltip},
    series::Bar,
};
use csv::Reader;

use crate::shared::{
    LEVEL_SCALINGS, MEMORY_FOOTPRINT_PLACE_ORDERS_CSV_PATH, smem_prof::SMemProfRow,
};

mod shared;

const MEMORY_FOOTPRINT_ALLOC_CHART_PATH: &str = "benches/results/memory_footprint_alloc.html";
const MEMORY_FOOTPRINT_GROW_CHART_PATH: &str = "benches/results/memory_footprint_grow.html";

fn main() {
    let mut reader = Reader::from_path(MEMORY_FOOTPRINT_PLACE_ORDERS_CSV_PATH).unwrap();
    let mut smem_prof_rows = vec![];

    for row in reader.deserialize() {
        let row: SMemProfRow = row.unwrap();
        smem_prof_rows.push(row);
    }

    let level_scaling_labels = LEVEL_SCALINGS
        .iter()
        .map(|(levels, orders)| format!("levels_{levels}/orders_{orders}"))
        .collect::<Vec<String>>();

    create_chart_memory_profiles(
        ChartKind::Alloc,
        level_scaling_labels.clone(),
        &smem_prof_rows,
    );

    create_chart_memory_profiles(
        ChartKind::Grow,
        level_scaling_labels.clone(),
        &smem_prof_rows,
    );
}

#[derive(Clone, Copy)]
enum ChartKind {
    Alloc,
    Grow,
}

#[allow(clippy::cast_precision_loss)]
fn create_chart_memory_profiles(
    chart_kind: ChartKind,
    level_scaling_labels: Vec<String>,
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

    let chart = Chart::new()
        .title(Title::new().text(chart_title))
        .tooltip(Tooltip::new())
        .legend(Legend::new())
        .x_axis(Axis::new().name(x_axis_name).type_(AxisType::Value))
        .y_axis(
            Axis::new()
                .name("Engines")
                .type_(AxisType::Category)
                .data(level_scaling_labels),
        )
        .series(
            Bar::new().name("EngineV1").data(
                smem_prof_rows
                    .iter()
                    .filter(|row| row.engine == "EngineV1")
                    .map(map_row)
                    .collect(),
            ),
        )
        .series(
            Bar::new().name("EngineV2").data(
                smem_prof_rows
                    .iter()
                    .filter(|row| row.engine == "EngineV2")
                    .map(map_row)
                    .collect(),
            ),
        )
        .series(
            Bar::new().name("EngineV3").data(
                smem_prof_rows
                    .iter()
                    .filter(|row| row.engine == "EngineV3")
                    .map(map_row)
                    .collect(),
            ),
        )
        .series(
            Bar::new().name("EngineV4").data(
                smem_prof_rows
                    .iter()
                    .filter(|row| row.engine == "EngineV4")
                    .map(map_row)
                    .collect(),
            ),
        );

    HtmlRenderer::new(chart_title, 1000, 600)
        .save(&chart, chart_file_name)
        .expect("Failed to save chart");
}
