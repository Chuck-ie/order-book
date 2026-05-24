// //! Distribution visualizers for the synthetic order book benchmark.
// //!
// //! Each function takes the distribution parameters + mid_price and prints
// //! a textplots chart showing probability density over absolute price levels.
// //! X-axis = price, Y-axis = density (both bid and ask sides shown).
// //!
// //! Add to Cargo.toml:
// //!   textplots = "0.8"
//
// use textplots::{Chart, LabelBuilder, LabelFormat, Plot, Shape, TickDisplay, TickDisplayBuilder};
//
// const CHART_W: u32 = 120;
// const CHART_H: u32 = 40;
//
// // ── helpers ──────────────────────────────────────────────────────────────────
//
// fn exp_pdf(lambda: f32, x: f32) -> f32 {
//     if x < 0.0 {
//         0.0
//     } else {
//         lambda * (-lambda * x).exp()
//     }
// }
//
// fn lognormal_pdf(mu: f32, sigma: f32, x: f32) -> f32 {
//     if x <= 0.0 {
//         return 0.0;
//     }
//     let z = (x.ln() - mu) / sigma;
//     (1.0 / (x * sigma * (2.0 * std::f32::consts::PI).sqrt())) * (-0.5 * z * z).exp()
// }
//
// // ─────────────────────────────────────────────────────────────────────────────
// // 1. PASSIVE ORDER DISTANCE  —  Exp(lambda)
// //
// //    Plotted over price space: for bids, the peak is at (mid - half_spread)
// //    and density falls off leftward. For asks, mirrored rightward.
// //    X-axis spans [mid - 60, mid + 60] so both sides are visible.
// //
// //    Parameter intuition:
// //      lambda = rate = 1/mean_distance
// //      High lambda (e.g. 1.0) → mean 1 tick away, very peaked at the spread
// //      Low  lambda (e.g. 0.1) → mean 10 ticks away, flat/spread-out
// // ─────────────────────────────────────────────────────────────────────────────
// pub fn plot_passive_exp(lambda: f32, mid_price: f64, half_spread: i64) {
//     let mid = mid_price as f32;
//     let hs = half_spread as f32;
//     let best_bid = mid - hs; // highest resting bid price
//     let best_ask = mid + hs; // lowest resting ask price
//     let x_lo = mid - 65.0;
//     let x_hi = mid + 65.0;
//
//     println!(
//         "\n── Passive orders  Exp(λ={lambda:.2})  mean distance = {:.1} ticks ──",
//         1.0 / lambda
//     );
//     println!(
//         "   Bid side peaks at {best_bid} and decays left  │  Ask side peaks at {best_ask} and decays right"
//     );
//
//     // Bid density: distance from best_bid leftward = best_bid - price
//     // Ask density: distance from best_ask rightward = price - best_ask
//     Chart::new(CHART_W, CHART_H, x_lo, x_hi)
//         .lineplot(&Shape::Continuous(Box::new(move |price| {
//             let bid_dist = best_bid - price;
//             let ask_dist = price - best_ask;
//             // show whichever side the price falls on; zero in the spread gap
//             if price < best_bid {
//                 exp_pdf(lambda, bid_dist)
//             } else if price > best_ask {
//                 exp_pdf(lambda, ask_dist)
//             } else {
//                 0.0 // inside the spread — no passive orders here
//             }
//         })))
//         .x_label_format(LabelFormat::Value)
//         .y_tick_display(TickDisplay::None)
//         .display();
//     println!(
//         "   ↑ density    spread gap [{best_bid}, {best_ask}]    λ={lambda:.2}  (try 0.1 / 0.4 / 1.0)"
//     );
// }
//
// // ─────────────────────────────────────────────────────────────────────────────
// // 2. QUANTITY  —  LogNormal(mu, sigma)
// //
// //    X-axis is quantity (not price), from 1 to ~500.
// //    Shown independently of price because quantity is price-agnostic.
// //
// //    Parameter intuition:
// //      mu    controls the median: median = e^mu
// //      sigma controls the tail thickness: higher → more extreme outliers
// //      mu=3.4, sigma=0.9  →  median≈30, mean≈45, occasional orders >500
// // ─────────────────────────────────────────────────────────────────────────────
// pub fn plot_qty_lognormal(mu: f32, sigma: f32) {
//     let median = mu.exp();
//     let mean = (mu + 0.5 * sigma * sigma).exp();
//     let x_hi = (mu + 3.5 * sigma).exp().min(600.0); // show ~99.9th percentile
//
//     println!("\n── Quantity  LogNormal(μ={mu:.2}, σ={sigma:.2}) ──");
//     println!("   median={median:.0}  mean={mean:.0}  (x-axis = quantity, y = density)");
//
//     Chart::new(CHART_W, CHART_H, 1.0, x_hi)
//         .lineplot(&Shape::Continuous(Box::new(move |qty| {
//             lognormal_pdf(mu, sigma, qty)
//         })))
//         .x_label_format(LabelFormat::Value)
//         .y_tick_display(TickDisplay::None)
//         .display();
//     println!("   ↑ density    μ={mu:.2} σ={sigma:.2}  (try μ=3.4/σ=0.3 vs σ=0.9 vs σ=1.5)");
// }
//
// // ─────────────────────────────────────────────────────────────────────────────
// // 3. MARKETABLE ORDER AGGRESSION  —  Uniform(0, max_aggression)
// //
// //    Marketable bids are placed above best_ask; asks below best_bid.
// //    Density is flat (uniform) across the aggression range.
// //    X-axis is absolute price, showing both sides.
// //
// //    Parameter intuition:
// //      max_aggression = how many ticks past the opposite best price
// //      Uniform means every level in [0, max_aggression] is equally likely.
// //      Current: 0..4  →  bids land in [best_ask, best_ask+3]
// // ─────────────────────────────────────────────────────────────────────────────
// pub fn plot_marketable_uniform(max_aggression: u32, mid_price: f64, half_spread: i64) {
//     let mid = mid_price as f32;
//     let hs = half_spread as f32;
//     let best_bid = mid - hs;
//     let best_ask = mid + hs;
//     let agg = max_aggression as f32;
//     let density = if agg > 0.0 { 1.0 / agg } else { 1.0 };
//
//     // show a window wide enough to see context around the spread
//     let x_lo = mid - agg - 4.0;
//     let x_hi = mid + agg + 4.0;
//
//     println!("\n── Marketable orders  Uniform(0, {max_aggression}) ──");
//     println!(
//         "   Bid range: [{best_ask:.0}, {:.0}]  │  Ask range: [{:.0}, {best_bid:.0}]",
//         best_ask + agg - 1.0,
//         best_bid - agg + 1.0,
//     );
//
//     Chart::new(CHART_W, CHART_H, x_lo, x_hi)
//         .lineplot(&Shape::Continuous(Box::new(move |price| {
//             let in_bid_range = price >= best_ask && price < best_ask + agg;
//             let in_ask_range = price > best_bid - agg && price <= best_bid;
//             if in_bid_range || in_ask_range {
//                 density
//             } else {
//                 0.0
//             }
//         })))
//         .x_label_format(LabelFormat::Value)
//         .y_tick_display(TickDisplay::None)
//         .display();
//     println!(
//         "   ↑ density    mid={mid:.0}  spread=[{best_bid:.0},{best_ask:.0}]  aggression 0..{max_aggression}"
//     );
// }
//
// // ─────────────────────────────────────────────────────────────────────────────
// // 4. FAR ORDERS  —  Uniform(min_dist, max_dist)
// //
// //    Far orders rest well outside the spread. Density is flat over the range.
// //    X-axis is absolute price, wide enough to show both the spread and far zones.
// //
// //    Parameter intuition:
// //      min_dist / max_dist = tick distance from the spread edge
// //      Current: 20..80  →  a ~60-tick-wide band far from mid on each side
// //      Increasing min_dist pushes orders further from mid (more "iceberg"-like)
// //      Decreasing max_dist narrows the far zone
// // ─────────────────────────────────────────────────────────────────────────────
// pub fn plot_far_uniform(min_dist: u32, max_dist: u32, mid_price: f64, half_spread: i64) {
//     let mid = mid_price as f32;
//     let hs = half_spread as f32;
//     let best_bid = mid - hs;
//     let best_ask = mid + hs;
//     let lo = min_dist as f32;
//     let hi = max_dist as f32;
//     let density = if hi > lo { 1.0 / (hi - lo) } else { 1.0 };
//
//     let x_lo = mid - hi - 5.0;
//     let x_hi = mid + hi + 5.0;
//
//     println!("\n── Far orders  Uniform({min_dist}, {max_dist}) ──");
//     println!(
//         "   Bid zone: [{:.0}, {:.0}]  │  Ask zone: [{:.0}, {:.0}]",
//         best_bid - hi,
//         best_bid - lo,
//         best_ask + lo,
//         best_ask + hi,
//     );
//
//     Chart::new(CHART_W, CHART_H, x_lo, x_hi)
//         .lineplot(&Shape::Continuous(Box::new(move |price| {
//             let bid_dist = best_bid - price;
//             let ask_dist = price - best_ask;
//             if (bid_dist >= lo && bid_dist < hi) || (ask_dist >= lo && ask_dist < hi) {
//                 density
//             } else {
//                 0.0
//             }
//         })))
//         .x_label_format(LabelFormat::Value)
//         .y_tick_display(TickDisplay::None)
//         .display();
//     println!(
//         "   ↑ density    mid={mid:.0}  spread=[{best_bid:.0},{best_ask:.0}]  distance {min_dist}..{max_dist} ticks"
//     );
// }
//
// // ─────────────────────────────────────────────────────────────────────────────
// // 5. ORDER TYPE MIX  —  WeightedIndex([passive, marketable, far])
// //
// //    Shows all three price zones simultaneously at their weighted densities,
// //    so you can see how the type mix shapes the combined order flow over price.
// //    Each type's density contribution is scaled by its weight.
// // ─────────────────────────────────────────────────────────────────────────────
// pub fn plot_type_mix(
//     w_passive: f32,
//     w_marketable: f32,
//     w_far: f32,
//     lambda: f32,
//     max_aggression: u32,
//     far_min: u32,
//     far_max: u32,
//     mid_price: f64,
//     half_spread: i64,
// ) {
//     let total = w_passive + w_marketable + w_far;
//     let wp = w_passive / total;
//     let wm = w_marketable / total;
//     let wf = w_far / total;
//
//     let mid = mid_price as f32;
//     let hs = half_spread as f32;
//     let best_bid = mid - hs;
//     let best_ask = mid + hs;
//     let agg = max_aggression as f32;
//     let flo = far_min as f32;
//     let fhi = far_max as f32;
//     let mkt_density = if agg > 0.0 { 1.0 / agg } else { 1.0 };
//     let far_density = if fhi > flo { 1.0 / (fhi - flo) } else { 1.0 };
//
//     let x_lo = mid - fhi - 5.0;
//     let x_hi = mid + fhi + 5.0;
//
//     println!(
//         "\n── Type mix  passive={:.0}%  marketable={:.0}%  far={:.0}% ──",
//         wp * 100.0,
//         wm * 100.0,
//         wf * 100.0
//     );
//     println!("   Combined density over price (each type weighted by its probability)");
//
//     Chart::new(CHART_W, CHART_H, x_lo, x_hi)
//         .lineplot(&Shape::Continuous(Box::new(move |price| {
//             let bid_dist_passive = best_bid - price;
//             let ask_dist_passive = price - best_ask;
//             let bid_dist_far = best_bid - price;
//             let ask_dist_far = price - best_ask;
//
//             let passive = if price < best_bid {
//                 wp * exp_pdf(lambda, bid_dist_passive)
//             } else if price > best_ask {
//                 wp * exp_pdf(lambda, ask_dist_passive)
//             } else {
//                 0.0
//             };
//
//             let marketable = {
//                 let in_bid = price >= best_ask && price < best_ask + agg;
//                 let in_ask = price > best_bid - agg && price <= best_bid;
//                 if in_bid || in_ask {
//                     wm * mkt_density
//                 } else {
//                     0.0
//                 }
//             };
//
//             let far = {
//                 let in_bid = bid_dist_far >= flo && bid_dist_far < fhi;
//                 let in_ask = ask_dist_far >= flo && ask_dist_far < fhi;
//                 if in_bid || in_ask {
//                     wf * far_density
//                 } else {
//                     0.0
//                 }
//             };
//
//             passive + marketable + far
//         })))
//         .x_label_format(LabelFormat::Value)
//         .y_tick_display(TickDisplay::None)
//         .display();
//     println!(
//         "   ↑ density    mid={mid:.0}  note: passive peak is tall but narrow; far is short but wide"
//     );
// }
//
// // ─────────────────────────────────────────────────────────────────────────────
// // Demo main — call each visualizer with the benchmark's actual parameters,
// // then show a variant so the parameter effect is obvious.
// // ─────────────────────────────────────────────────────────────────────────────
// fn main() {
//     let mid: f64 = 10_000.0;
//     let half_spread: i64 = 1;
//
//     // --- passive: show effect of lambda ---
//     plot_passive_exp(0.4, mid, half_spread); // benchmark value
//     plot_passive_exp(1.0, mid, half_spread); // more clustered
//     plot_passive_exp(0.1, mid, half_spread); // more spread out
//
//     // --- quantity: show effect of sigma ---
//     plot_qty_lognormal(3.4, 0.9); // benchmark value
//     plot_qty_lognormal(3.4, 0.3); // tight sizes
//     plot_qty_lognormal(3.4, 1.5); // fat tail
//
//     // --- marketable: show effect of aggression window ---
//     plot_marketable_uniform(4, mid, half_spread); // benchmark value
//     plot_marketable_uniform(10, mid, half_spread); // wider crossing range
//
//     // --- far: show effect of distance range ---
//     plot_far_uniform(20, 80, mid, half_spread); // benchmark value
//     plot_far_uniform(5, 30, mid, half_spread); // closer to mid
//
//     // --- type mix: benchmark defaults, then skewed toward marketable ---
//     plot_type_mix(60.0, 30.0, 10.0, 0.4, 4, 20, 80, mid, half_spread);
//     plot_type_mix(20.0, 70.0, 10.0, 0.4, 4, 20, 80, mid, half_spread); // aggressive market
// }

