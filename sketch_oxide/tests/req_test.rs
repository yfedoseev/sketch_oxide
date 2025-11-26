use sketch_oxide::quantiles::req::{ReqMode, ReqSketch};

#[cfg(test)]
mod basic_tests {
    use super::*;

    #[test]
    fn test_new_valid_k() {
        let sketch = ReqSketch::new(4, ReqMode::HighRankAccuracy);
        assert!(sketch.is_ok());

        let sketch = ReqSketch::new(128, ReqMode::LowRankAccuracy);
        assert!(sketch.is_ok());

        let sketch = ReqSketch::new(1024, ReqMode::HighRankAccuracy);
        assert!(sketch.is_ok());
    }

    #[test]
    fn test_new_invalid_k() {
        let sketch = ReqSketch::new(3, ReqMode::HighRankAccuracy);
        assert!(sketch.is_err());

        let sketch = ReqSketch::new(1025, ReqMode::HighRankAccuracy);
        assert!(sketch.is_err());

        let sketch = ReqSketch::new(0, ReqMode::HighRankAccuracy);
        assert!(sketch.is_err());
    }

    #[test]
    fn test_empty_sketch() {
        let sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        assert_eq!(sketch.n(), 0);
        assert!(sketch.is_empty());
        assert!(sketch.quantile(0.5).is_none());
        assert!(sketch.min().is_none());
        assert!(sketch.max().is_none());
    }

    #[test]
    fn test_single_value() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        sketch.update(42.0);

        assert_eq!(sketch.n(), 1);
        assert!(!sketch.is_empty());
        assert_eq!(sketch.min(), Some(42.0));
        assert_eq!(sketch.max(), Some(42.0));
        assert_eq!(sketch.quantile(0.0), Some(42.0));
        assert_eq!(sketch.quantile(0.5), Some(42.0));
        assert_eq!(sketch.quantile(1.0), Some(42.0));
    }

    #[test]
    fn test_update_multiple_values() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        for i in 1..=100 {
            sketch.update(i as f64);
        }

        assert_eq!(sketch.n(), 100);
        assert_eq!(sketch.min(), Some(1.0));
        assert_eq!(sketch.max(), Some(100.0));
    }
}

#[cfg(test)]
mod hra_mode_tests {
    use super::*;

    /// ⭐ CRITICAL: HRA mode MUST have zero error at p100
    #[test]
    fn test_hra_p100_exact() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        // Add 1000 values
        for i in 1..=1000 {
            sketch.update(i as f64);
        }

        // p100 MUST equal the maximum value exactly (zero error)
        assert_eq!(sketch.quantile(1.0), Some(1000.0));
        assert_eq!(sketch.max(), Some(1000.0));
    }

    #[test]
    fn test_hra_p100_exact_with_duplicates() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        // Add values with duplicates at the max
        for i in 1..=100 {
            sketch.update(i as f64);
        }
        for _ in 0..50 {
            sketch.update(100.0); // Duplicate max values
        }

        // p100 MUST still be exact
        assert_eq!(sketch.quantile(1.0), Some(100.0));
        assert_eq!(sketch.max(), Some(100.0));
    }

    #[test]
    fn test_hra_p100_exact_random_order() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        // Add in random order but max=500
        let values = vec![50.0, 500.0, 25.0, 300.0, 100.0, 450.0, 1.0];
        for v in values {
            sketch.update(v);
        }

        assert_eq!(sketch.quantile(1.0), Some(500.0));
        assert_eq!(sketch.max(), Some(500.0));
    }

    #[test]
    fn test_hra_high_quantiles_accuracy() {
        let mut sketch = ReqSketch::new(128, ReqMode::HighRankAccuracy).unwrap();

        // Add 10000 values
        for i in 1..=10000 {
            sketch.update(i as f64);
        }

        // p99 should be close to 9900 (within relative error)
        let p99 = sketch.quantile(0.99).unwrap();
        assert!((9800.0..=10000.0).contains(&p99));

        // p99.9 should be close to 9990
        let p999 = sketch.quantile(0.999).unwrap();
        assert!((9900.0..=10000.0).contains(&p999));

        // p100 MUST be exact
        assert_eq!(sketch.quantile(1.0), Some(10000.0));
    }

    #[test]
    fn test_hra_tail_quantiles_after_many_compactions() {
        let mut sketch = ReqSketch::new(16, ReqMode::HighRankAccuracy).unwrap();

        // Small k forces many compactions
        for i in 1..=100000 {
            sketch.update(i as f64);
        }

        // Even after many compactions, p100 MUST be exact
        assert_eq!(sketch.quantile(1.0), Some(100000.0));
        assert_eq!(sketch.max(), Some(100000.0));

        // High quantiles should still be accurate
        let p99 = sketch.quantile(0.99).unwrap();
        assert!(p99 >= 98000.0); // Should be reasonably close
    }
}

