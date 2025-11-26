use sketch_oxide::membership::BinaryFuseFilter;

fn main() {
    // This is the failing case from proptest
    let items = [
        4728917751571991075,
        2595269899220801773,
        1834073920345414025,
        8292935475340385966,
        179459099947392842,
    ];

    println!("Testing with {} items", items.len());

    match BinaryFuseFilter::from_items(items.iter().copied(), 9) {
        Ok(filter) => {
            println!("✓ Filter constructed");
            println!("  Size: {}", filter.len());

            println!("\nChecking each item:");
            for (i, item) in items.iter().enumerate() {
                let found = filter.contains(item);
                if found {
                    println!("  [{}] {}: ✓", i, item);
                } else {
                    println!("  [{}] {}: ✗ FALSE NEGATIVE!", i, item);
                }
            }
        }
        Err(e) => {
            println!("✗ Construction failed: {:?}", e);
        }
    }
}
