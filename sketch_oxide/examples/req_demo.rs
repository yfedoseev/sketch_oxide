use sketch_oxide::quantiles::req::{ReqMode, ReqSketch};

fn main() {
    // Demo: HRA mode - exact p100
    println!("=== REQ Sketch Demo: HRA Mode (p100 exact) ===");
    let mut hra = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

    // Add 10,000 values
    for i in 1..=10000 {
        hra.update(i as f64);
    }

    println!("Processed: {} items", hra.n());
    println!("Min: {}", hra.min().unwrap());
    println!("Max: {}", hra.max().unwrap());
    println!("p50: {:.2}", hra.quantile(0.5).unwrap());
    println!("p99: {:.2}", hra.quantile(0.99).unwrap());
    println!(
        "p100: {:.2} (EXACT - zero error)",
        hra.quantile(1.0).unwrap()
    );

    // Demo: LRA mode - exact p0
    println!("\n=== REQ Sketch Demo: LRA Mode (p0 exact) ===");
    let mut lra = ReqSketch::new(32, ReqMode::LowRankAccuracy).unwrap();

    // Add 10,000 values
    for i in 1..=10000 {
        lra.update(i as f64);
    }

    println!("Processed: {} items", lra.n());
    println!("p0: {:.2} (EXACT - zero error)", lra.quantile(0.0).unwrap());
    println!("p1: {:.2}", lra.quantile(0.01).unwrap());
    println!("p50: {:.2}", lra.quantile(0.5).unwrap());
    println!("Min: {}", lra.min().unwrap());
    println!("Max: {}", lra.max().unwrap());

    // Demo: Merge
    println!("\n=== REQ Sketch Demo: Merge ===");
    let mut sketch1 = ReqSketch::new(64, ReqMode::HighRankAccuracy).unwrap();
    let mut sketch2 = ReqSketch::new(64, ReqMode::HighRankAccuracy).unwrap();

    for i in 1..=500 {
        sketch1.update(i as f64);
    }
    for i in 501..=1000 {
        sketch2.update(i as f64);
    }

    let merged = sketch1.merge(&sketch2).unwrap();
    println!(
        "Sketch1: {} items, max = {}",
        sketch1.n(),
        sketch1.max().unwrap()
    );
    println!(
        "Sketch2: {} items, max = {}",
        sketch2.n(),
        sketch2.max().unwrap()
    );
    println!(
        "Merged: {} items, max = {} (EXACT)",
        merged.n(),
        merged.max().unwrap()
    );
    println!(
        "Merged p100: {} (EXACT - zero error)",
        merged.quantile(1.0).unwrap()
    );
}