#[cfg(test)]
mod lra_mode_tests {
    use super::*;

    /// ⭐ CRITICAL: LRA mode MUST have zero error at p0
    #[test]
    fn test_lra_p0_exact() {
        let mut sketch = ReqSketch::new(32, ReqMode::LowRankAccuracy).unwrap();

        // Add 1000 values
        for i in 1..=1000 {
            sketch.update(i as f64);
        }

        // p0 MUST equal the minimum value exactly (zero error)
        assert_eq!(sketch.quantile(0.0), Some(1.0));
        assert_eq!(sketch.min(), Some(1.0));
    }

    #[test]
    fn test_lra_p0_exact_with_duplicates() {
        let mut sketch = ReqSketch::new(32, ReqMode::LowRankAccuracy).unwrap();

        // Add values with duplicates at the min
        for _ in 0..50 {
            sketch.update(1.0); // Duplicate min values
        }
        for i in 2..=100 {
            sketch.update(i as f64);
        }

        // p0 MUST still be exact
        assert_eq!(sketch.quantile(0.0), Some(1.0));
        assert_eq!(sketch.min(), Some(1.0));
    }

    #[test]
    fn test_lra_p0_exact_random_order() {
        let mut sketch = ReqSketch::new(32, ReqMode::LowRankAccuracy).unwrap();

        // Add in random order but min=0.5
        let values = vec![50.0, 100.0, 25.0, 0.5, 300.0, 5.0, 200.0];
        for v in values {
            sketch.update(v);
        }

        assert_eq!(sketch.quantile(0.0), Some(0.5));
        assert_eq!(sketch.min(), Some(0.5));
    }

    #[test]
    fn test_lra_low_quantiles_accuracy() {
        let mut sketch = ReqSketch::new(128, ReqMode::LowRankAccuracy).unwrap();

        // Add 10000 values
        for i in 1..=10000 {
            sketch.update(i as f64);
        }

        // p1 should be close to 100 (within relative error)
        let p1 = sketch.quantile(0.01).unwrap();
        assert!((1.0..=200.0).contains(&p1));

        // p0.1 should be close to 10
        let p01 = sketch.quantile(0.001).unwrap();
        assert!((1.0..=100.0).contains(&p01));

        // p0 MUST be exact
        assert_eq!(sketch.quantile(0.0), Some(1.0));
    }

    #[test]
    fn test_lra_tail_quantiles_after_many_compactions() {
        let mut sketch = ReqSketch::new(16, ReqMode::LowRankAccuracy).unwrap();

        // Small k forces many compactions
        for i in 1..=100000 {
            sketch.update(i as f64);
        }

        // Even after many compactions, p0 MUST be exact
        assert_eq!(sketch.quantile(0.0), Some(1.0));
        assert_eq!(sketch.min(), Some(1.0));

        // Low quantiles should still be reasonable (with k=16, accuracy is limited)
        let p1 = sketch.quantile(0.01).unwrap();
        assert!(p1 <= 5000.0); // Should be reasonably close to 1000
    }
}

