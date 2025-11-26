//! MinHash Demo: Document Similarity Detection
//!
//! This example demonstrates using MinHash for document similarity detection,
//! a common real-world use case for near-duplicate detection and clustering.

use sketch_oxide::similarity::MinHash;
use sketch_oxide::Mergeable;

fn main() {
    println!("=== MinHash Document Similarity Demo ===\n");

    // Create MinHash sketches with 128 hash functions
    let mut doc1 = MinHash::new(128).expect("Failed to create MinHash");
    let mut doc2 = MinHash::new(128).expect("Failed to create MinHash");
    let mut doc3 = MinHash::new(128).expect("Failed to create MinHash");

    // Sample documents (in practice, these would be much longer)
    let text1 = "the quick brown fox jumps over the lazy dog";
    let text2 = "the quick brown fox leaps over the sleepy cat";
    let text3 = "machine learning algorithms process data efficiently";

    println!("Document 1: \"{}\"", text1);
    println!("Document 2: \"{}\"", text2);
    println!("Document 3: \"{}\"\n", text3);

    // Generate character trigrams (3-grams) for each document
    // This is a simple form of document shingling
    println!("Generating trigrams...");

    for window in text1.as_bytes().windows(3) {
        doc1.update(&window);
    }
    println!("  Doc1: {} trigrams", text1.len() - 2);

    for window in text2.as_bytes().windows(3) {
        doc2.update(&window);
    }
    println!("  Doc2: {} trigrams", text2.len() - 2);

    for window in text3.as_bytes().windows(3) {
        doc3.update(&window);
    }
    println!("  Doc3: {} trigrams\n", text3.len() - 2);

    // Compute pairwise Jaccard similarities
    println!("Computing Jaccard similarities:");

    let sim_12 = doc1
        .jaccard_similarity(&doc2)
        .expect("Failed to compute similarity");
    let sim_13 = doc1
        .jaccard_similarity(&doc3)
        .expect("Failed to compute similarity");
    let sim_23 = doc2
        .jaccard_similarity(&doc3)
        .expect("Failed to compute similarity");

    println!("  Doc1 vs Doc2: {:.3} (similar topics)", sim_12);
    println!("  Doc1 vs Doc3: {:.3} (different topics)", sim_13);
    println!("  Doc2 vs Doc3: {:.3} (different topics)\n", sim_23);

    // Demonstrate merging for union
    println!("=== Merging Sketches ===\n");

    let mut union_12 = doc1.clone();
    union_12.merge(&doc2).expect("Failed to merge sketches");

    println!("Created union of Doc1 and Doc2");
    println!("Comparing union with Doc3:");

    let sim_union_3 = union_12
        .jaccard_similarity(&doc3)
        .expect("Failed to compute similarity");
    println!("  Union(Doc1,Doc2) vs Doc3: {:.3}\n", sim_union_3);

    // Demonstrate set operations
    println!("=== Set-Based Similarity ===\n");

    let mut set_a = MinHash::new(128).expect("Failed to create MinHash");
    let mut set_b = MinHash::new(128).expect("Failed to create MinHash");

    // Set A: {0, 1, 2, ..., 99}
    for i in 0..100 {
        set_a.update(&i);
    }

    // Set B: {50, 51, 52, ..., 149}
    for i in 50..150 {
        set_b.update(&i);
    }

    let jaccard = set_a
        .jaccard_similarity(&set_b)
        .expect("Failed to compute similarity");

    println!("Set A: {{0..100}} (100 elements)");
    println!("Set B: {{50..150}} (100 elements)");
    println!("Intersection: {{50..100}} (50 elements)");
    println!("Union: {{0..150}} (150 elements)");
    println!("True Jaccard: 50/150 = {:.3}", 50.0 / 150.0);
    println!("Estimated Jaccard: {:.3}", jaccard);
    println!("Error: {:.3}%\n", ((jaccard - 0.333).abs() / 0.333 * 100.0));

    // Demonstrate accuracy vs num_perm trade-off
    println!("=== Accuracy vs. Memory Trade-off ===\n");

    for &k in &[32, 64, 128, 256] {
        let mut mh_a = MinHash::new(k).expect("Failed to create MinHash");
        let mut mh_b = MinHash::new(k).expect("Failed to create MinHash");

        for i in 0..100 {
            mh_a.update(&i);
        }
        for i in 50..150 {
            mh_b.update(&i);
        }

        let estimated = mh_a
            .jaccard_similarity(&mh_b)
            .expect("Failed to compute similarity");
        let error = (estimated - 0.333).abs() / 0.333 * 100.0;
        let memory = k * 8 * 2; // k hash values + k seeds, each 8 bytes

        println!(
            "  k={:3} → error: {:5.2}%, memory: {:4} bytes",
            k, error, memory
        );
    }

    println!("\n=== Performance Characteristics ===\n");
    println!("Time Complexity:");
    println!("  - Update:     O(k) where k = num_perm");
    println!("  - Similarity: O(k) comparisons");
    println!("  - Merge:      O(k) min operations");
    println!("\nSpace Complexity:");
    println!("  - O(k) for k hash values");
    println!("\nAccuracy:");
    println!("  - Standard error ≈ 1/√k");
    println!("  - k=128 → ~8.8% error");
    println!("  - k=256 → ~6.25% error");

    println!("\n=== MinHash Use Cases ===\n");
    println!("1. Near-duplicate detection (documents, web pages)");
    println!("2. Clustering similar items");
    println!("3. Recommendation systems (finding similar users/items)");
    println!("4. Plagiarism detection");
    println!("5. LSH (Locality-Sensitive Hashing) for fast nearest neighbor search");
}
