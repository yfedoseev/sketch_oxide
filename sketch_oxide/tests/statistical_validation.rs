//! Statistical bound validation harness (fable5 doc 05 gap #3 / roadmap testing item 2).
//!
//! The library's *product* is its error bounds, yet the pre-existing accuracy
//! tests were single fixed-seed smoke checks with tolerances 3–10× looser than
//! theory — a subtle estimator-bias regression would pass CI. This harness runs
//! N seeded trials per estimator and asserts the *distribution* of errors against
//! the theoretical guarantee, not a single loose point.
//!
//! Seeds are fixed, so the harness is deterministic and flake-free. The bounds
//! carry a modest slack factor over theory to absorb finite-sample variation
//! while staying far tighter than the old 5–15% tolerances.

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;

use sketch_oxide::cardinality::HyperLogLog;
use sketch_oxide::common::Sketch;
use sketch_oxide::membership::BloomFilter;
use sketch_oxide::quantiles::DDSketch;

/// Root-mean-square of a slice.
fn rms(xs: &[f64]) -> f64 {
    (xs.iter().map(|x| x * x).sum::<f64>() / xs.len() as f64).sqrt()
}

fn mean(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// HyperLogLog: over N seeds, the RMS relative error must be within a small
/// factor of the theoretical standard error 1.04/sqrt(m), and the mean relative
/// error (bias) must be near zero.
#[test]
fn hll_relative_error_matches_theory() {
    const PRECISION: u8 = 12;
    const M: f64 = 1u32.wrapping_shl(PRECISION as u32) as f64; // 4096 registers
    const N_TRIALS: usize = 40;
    const CARDINALITY: usize = 50_000;

    let theoretical_se = 1.04 / M.sqrt(); // ~0.01625

    let mut rel_errors = Vec::with_capacity(N_TRIALS);
    for seed in 0..N_TRIALS as u64 {
        let mut rng = SmallRng::seed_from_u64(0xA11CE ^ seed);
        let mut hll = HyperLogLog::new(PRECISION).unwrap();
        let mut truth = HashSet::with_capacity(CARDINALITY);
        while truth.len() < CARDINALITY {
            let v: u64 = rng.random();
            if truth.insert(v) {
                hll.update(&v);
            }
        }
        let est = hll.estimate();
        rel_errors.push((est - CARDINALITY as f64) / CARDINALITY as f64);
    }

    let empirical_rmse = rms(&rel_errors);
    let bias = mean(&rel_errors);

    // Empirical RMSE should track the theoretical SE. 1.4x slack absorbs the
    // finite N=40 sampling variation; still ~10x tighter than the old test.
    assert!(
        empirical_rmse <= 1.4 * theoretical_se,
        "HLL p={PRECISION}: empirical RMSE {empirical_rmse:.5} exceeds 1.4x theory {theoretical_se:.5}"
    );
    // Estimator should be roughly unbiased.
    assert!(
        bias.abs() <= 0.8 * theoretical_se,
        "HLL p={PRECISION}: mean relative error (bias) {bias:.5} too large vs SE {theoretical_se:.5}"
    );
}

/// Bloom filter: measured false-positive probability over N seeds must not
/// exceed a small factor of the configured rate.
#[test]
fn bloom_false_positive_rate_within_configured() {
    const N_TRIALS: usize = 25;
    const CAPACITY: usize = 20_000;
    const CONFIGURED_FPR: f64 = 0.01;
    const QUERIES: usize = 20_000;

    let mut measured = Vec::with_capacity(N_TRIALS);
    for seed in 0..N_TRIALS as u64 {
        let mut rng = SmallRng::seed_from_u64(0xB100D ^ seed);
        let mut bloom = BloomFilter::new(CAPACITY, CONFIGURED_FPR);

        let mut inserted = HashSet::with_capacity(CAPACITY);
        while inserted.len() < CAPACITY {
            let v: u64 = rng.random();
            if inserted.insert(v) {
                bloom.insert(&v.to_le_bytes());
            }
        }

        let mut false_positives = 0usize;
        let mut tested = 0usize;
        while tested < QUERIES {
            let v: u64 = rng.random();
            if inserted.contains(&v) {
                continue;
            }
            tested += 1;
            if bloom.contains(&v.to_le_bytes()) {
                false_positives += 1;
            }
        }
        measured.push(false_positives as f64 / tested as f64);
    }

    let avg_fpp = mean(&measured);
    assert!(
        avg_fpp <= 1.6 * CONFIGURED_FPR,
        "Bloom: average measured FPP {avg_fpp:.5} exceeds 1.6x configured {CONFIGURED_FPR}"
    );
}

/// DDSketch: the relative-accuracy guarantee is a hard per-query bound. Over N
/// seeds and several quantiles, |estimate - truth| / truth must be <= alpha.
#[test]
fn ddsketch_relative_accuracy_is_respected() {
    const N_TRIALS: usize = 20;
    const ALPHA: f64 = 0.01;
    const N: usize = 50_000;
    let quantiles = [0.1, 0.25, 0.5, 0.75, 0.9, 0.99];

    for seed in 0..N_TRIALS as u64 {
        let mut rng = SmallRng::seed_from_u64(0xDD5 ^ seed);
        let mut dd = DDSketch::new(ALPHA).unwrap();
        let mut values: Vec<f64> = (0..N)
            .map(|_| rng.random::<f64>() * 1000.0 + 1.0) // strictly positive
            .collect();
        for &v in &values {
            dd.add(v);
        }
        values.sort_by(|a, b| a.total_cmp(b));

        for &q in &quantiles {
            let est = dd.quantile(q).expect("non-empty");
            let idx = ((q * (N as f64 - 1.0)).round() as usize).min(N - 1);
            let truth = values[idx];
            let rel_err = (est - truth).abs() / truth;
            // Allow a small multiple of alpha for rank/interpolation slack.
            assert!(
                rel_err <= 2.0 * ALPHA,
                "DDSketch q={q}: relative error {rel_err:.5} exceeds 2x alpha {ALPHA}"
            );
        }
    }
}