#[cfg(test)]
mod compaction_tests {
    use super::*;

    #[test]
    fn test_compaction_triggers() {
        let mut sketch = ReqSketch::new(8, ReqMode::HighRankAccuracy).unwrap();

        // With k=8, should trigger compaction at some point
        for i in 1..=100 {
            sketch.update(i as f64);
        }

        // Verify it still works correctly
        assert_eq!(sketch.n(), 100);
        assert_eq!(sketch.min(), Some(1.0));
        assert_eq!(sketch.max(), Some(100.0));
    }

    #[test]
    fn test_multiple_compaction_levels() {
        let mut sketch = ReqSketch::new(8, ReqMode::HighRankAccuracy).unwrap();

        // Force multiple levels of compaction
        for i in 1..=10000 {
            sketch.update(i as f64);
        }

        // Should still maintain accuracy
        assert_eq!(sketch.quantile(1.0), Some(10000.0));
        assert_eq!(sketch.quantile(0.0), Some(1.0));
    }

    #[test]
    fn test_compaction_preserves_hra_p100() {
        let mut sketch = ReqSketch::new(16, ReqMode::HighRankAccuracy).unwrap();

        // Add many values to force compaction
        for i in 1..=50000 {
            sketch.update(i as f64);
        }

        // p100 MUST still be exact after all compactions
        assert_eq!(sketch.quantile(1.0), Some(50000.0));
    }

    #[test]
    fn test_compaction_preserves_lra_p0() {
        let mut sketch = ReqSketch::new(16, ReqMode::LowRankAccuracy).unwrap();

        // Add many values to force compaction
        for i in 1..=50000 {
            sketch.update(i as f64);
        }

        // p0 MUST still be exact after all compactions
        assert_eq!(sketch.quantile(0.0), Some(1.0));
    }
}

#[cfg(test)]
mod merge_tests {
    use super::*;

    #[test]
    fn test_merge_empty_sketches() {
        let sketch1 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        let sketch2 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        let merged = sketch1.merge(&sketch2);
        assert!(merged.is_ok());
        assert!(merged.unwrap().is_empty());
    }

    #[test]
    fn test_merge_empty_with_nonempty() {
        let mut sketch1 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        let sketch2 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        for i in 1..=100 {
            sketch1.update(i as f64);
        }

        let merged = sketch1.merge(&sketch2).unwrap();
        assert_eq!(merged.n(), 100);
        assert_eq!(merged.min(), Some(1.0));
        assert_eq!(merged.max(), Some(100.0));
    }

    #[test]
    fn test_merge_incompatible_k() {
        let sketch1 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        let sketch2 = ReqSketch::new(64, ReqMode::HighRankAccuracy).unwrap();

        let merged = sketch1.merge(&sketch2);
        assert!(merged.is_err());
    }

    #[test]
    fn test_merge_incompatible_modes() {
        let sketch1 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        let sketch2 = ReqSketch::new(32, ReqMode::LowRankAccuracy).unwrap();

        let merged = sketch1.merge(&sketch2);
        assert!(merged.is_err());
    }

    #[test]
    fn test_merge_preserves_hra_p100() {
        let mut sketch1 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        let mut sketch2 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        // sketch1: 1-500, max=500
        for i in 1..=500 {
            sketch1.update(i as f64);
        }

        // sketch2: 1-300, max=300
        for i in 1..=300 {
            sketch2.update(i as f64);
        }

        let merged = sketch1.merge(&sketch2).unwrap();

        // After merge, p100 MUST be 500 (the true max)
        assert_eq!(merged.quantile(1.0), Some(500.0));
        assert_eq!(merged.max(), Some(500.0));
        assert_eq!(merged.n(), 800);
    }

