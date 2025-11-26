use sketch_oxide::membership::BinaryFuseFilter;

fn main() {
    let items = vec![1u64, 2, 3, 4, 5, 100, 200, 500];

    println!(
        "Attempting to construct Binary Fuse Filter with {} items",
        items.len()
    );

    match BinaryFuseFilter::from_items(items.iter().copied(), 9) {
        Ok(filter) => {
            println!("✓ Success! Filter constructed");
            println!("  Size: {}", filter.len());
            println!("  Bits per entry: {:.2}", filter.bits_per_entry());

            println!("\nTesting membership:");
            for item in &items {
                let found = filter.contains(item);
                println!(
                    "  Item {}: {}",
                    item,
                    if found { "✓" } else { "✗ FALSE NEGATIVE!" }
                );
            }
        }
        Err(e) => {
            println!("✗ Construction failed: {:?}", e);
        }
    }
}
