//! Bulk-ingest ergonomics: `Extend` for the hot sketches (fable5 doc 01 F8/F9,
//! doc 02 #5).
//!
//! Before this the core had zero `Extend`/`FromIterator` impls, so every binding
//! re-implemented batching by looping over per-item FFI calls. Implementing
//! `Extend` gives idiomatic bulk construction (`sketch.extend(iter)`,
//! `iter.collect()`-style flows) and a single place for a binding batch fast
//! path to bottom out (amortizing the FFI crossing over a whole slice).
//!
//! These are thin `for`-loops over the existing `update` today; a future SIMD
//! kernel can specialize them without changing the call sites.

use crate::cardinality::HyperLogLog;
use crate::frequency::CountMinSketch;
use crate::quantiles::{DDSketch, KllSketch};
use crate::similarity::MinHash;
use std::hash::Hash;

impl<T: Hash> Extend<T> for HyperLogLog {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.update(&item);
        }
    }
}

impl<T: Hash> Extend<T> for CountMinSketch {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.update(&item);
        }
    }
}

impl<T: Hash> Extend<T> for MinHash {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.update(&item);
        }
    }
}

impl Extend<f64> for DDSketch {
    fn extend<I: IntoIterator<Item = f64>>(&mut self, iter: I) {
        for v in iter {
            self.add(v);
        }
    }
}

impl Extend<f64> for KllSketch {
    fn extend<I: IntoIterator<Item = f64>>(&mut self, iter: I) {
        for v in iter {
            self.update(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::Sketch;

    #[test]
    fn extend_matches_repeated_update() {
        let items: Vec<u64> = (0..5_000).collect();

        let mut a = HyperLogLog::new(12).unwrap();
        a.extend(items.iter().copied());

        let mut b = HyperLogLog::new(12).unwrap();
        for &x in &items {
            b.update(&x);
        }
        assert_eq!(a.estimate(), b.estimate());
    }

    #[test]
    fn extend_quantile_sketches() {
        let mut dd = DDSketch::new(0.01).unwrap();
        dd.extend((1..=1000).map(|i| i as f64));
        assert!((dd.quantile(0.5).unwrap() - 500.0).abs() / 500.0 <= 0.02);

        let mut kll = KllSketch::new(200).unwrap();
        kll.extend((1..=1000).map(|i| i as f64));
        assert!(kll.quantile(0.5).is_some());
    }
}
