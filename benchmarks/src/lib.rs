use std::{collections::HashSet, time::Instant};

use serde::Deserialize;
use shared::{
    LimitOrderRequest, MatcherCommand, OrderMatcherExt, ob_slot_map_unsafe::OrderMatcher,
};

#[derive(Debug, Deserialize)]
struct CsvOrder {
    pub time: f32,
    pub order_type: i8,
    pub id: u32,
    pub size: u32,
    pub price: u32,
    pub side: i8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path("benchmarks/data/AMZN.csv")?;

    let mut order_commands = vec![];

    for result in rdr.deserialize() {
        let record: CsvOrder = result?;

        if record.order_type == 1 {
            order_commands.push(MatcherCommand::PlaceOrder(LimitOrderRequest {
                side: record.side.into(),
                amount: record.size,
                limit: record.price,
            }));
        }
    }

    let order_command_count = order_commands.len();
    let mut matcher = OrderMatcher::new();

    // Version 1 (without warmup):
    let start = Instant::now();

    for cmd in order_commands {
        matcher.process(cmd);
    }

    let duration = start.elapsed().as_nanos();

    println!("Processed {order_command_count} commands in: {duration}");

    println!(
        "Average time per op: {:?}",
        duration / order_command_count as u128
    );

    Ok(())
}