    #[test]
    fn test_merge_preserves_lra_p0() {
        let mut sketch1 = ReqSketch::new(32, ReqMode::LowRankAccuracy).unwrap();
        let mut sketch2 = ReqSketch::new(32, ReqMode::LowRankAccuracy).unwrap();

        // sketch1: 10-500, min=10
        for i in 10..=500 {
            sketch1.update(i as f64);
        }

        // sketch2: 5-300, min=5
        for i in 5..=300 {
            sketch2.update(i as f64);
        }

        let merged = sketch1.merge(&sketch2).unwrap();

        // After merge, p0 MUST be 5 (the true min)
        assert_eq!(merged.quantile(0.0), Some(5.0));
        assert_eq!(merged.min(), Some(5.0));
    }

    #[test]
    fn test_merge_multiple_sketches() {
        let mut sketch1 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        let mut sketch2 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();
        let mut sketch3 = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        for i in 1..=100 {
            sketch1.update(i as f64);
        }
        for i in 101..=200 {
            sketch2.update(i as f64);
        }
        for i in 201..=300 {
            sketch3.update(i as f64);
        }

        let merged12 = sketch1.merge(&sketch2).unwrap();
        let merged_all = merged12.merge(&sketch3).unwrap();

        assert_eq!(merged_all.n(), 300);
        assert_eq!(merged_all.min(), Some(1.0));
        assert_eq!(merged_all.max(), Some(300.0));
        assert_eq!(merged_all.quantile(1.0), Some(300.0)); // p100 exact
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use rand::Rng;

    /// ⭐ CRITICAL PROPERTY: HRA mode always has p100 = max
    #[test]
    fn property_hra_p100_always_exact() {
        let mut rng = rand::rng();

        for _ in 0..100 {
            let k = rng.random_range(8..=128);
            let mut sketch = ReqSketch::new(k, ReqMode::HighRankAccuracy).unwrap();

            let n = rng.random_range(100..=5000);
            let mut true_max = f64::NEG_INFINITY;

            for _ in 0..n {
                let value = rng.random_range(-1000.0..=1000.0);
                sketch.update(value);
                true_max = true_max.max(value);
            }

            // p100 MUST equal the true maximum
            assert_eq!(sketch.quantile(1.0), Some(true_max));
            assert_eq!(sketch.max(), Some(true_max));
        }
    }

    /// ⭐ CRITICAL PROPERTY: LRA mode always has p0 = min
    #[test]
    fn property_lra_p0_always_exact() {
        let mut rng = rand::rng();

        for _ in 0..100 {
            let k = rng.random_range(8..=128);
            let mut sketch = ReqSketch::new(k, ReqMode::LowRankAccuracy).unwrap();

            let n = rng.random_range(100..=5000);
            let mut true_min = f64::INFINITY;

            for _ in 0..n {
                let value = rng.random_range(-1000.0..=1000.0);
                sketch.update(value);
                true_min = true_min.min(value);
            }

            // p0 MUST equal the true minimum
            assert_eq!(sketch.quantile(0.0), Some(true_min));
            assert_eq!(sketch.min(), Some(true_min));
        }
    }

    #[test]
    fn property_quantiles_are_ordered() {
        let mut rng = rand::rng();

        for _ in 0..50 {
            let k = rng.random_range(16..=64);
            let mut sketch = ReqSketch::new(k, ReqMode::HighRankAccuracy).unwrap();

            for _ in 0..1000 {
                let value = rng.random_range(0.0..=1000.0);
                sketch.update(value);
            }

            // Quantiles should be monotonically increasing
            let quantiles = [0.0, 0.25, 0.5, 0.75, 0.9, 0.99, 1.0];
            let values: Vec<f64> = quantiles
                .iter()
                .map(|&q| sketch.quantile(q).unwrap())
                .collect();

            for i in 1..values.len() {
                assert!(
                    values[i] >= values[i - 1],
                    "Quantiles not ordered: {:?}",
                    values
                );
            }
        }
    }

    #[test]
    fn property_merge_preserves_extremes() {
        let mut rng = rand::rng();

        for _ in 0..50 {
            let k = rng.random_range(16..=64);
            let mut sketch1 = ReqSketch::new(k, ReqMode::HighRankAccuracy).unwrap();
            let mut sketch2 = ReqSketch::new(k, ReqMode::HighRankAccuracy).unwrap();

            let mut global_max = f64::NEG_INFINITY;
            let mut global_min = f64::INFINITY;

            for _ in 0..500 {
                let value = rng.random_range(-500.0..=500.0);
                sketch1.update(value);
                global_max = global_max.max(value);
                global_min = global_min.min(value);
            }

            for _ in 0..500 {
                let value = rng.random_range(-500.0..=500.0);
                sketch2.update(value);
                global_max = global_max.max(value);
                global_min = global_min.min(value);
            }

            let merged = sketch1.merge(&sketch2).unwrap();

            // Merged sketch MUST preserve exact extremes
            assert_eq!(merged.max(), Some(global_max));
            assert_eq!(merged.min(), Some(global_min));
            assert_eq!(merged.quantile(1.0), Some(global_max));
            assert_eq!(merged.quantile(0.0), Some(global_min));
        }
    }

    #[test]
    fn property_count_is_accurate() {
        let mut rng = rand::rng();

        for _ in 0..50 {
            let k = rng.random_range(8..=64);
            let mut sketch = ReqSketch::new(k, ReqMode::HighRankAccuracy).unwrap();

            let n = rng.random_range(100..=5000);
            for _ in 0..n {
                let value = rng.random_range(0.0..=1000.0);
                sketch.update(value);
            }

            assert_eq!(sketch.n(), n);
        }
    }

    #[test]
    fn property_all_same_values() {
        let mut rng = rand::rng();

        for _ in 0..20 {
            let k = rng.random_range(8..=64);
            let mut sketch = ReqSketch::new(k, ReqMode::HighRankAccuracy).unwrap();

            let value = rng.random_range(-100.0..=100.0);
            for _ in 0..1000 {
                sketch.update(value);
            }

            // All quantiles should return the same value
            assert_eq!(sketch.quantile(0.0), Some(value));
            assert_eq!(sketch.quantile(0.5), Some(value));
            assert_eq!(sketch.quantile(1.0), Some(value));
            assert_eq!(sketch.min(), Some(value));
            assert_eq!(sketch.max(), Some(value));
        }
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_negative_values() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        for i in -100..=0 {
            sketch.update(i as f64);
        }

        assert_eq!(sketch.min(), Some(-100.0));
        assert_eq!(sketch.max(), Some(0.0));
        assert_eq!(sketch.quantile(1.0), Some(0.0));
    }

    #[test]
    fn test_floating_point_values() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        let values = vec![1.1, 2.2, 3.3, 4.4, 5.5];
        for v in &values {
            sketch.update(*v);
        }

        assert_eq!(sketch.min(), Some(1.1));
        assert_eq!(sketch.max(), Some(5.5));
        assert_eq!(sketch.quantile(1.0), Some(5.5));
    }

    #[test]
    fn test_very_large_n() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        // Add 1 million values
        for i in 1..=1_000_000 {
            sketch.update(i as f64);
        }

        assert_eq!(sketch.n(), 1_000_000);
        assert_eq!(sketch.quantile(1.0), Some(1_000_000.0));
        assert_eq!(sketch.quantile(0.0), Some(1.0));
    }

    #[test]
    fn test_invalid_quantile_queries() {
        let mut sketch = ReqSketch::new(32, ReqMode::HighRankAccuracy).unwrap();

        for i in 1..=100 {
            sketch.update(i as f64);
        }

        // Invalid quantiles should return None
        assert!(sketch.quantile(-0.1).is_none());
        assert!(sketch.quantile(1.1).is_none());
        assert!(sketch.quantile(f64::NAN).is_none());
    }
}
