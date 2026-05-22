use std::{any::type_name, time::Instant};

use serde::Deserialize;
use shared::{
    LimitOrderRequest, MatcherCommand, OrderMatcherExt, ob_naive, ob_slot_map_naive,
    ob_slot_map_optimized, ob_slot_map_unsafe, ob_standard,
};

#[derive(Debug, Deserialize)]
struct CsvOrder {
    pub time: f32,
    pub order_type: i8,
    pub id: u32,
    pub size: u64,
    pub price: u64,
    pub side: i8,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_benchmark::<ob_naive::OrderMatcher>()?;
    run_benchmark::<ob_standard::OrderMatcher>()?;
    run_benchmark::<ob_slot_map_naive::OrderMatcher>()?;
    run_benchmark::<ob_slot_map_optimized::OrderMatcher>()?;
    run_benchmark::<ob_slot_map_unsafe::OrderMatcher>()?;

    Ok(())
}

fn run_benchmark<M>() -> Result<(), Box<dyn std::error::Error>>
where
    M: OrderMatcherExt,
{
    let full_name = type_name::<M>();
    // let readable_name = full_name.split("::").nth_back(1).unwrap_or(full_name);

    println!("\nBenchmarking: {full_name}");

    let tickers = vec!["AAPL", "AMZN", "GOOG", "INTC", "MSFT"];
    let mut total_commands = 0;
    let mut total_duration_ns = 0u128;

    println!(
        "{:<10} | {:<12} | {:<18} | {:<16}",
        "Ticker", "Commands", "Total Time (ns)", "Avg/Op (ns)"
    );
    println!("{:-<65}", "");

    for ticker in &tickers {
        let path = format!("benchmarks/data/{ticker}.csv");

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(&path)?;

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
        if order_command_count == 0 {
            println!("{ticker:<10} | No commands found in {path}");
            continue;
        }

        let mut matcher = M::new();

        let start = Instant::now();
        for cmd in order_commands {
            matcher.process(cmd);
        }
        let duration = start.elapsed().as_nanos();

        let avg_op = duration / order_command_count as u128;

        println!("{ticker:<10} | {order_command_count:<12} | {duration:<18} | {avg_op:<16}");

        total_commands += order_command_count;
        total_duration_ns += duration;
    }

    if total_commands > 0 {
        println!("{:-<65}", "");
        println!(
            "{:<10} | {:<12} | {:<18} | {:<16}",
            "TOTAL",
            total_commands,
            total_duration_ns,
            total_duration_ns / total_commands as u128
        );
    }

    Ok(())
}
