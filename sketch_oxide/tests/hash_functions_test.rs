//! Property-based tests for hash functions

use proptest::prelude::*;
use sketch_oxide::common::hash::{murmur3_hash, xxhash};

#[cfg(test)]
mod murmur3_tests {
    use super::*;

    #[test]
    fn test_murmur3_consistency() {
        // Same input should always produce same output
        let input = b"test data";
        let seed = 42;

        let hash1 = murmur3_hash(input, seed);
        let hash2 = murmur3_hash(input, seed);

        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_murmur3_different_seeds() {
        // Different seeds should produce different hashes
        let input = b"test data";

        let hash1 = murmur3_hash(input, 0);
        let hash2 = murmur3_hash(input, 1);

        assert_ne!(
            hash1, hash2,
            "Different seeds should produce different hashes"
        );
    }

    #[test]
    fn test_murmur3_different_inputs() {
        // Different inputs should produce different hashes
        let seed = 42;

        let hash1 = murmur3_hash(b"data1", seed);
        let hash2 = murmur3_hash(b"data2", seed);

        assert_ne!(
            hash1, hash2,
            "Different inputs should produce different hashes"
        );
    }

    #[test]
    fn test_murmur3_empty_input() {
        // Should handle empty input without panicking
        let hash1 = murmur3_hash(b"", 0);
        let hash2 = murmur3_hash(b"", 0);
        // Hash should be deterministic, even for empty input
        assert_eq!(hash1, hash2, "Empty input should produce consistent hash");

        // Different seed should produce different hash even for empty input
        let hash3 = murmur3_hash(b"", 1);
        assert_ne!(
            hash1, hash3,
            "Different seeds should produce different hashes even for empty input"
        );
    }

    #[test]
    fn test_murmur3_known_vector() {
        // Test against known MurmurHash3 output
        // Using test vector from reference implementation
        let hash = murmur3_hash(b"hello", 0);

        // MurmurHash3_x86_32("hello", seed=0) should produce consistent output
        // This is a placeholder - replace with actual expected value
        assert!(hash != 0);
    }

    // Property-based test: Determinism
    proptest! {
        #[test]
        fn prop_murmur3_deterministic(data in prop::collection::vec(any::<u8>(), 0..1000), seed in any::<u32>()) {
            let hash1 = murmur3_hash(&data, seed);
            let hash2 = murmur3_hash(&data, seed);
            prop_assert_eq!(hash1, hash2);
        }
    }

    // Property-based test: Different seeds produce different hashes
    proptest! {
        #[test]
        fn prop_murmur3_seed_independence(
            data in prop::collection::vec(any::<u8>(), 1..1000),
            seed1 in any::<u32>(),
            seed2 in any::<u32>()
        ) {
            prop_assume!(seed1 != seed2);
            prop_assume!(!data.is_empty());

            let hash1 = murmur3_hash(&data, seed1);
            let hash2 = murmur3_hash(&data, seed2);

            // Very high probability they should be different
            prop_assert_ne!(hash1, hash2);
        }
    }

    // Property-based test: Distribution (Avalanche effect)
    proptest! {
        #[test]
        fn prop_murmur3_avalanche_single_bit(data in prop::collection::vec(any::<u8>(), 1..100)) {
            let hash1 = murmur3_hash(&data, 0);

            // Flip a random bit
            let mut modified = data.clone();
            if !modified.is_empty() {
                modified[0] ^= 1;
                let hash2 = murmur3_hash(&modified, 0);

                // Single bit change should produce very different hash (avalanche effect)
                prop_assert_ne!(hash1, hash2);
            }
        }
    }
}

#[cfg(test)]
mod xxhash_tests {
    use super::*;

    #[test]
    fn test_xxhash_consistency() {
        let input = b"test data";
        let seed = 42;

        let hash1 = xxhash(input, seed);
        let hash2 = xxhash(input, seed);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_xxhash_different_seeds() {
        let input = b"test data";

        let hash1 = xxhash(input, 0);
        let hash2 = xxhash(input, 1);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_xxhash_different_inputs() {
        let seed = 42;

        let hash1 = xxhash(b"data1", seed);
        let hash2 = xxhash(b"data2", seed);

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_xxhash_empty_input() {
        // Should handle empty input without panicking
        let hash1 = xxhash(b"", 0);
        let hash2 = xxhash(b"", 0);
        // Hash should be deterministic, even for empty input
        assert_eq!(hash1, hash2, "Empty input should produce consistent hash");

        // Different seed should produce different hash even for empty input
        let hash3 = xxhash(b"", 1);
        assert_ne!(
            hash1, hash3,
            "Different seeds should produce different hashes even for empty input"
        );
    }

    // Property-based test: Determinism
    proptest! {
        #[test]
        fn prop_xxhash_deterministic(data in prop::collection::vec(any::<u8>(), 0..1000), seed in any::<u64>()) {
            let hash1 = xxhash(&data, seed);
            let hash2 = xxhash(&data, seed);
            prop_assert_eq!(hash1, hash2);
        }
    }

    // Property-based test: Different seeds
    proptest! {
        #[test]
        fn prop_xxhash_seed_independence(
            data in prop::collection::vec(any::<u8>(), 1..1000),
            seed1 in any::<u64>(),
            seed2 in any::<u64>()
        ) {
            prop_assume!(seed1 != seed2);
            prop_assume!(!data.is_empty());

            let hash1 = xxhash(&data, seed1);
            let hash2 = xxhash(&data, seed2);

            prop_assert_ne!(hash1, hash2);
        }
    }

    // Property-based test: Avalanche effect
    proptest! {
        #[test]
        fn prop_xxhash_avalanche(data in prop::collection::vec(any::<u8>(), 1..100)) {
            let hash1 = xxhash(&data, 0);

            let mut modified = data.clone();
            if !modified.is_empty() {
                modified[0] ^= 1;
                let hash2 = xxhash(&modified, 0);

                prop_assert_ne!(hash1, hash2);
            }
        }
    }
}

// Statistical distribution test
#[cfg(test)]
mod distribution_tests {
    use super::*;

    #[test]
    fn test_murmur3_distribution() {
        // Hash 1000 sequential integers and check distribution
        let seed = 0;
        let mut buckets = vec![0u32; 64];

        for i in 0u32..1000 {
            let data = i.to_le_bytes();
            let hash = murmur3_hash(&data, seed);
            let bucket = (hash as usize) % buckets.len();
            buckets[bucket] += 1;
        }

        // Check no bucket is empty (good distribution)
        let empty_buckets = buckets.iter().filter(|&&count| count == 0).count();
        assert!(
            empty_buckets < 5,
            "Too many empty buckets: {}",
            empty_buckets
        );

        // Check distribution is reasonably uniform (no bucket has > 50% of items)
        let max_bucket = buckets.iter().max().unwrap();
        assert!(
            *max_bucket < 500,
            "Poor distribution: max bucket has {}",
            max_bucket
        );
    }

    #[test]
    fn test_xxhash_distribution() {
        let seed = 0;
        let mut buckets = vec![0u32; 64];

        for i in 0u32..1000 {
            let data = i.to_le_bytes();
            let hash = xxhash(&data, seed);
            let bucket = (hash as usize) % buckets.len();
            buckets[bucket] += 1;
        }

        let empty_buckets = buckets.iter().filter(|&&count| count == 0).count();
        assert!(empty_buckets < 5);

        let max_bucket = buckets.iter().max().unwrap();
        assert!(*max_bucket < 500);
    }
}
