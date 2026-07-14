//! Capability traits — the fable5 doc 01 F3 "split the `Sketch` trait" work.
//!
//! The original [`Sketch`](crate::common::Sketch) trait is simultaneously too
//! narrow (one `f64` `estimate`), too broad (mandatory infallible `serialize`),
//! and under-adopted. That forces misfits: `CountMinSketch::estimate` returns a
//! placeholder `0.0`, `BinaryFuseFilter::update` panics, and
//! `SpaceSaving::serialize` silently drops data because it cannot fail.
//!
//! These smaller, orthogonal capability traits let each type express exactly
//! what it supports:
//! - [`Update`] — streaming ingest of `T` (generic, unlike `Sketch`'s erased
//!   `type Item`).
//! - [`CardinalityEstimate`] — set-cardinality estimation.
//! - [`PointQuery`] — per-item frequency/count (what Count-Min actually does).
//! - [`Filter`] — approximate membership.
//! - [`QuantileQuery`] — rank/quantile queries.
//! - [`Serializable`] — *fallible* encoding, so a type that cannot always encode
//!   reports an error instead of lying.
//!
//! This layer is additive: the existing `Sketch`/`Mergeable` impls are
//! unchanged, so nothing downstream breaks. New generic infrastructure can
//! target the precise capability it needs.

use super::error::{Result, SketchError};

/// Streaming ingest of items of type `T`.
///
/// Unlike `Sketch` (whose associated `type Item` erases the real genericity),
/// a type may implement `Update<T>` for every `T` it accepts. Immutable,
/// build-once structures (e.g. Binary Fuse filters) simply do **not** implement
/// this — resolving the Liskov violation where `Sketch::update` had to panic.
pub trait Update<T: ?Sized> {
    /// Fold `item` into the sketch.
    fn update(&mut self, item: &T);
}

/// A sketch that estimates the number of distinct items.
pub trait CardinalityEstimate {
    /// Estimated cardinality (number of distinct items seen).
    #[must_use]
    fn estimate_cardinality(&self) -> f64;
}

/// A point-query sketch: estimate the frequency/count of a specific item.
///
/// This is the capability Count-Min actually has — a single scalar `estimate()`
/// (as `Sketch` requires) is meaningless for it, which is why its `Sketch` impl
/// had to return a placeholder.
pub trait PointQuery<T: ?Sized> {
    /// Estimated count of `item` (never an underestimate for CM-family sketches).
    #[must_use]
    fn query(&self, item: &T) -> u64;
}

/// Approximate-membership filter over items of type `T`.
pub trait Filter<T: ?Sized> {
    /// Whether `item` might be present (may be a false positive; never a false
    /// negative for the standard filter families).
    #[must_use]
    fn contains(&self, item: &T) -> bool;
}

/// Rank/quantile queries over an ordered stream.
pub trait QuantileQuery {
    /// The value at normalized rank `rank` in `[0, 1]`, or `None` if empty.
    #[must_use]
    fn quantile(&self, rank: f64) -> Option<f64>;
}

/// Fallible, self-describing serialization.
///
/// Unlike `Sketch::serialize` (which returns `Vec<u8>` infallibly and so forces
/// types that cannot always encode to silently drop data), this returns a
/// `Result` so encoding can honestly fail.
pub trait Serializable: Sized {
    /// Encode to bytes, or `Err` if this instance cannot be encoded.
    fn to_bytes(&self) -> Result<Vec<u8>>;

    /// Decode from bytes.
    fn from_bytes(bytes: &[u8]) -> Result<Self>;
}

// ---------------------------------------------------------------------------
// Representative adoptions demonstrating the split resolves the real misfits.
// (Full library-wide adoption across all modules is the follow-on migration.)
// ---------------------------------------------------------------------------

use crate::cardinality::HyperLogLog;
use crate::common::Sketch;
use crate::frequency::{CountMinSketch, SpaceSaving};
use crate::membership::{BinaryFuseFilter, BloomFilter};
use crate::quantiles::DDSketch;
use std::hash::Hash;

impl<T: Hash> Update<T> for HyperLogLog {
    fn update(&mut self, item: &T) {
        HyperLogLog::update(self, item);
    }
}

impl CardinalityEstimate for HyperLogLog {
    fn estimate_cardinality(&self) -> f64 {
        <Self as Sketch>::estimate(self)
    }
}