//! Distribution visualizers for the synthetic order book benchmark.
//!
//! Each function takes the distribution parameters + mid_price and prints
//! a textplots chart showing probability density over absolute price levels.
//!
//! Add to Cargo.toml:
//!   textplots = "0.8"

use textplots::{Chart, LabelBuilder, LabelFormat, Plot, Shape, TickDisplay, TickDisplayBuilder};

const CHART_W: u32 = 120;
const CHART_H: u32 = 40;

// textplots filters out samples where y.is_normal() == false.
// In Rust, 0.0f32.is_normal() == false (zero is not "normal" in IEEE 754).
// So returning 0.0 from a Continuous closure means those points are excluded
// from the auto y-scale computation — if all your non-zero points are outside
// the visible x window, ymin==ymax==0 and nothing renders.
//
// Fix: return ZERO (below) instead of 0.0. It is visually identical but
// is_normal() returns true, so the rescaler builds a valid y range.
const ZERO: f32 = f32::MIN_POSITIVE;

// ── helpers ──────────────────────────────────────────────────────────────────

fn exp_pdf(lambda: f32, x: f32) -> f32 {
    if x < 0.0 {
        ZERO
    } else {
        lambda * (-lambda * x).exp()
    }
}

fn lognormal_pdf(mu: f32, sigma: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return ZERO;
    }
    let z = (x.ln() - mu) / sigma;
    (1.0 / (x * sigma * (2.0 * std::f32::consts::PI).sqrt())) * (-0.5 * z * z).exp()
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. PASSIVE ORDER DISTANCE  —  Exp(lambda)
//
//    Peak at best_bid (left) and best_ask (right), decays outward.
//    The spread gap [best_bid, best_ask] is empty.
//
//    lambda = 1 / mean_distance_in_ticks
//      lambda=1.0 → mean 1 tick, tight cluster at spread
//      lambda=0.4 → mean 2.5 ticks  (benchmark default)
//      lambda=0.1 → mean 10 ticks, broad spread
// ─────────────────────────────────────────────────────────────────────────────
pub fn plot_passive_exp(lambda: f32, mid_price: f64, half_spread: i64) {
    let mid = mid_price as f32;
    let hs = half_spread as f32;
    let best_bid = mid - hs;
    let best_ask = mid + hs;
    let window = (5.0 / lambda).max(15.0); // cover ~99% of mass
    let x_lo = best_bid - window;
    let x_hi = best_ask + window;

    println!(
        "\n── Passive orders  Exp(λ={lambda:.2})  mean = {:.1} ticks from spread ──",
        1.0 / lambda
    );
    println!("   Peaks at {best_bid:.0} (bid) and {best_ask:.0} (ask), decays away from spread");

    Chart::new(CHART_W, CHART_H, x_lo, x_hi)
        .lineplot(&Shape::Continuous(Box::new(move |price| {
            if price < best_bid {
                exp_pdf(lambda, best_bid - price)
            } else if price > best_ask {
                exp_pdf(lambda, price - best_ask)
            } else {
                ZERO
            }
        })))
        .x_label_format(LabelFormat::Value)
        .y_tick_display(TickDisplay::None)
        .display();
    println!("   ↑ density    spread=[{best_bid:.0},{best_ask:.0}]  λ={lambda:.2}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. QUANTITY  —  LogNormal(mu, sigma)
//
//    mode = e^(mu - sigma^2)   ← peak of the curve
//    median = e^mu
//    mean = e^(mu + sigma^2/2) ← pulled right by the tail
//
//    As sigma grows, mode shifts left and the right tail gets heavier.
// ─────────────────────────────────────────────────────────────────────────────
pub fn plot_qty_lognormal(mu: f32, sigma: f32) {
    let mode = (mu - sigma * sigma).exp();
    let median = mu.exp();
    let mean = (mu + 0.5 * sigma * sigma).exp();
    let x_lo = (mode * 0.3).max(0.5); // start before peak so rising edge shows
    let x_hi = (mu + 3.0 * sigma).exp().min(800.0);

    println!("\n── Quantity  LogNormal(μ={mu:.2}, σ={sigma:.2}) ──");
    println!("   mode={mode:.0}  median={median:.0}  mean={mean:.0}");

    Chart::new(CHART_W, CHART_H, x_lo, x_hi)
        .lineplot(&Shape::Continuous(Box::new(move |qty| {
            lognormal_pdf(mu, sigma, qty)
        })))
        .x_label_format(LabelFormat::Value)
        .y_tick_display(TickDisplay::None)
        .display();
    println!(
        "   ↑ density    μ={mu:.2} σ={sigma:.2}  (σ=0.3 tight / σ=0.9 default / σ=1.5 fat tail)"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. MARKETABLE ORDER AGGRESSION  —  Uniform(0, max_aggression)
//
//    Marketable bids cross upward past best_ask.
//    Marketable asks cross downward past best_bid.
//    Flat plateau of width max_aggression ticks on each side of the spread.
//
//    Uses new_with_y_range to pin the y-axis to the known density value,
//    bypassing the auto-scale (which breaks on all-ZERO samples).
// ─────────────────────────────────────────────────────────────────────────────
pub fn plot_marketable_uniform(max_aggression: u32, mid_price: f64, half_spread: i64) {
    let mid = mid_price as f32;
    let hs = half_spread as f32;
    let best_bid = mid - hs;
    let best_ask = mid + hs;
    let agg = max_aggression as f32;
    let density = if agg > 0.0 { 1.0 / agg } else { 1.0 };

    let x_lo = best_bid - agg - 4.0;
    let x_hi = best_ask + agg + 4.0;

    println!("\n── Marketable orders  Uniform(0, {max_aggression}) ──");
    println!(
        "   Bid zone (crosses ask): [{best_ask:.0}, {:.0})  │  Ask zone (crosses bid): ({:.0}, {best_bid:.0}]",
        best_ask + agg,
        best_bid - agg,
    );

    // Pin y range: 0 to density * 1.5 so the plateau has visual headroom
    Chart::new_with_y_range(CHART_W, CHART_H, x_lo, x_hi, 0.0, density * 1.5)
        .lineplot(&Shape::Continuous(Box::new(move |price| {
            let in_bid = price >= best_ask && price < best_ask + agg;
            let in_ask = price > best_bid - agg && price <= best_bid;
            if in_bid || in_ask { density } else { ZERO }
        })))
        .x_label_format(LabelFormat::Value)
        .y_tick_display(TickDisplay::None)
        .display();
    println!(
        "   ↑ density    mid={mid:.0}  spread=[{best_bid:.0},{best_ask:.0}]  aggression 0..{max_aggression}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. FAR ORDERS  —  Uniform(min_dist, max_dist)
//
//    Flat band [min_dist, max_dist) ticks beyond each spread edge.
//    The large gap between the spread and the far zone is empty.
//
//    Uses new_with_y_range for the same reason as marketable — the empty
//    gap zones between spread and far zone would fool the auto-scaler.
// ─────────────────────────────────────────────────────────────────────────────
pub fn plot_far_uniform(min_dist: u32, max_dist: u32, mid_price: f64, half_spread: i64) {
    let mid = mid_price as f32;
    let hs = half_spread as f32;
    let best_bid = mid - hs;
    let best_ask = mid + hs;
    let lo = min_dist as f32;
    let hi = max_dist as f32;
    let density = if hi > lo { 1.0 / (hi - lo) } else { 1.0 };

    // anchor to actual zone edges, not to mid
    let x_lo = best_bid - hi - 5.0;
    let x_hi = best_ask + hi + 5.0;

    println!("\n── Far orders  Uniform({min_dist}, {max_dist}) ──");
    println!(
        "   Bid zone: [{:.0}, {:.0})  │  Ask zone: [{:.0}, {:.0})",
        best_bid - hi,
        best_bid - lo,
        best_ask + lo,
        best_ask + hi,
    );

    Chart::new_with_y_range(CHART_W, CHART_H, x_lo, x_hi, 0.0, density * 1.5)
        .lineplot(&Shape::Continuous(Box::new(move |price| {
            let bid_dist = best_bid - price;
            let ask_dist = price - best_ask;
            let in_bid = bid_dist >= lo && bid_dist < hi;
            let in_ask = ask_dist >= lo && ask_dist < hi;
            if in_bid || in_ask { density } else { ZERO }
        })))
        .x_label_format(LabelFormat::Value)
        .y_tick_display(TickDisplay::None)
        .display();
    println!(
        "   ↑ density    mid={mid:.0}  spread=[{best_bid:.0},{best_ask:.0}]  distance {min_dist}..{max_dist} ticks"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. ORDER TYPE MIX  —  WeightedIndex([passive, marketable, far])
//
//    Overlays all three types weighted by their probabilities.
//    Passive peak dominates near the spread; far shows as low flat plateaus.
//    Uses auto-scale — the passive component ensures a valid ymax.
// ─────────────────────────────────────────────────────────────────────────────
pub fn plot_type_mix(
    w_passive: f32,
    w_marketable: f32,
    w_far: f32,
    lambda: f32,
    max_aggression: u32,
    far_min: u32,
    far_max: u32,
    mid_price: f64,
    half_spread: i64,
) {
    let total = w_passive + w_marketable + w_far;
    let wp = w_passive / total;
    let wm = w_marketable / total;
    let wf = w_far / total;

    let mid = mid_price as f32;
    let hs = half_spread as f32;
    let best_bid = mid - hs;
    let best_ask = mid + hs;
    let agg = max_aggression as f32;
    let flo = far_min as f32;
    let fhi = far_max as f32;
    let mkt_density = if agg > 0.0 { 1.0 / agg } else { 1.0 };
    let far_density = if fhi > flo { 1.0 / (fhi - flo) } else { 1.0 };

    let x_lo = best_bid - fhi - 5.0;
    let x_hi = best_ask + fhi + 5.0;

    println!(
        "\n── Type mix  passive={:.0}%  marketable={:.0}%  far={:.0}% ──",
        wp * 100.0,
        wm * 100.0,
        wf * 100.0
    );
    println!("   Combined weighted density over absolute price");

    Chart::new(CHART_W, CHART_H, x_lo, x_hi)
        .lineplot(&Shape::Continuous(Box::new(move |price| {
            let passive = if price < best_bid {
                wp * exp_pdf(lambda, best_bid - price)
            } else if price > best_ask {
                wp * exp_pdf(lambda, price - best_ask)
            } else {
                ZERO
            };

            let marketable = {
                let in_bid = price >= best_ask && price < best_ask + agg;
                let in_ask = price > best_bid - agg && price <= best_bid;
                if in_bid || in_ask {
                    wm * mkt_density
                } else {
                    ZERO
                }
            };

            let far = {
                let bid_dist = best_bid - price;
                let ask_dist = price - best_ask;
                let in_bid = bid_dist >= flo && bid_dist < fhi;
                let in_ask = ask_dist >= flo && ask_dist < fhi;
                if in_bid || in_ask {
                    wf * far_density
                } else {
                    ZERO
                }
            };

            // sum — at least one component is always non-ZERO so auto-scale works
            passive + marketable + far
        })))
        .x_label_format(LabelFormat::Value)
        .y_tick_display(TickDisplay::None)
        .display();
    println!(
        "   ↑ density    mid={mid:.0}  passive=tall narrow peaks  far=short wide plateaus at edges"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Demo
// ─────────────────────────────────────────────────────────────────────────────
fn main() {
    let mid: f64 = 10_000.0;
    let hs: i64 = 1;

    plot_passive_exp(0.4, mid, hs);
    plot_passive_exp(1.0, mid, hs);
    plot_passive_exp(0.1, mid, hs);

    plot_qty_lognormal(3.4, 0.9);
    plot_qty_lognormal(3.4, 0.3);
    plot_qty_lognormal(3.4, 1.5);

    plot_marketable_uniform(4, mid, hs);
    plot_marketable_uniform(10, mid, hs);

    plot_far_uniform(20, 80, mid, hs);
    plot_far_uniform(5, 30, mid, hs);

    plot_type_mix(60.0, 30.0, 10.0, 0.4, 4, 20, 80, mid, hs);
    plot_type_mix(20.0, 70.0, 10.0, 0.4, 4, 20, 80, mid, hs);
}
