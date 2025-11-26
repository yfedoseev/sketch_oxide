// Debug false negative issue

fn hash_to_indices(item: u64, seed: u64, segment_length: u32) -> (u32, u32, u32) {
    use sketch_oxide::common::hash::xxhash;

    let mixed = item ^ seed;
    let item_bytes = mixed.to_le_bytes();
    let h = xxhash(&item_bytes, seed);

    let h0_raw = h.wrapping_mul(0x9E3779B97F4A7C15);
    let h1_raw = h.wrapping_mul(0xBF58476D1CE4E5B9);
    let h2_raw = h.wrapping_mul(0x94D049BB133111EB);

    let h0_mixed = (h0_raw ^ (h0_raw >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    let h1_mixed = (h1_raw ^ (h1_raw >> 27)).wrapping_mul(0x94D049BB133111EB);
    let h2_mixed = (h2_raw ^ (h2_raw >> 31)).wrapping_mul(0x9E3779B97F4A7C15);

    let segment_length_mask = segment_length - 1;

    let h0 = (h0_mixed as u32) & segment_length_mask;
    let h1 = segment_length + ((h1_mixed as u32) & segment_length_mask);
    let h2 = 2 * segment_length + ((h2_mixed as u32) & segment_length_mask);

    (h0, h1, h2)
}

fn get_fingerprint(item: u64, bits_per_entry: u8) -> u8 {
    let shift = 64 - bits_per_entry;
    let mask = ((1u16 << bits_per_entry) - 1) as u8;
    let mixed = item.wrapping_mul(0x9E3779B97F4A7C15);
    ((mixed >> shift) as u8) & mask
}

fn main() {
    let items = [
        4728917751571991075,
        2595269899220801773,
        1834073920345414025,
        8292935475340385966, // This one fails
        179459099947392842,
    ];

    let segment_length = 4; // For 5 items with 1.27x overhead
    let seed = 0;

    println!("Debugging item 3: {}", items[3]);
    println!("segment_length = {}", segment_length);
    println!("seed = {}", seed);

    let (h0, h1, h2) = hash_to_indices(items[3], seed, segment_length);
    let fp = get_fingerprint(items[3], 9);

    println!("\nItem 3 hashes to:");
    println!("  h0 = {}", h0);
    println!("  h1 = {}", h1);
    println!("  h2 = {}", h2);
    println!("  fingerprint = {}", fp);

    // Now check all items for collisions
    println!("\nAll items:");
    for (i, &item) in items.iter().enumerate() {
        let (h0, h1, h2) = hash_to_indices(item, seed, segment_length);
        let fp = get_fingerprint(item, 9);
        println!("[{}] h0={}, h1={}, h2={}, fp={}", i, h0, h1, h2, fp);
    }
}