impl<T: Hash> Update<T> for CountMinSketch {
    fn update(&mut self, item: &T) {
        CountMinSketch::update(self, item);
    }
}

// The capability Count-Min really has — no placeholder `0.0`.
impl<T: Hash> PointQuery<T> for CountMinSketch {
    fn query(&self, item: &T) -> u64 {
        self.estimate(item)
    }
}

impl Update<[u8]> for BloomFilter {
    fn update(&mut self, item: &[u8]) {
        self.insert(item);
    }
}

impl Filter<[u8]> for BloomFilter {
    fn contains(&self, item: &[u8]) -> bool {
        BloomFilter::contains(self, item)
    }
}

// Binary Fuse is build-once/immutable: it implements `Filter` but deliberately
// NOT `Update`, so the type system rejects `update` at compile time instead of
// panicking at runtime.
impl Filter<u64> for BinaryFuseFilter {
    fn contains(&self, item: &u64) -> bool {
        BinaryFuseFilter::contains(self, item)
    }
}

impl Update<f64> for DDSketch {
    fn update(&mut self, item: &f64) {
        self.add(*item);
    }
}

impl QuantileQuery for DDSketch {
    fn quantile(&self, rank: f64) -> Option<f64> {
        DDSketch::quantile(self, rank)
    }
}

// Honest, fallible serialization for the generic Space-Saving counters: rather
// than silently drop counters (as the infallible `Sketch::serialize` does), it
// reports `Unsupported` when counters are present.
impl<T: Hash + Eq + Clone + 'static> Serializable for SpaceSaving<T> {
    fn to_bytes(&self) -> Result<Vec<u8>> {
        if self.num_items() > 0 {
            return Err(SketchError::Unsupported {
                op: "SpaceSaving::to_bytes with populated counters (generic T not yet byte-encodable)",
            });
        }
        Ok(<Self as Sketch>::serialize(self))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        <Self as Sketch>::deserialize(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A generic pipeline that works over ANY cardinality estimator via the
    // capability traits — the composition the split is meant to enable.
    fn count_distinct<S: Update<u64> + CardinalityEstimate>(sketch: &mut S, items: &[u64]) -> f64 {
        for &x in items {
            sketch.update(&x);
        }
        sketch.estimate_cardinality()
    }

    #[test]
    fn generic_cardinality_pipeline_over_capability_traits() {
        let mut hll = HyperLogLog::new(12).unwrap();
        let items: Vec<u64> = (0..10_000).collect();
        let est = count_distinct(&mut hll, &items);
        assert!((est - 10_000.0).abs() < 1_000.0, "HLL estimate off: {est}");
    }

    #[test]
    fn count_min_point_query_is_meaningful_not_placeholder() {
        let mut cm = CountMinSketch::new(0.01, 0.01).unwrap();
        for _ in 0..5 {
            Update::update(&mut cm, &42u64);
        }
        // Via the capability trait we get a real count, not the `Sketch::estimate`
        // placeholder `0.0`.
        assert!(PointQuery::query(&cm, &42u64) >= 5);
    }

    #[test]
    fn binary_fuse_is_filter_but_not_update() {
        let filter = BinaryFuseFilter::from_items([1u64, 2, 3].into_iter(), 9).unwrap();
        assert!(Filter::contains(&filter, &1u64));
        assert!(!Filter::contains(&filter, &99u64));
        // Note: `BinaryFuseFilter` deliberately does NOT implement `Update<_>`,
        // so `Update::update(&mut filter, &x)` would fail to compile — the
        // immutability is enforced by the type system, not a runtime panic.
    }

    #[test]
    fn ddsketch_quantile_via_capability_trait() {
        let mut dd = DDSketch::new(0.01).unwrap();
        for i in 1..=1000 {
            Update::update(&mut dd, &(i as f64));
        }
        let median = QuantileQuery::quantile(&dd, 0.5).unwrap();
        assert!((median - 500.0).abs() / 500.0 <= 0.02);
    }

    #[test]
    fn space_saving_serialize_is_honest_when_populated() {
        let mut ss: SpaceSaving<u64> = SpaceSaving::with_capacity(8).unwrap();
        ss.update(1);
        ss.update(2);
        // Fallible capability reports the limitation instead of dropping data.
        assert!(Serializable::to_bytes(&ss).is_err());

        let empty: SpaceSaving<u64> = SpaceSaving::with_capacity(8).unwrap();
        assert!(Serializable::to_bytes(&empty).is_ok());
    }
}
