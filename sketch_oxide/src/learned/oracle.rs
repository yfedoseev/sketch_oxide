//! The oracle / score-function interface for ML-augmented sketches.
//!
//! Learned data structures (PLBF, Sandwiched / Ada-BF learned Bloom filters, learned
//! Count-Min / Count-Sketch, learning-augmented Misra-Gries and windows) all share one
//! dependency: a way for the user to supply a model's prediction for a key. This module
//! defines that interface once, so every learned sketch consumes scores the same way.
//!
//! # What crosses the boundary: scores, not models
//!
//! The model itself stays on the caller's side. A sketch only ever asks "what is the score
//! for this key?" via [`Oracle::score`]. In Rust the oracle can be a closure
//! ([`ClosureOracle`]); across the FFI boundary — where calling back into a Python/JVM
//! model per element would dominate cost — scores are computed host-side in batches and
//! supplied as data ([`PrecomputedOracle`]). Either way the sketch sees only `&[u8] ->
//! Score`.
//!
//! # Score semantics
//!
//! A [`Score`] is a real number where **higher means more strongly predicted**. Most
//! learned filters interpret it as an estimate of `P(key is a positive)` in `[0, 1]`, but
//! the interface does not require a particular range — a learned Count-Min may use a
//! predicted frequency, for instance. Each consuming sketch documents how it reads the
//! score (e.g. a threshold, or a partition boundary).

use std::collections::HashMap;

/// A model's prediction for a key. Higher means more strongly predicted (e.g. more likely
/// to be a member, or a higher predicted frequency). See the module docs for semantics.
pub type Score = f64;

/// A user-supplied predictor that scores keys for a learned sketch.
///
/// Implement this to plug a model into any learned structure. Two ready-made adapters are
/// provided: [`ClosureOracle`] (wrap a Rust `Fn`) and [`PrecomputedOracle`] (host-computed
/// scores supplied as data, the FFI-friendly path).
pub trait Oracle {
    /// Returns the score for `key`. Must be deterministic for a given key so that repeated
    /// lookups (e.g. insert then query) agree.
    fn score(&self, key: &[u8]) -> Score;
}

/// Adapts any `Fn(&[u8]) -> Score` into an [`Oracle`].
///
/// # Examples
/// ```
/// use sketch_oxide::learned::{ClosureOracle, Oracle};
///
/// // Score keys by length, normalised — just for illustration.
/// let oracle = ClosureOracle::new(|key: &[u8]| key.len() as f64 / 32.0);
/// assert!(oracle.score(b"short") < oracle.score(b"a-much-longer-key-value"));
/// ```
pub struct ClosureOracle<F> {
    f: F,
}

impl<F: Fn(&[u8]) -> Score> ClosureOracle<F> {
    /// Wraps a scoring closure.
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F: Fn(&[u8]) -> Score> Oracle for ClosureOracle<F> {
    #[inline]
    fn score(&self, key: &[u8]) -> Score {
        (self.f)(key)
    }
}

/// An [`Oracle`] backed by host-computed scores — the canonical FFI path.
///
/// The model runs on the caller's side (Python/JVM/.NET), scores a batch of keys, and
/// hands the `(key, score)` pairs to the sketch. Keys not present fall back to a default
/// score, so unseen keys behave predictably (e.g. "treat as negative").
///
/// # Examples
/// ```
/// use sketch_oxide::learned::{PrecomputedOracle, Oracle};
///
/// let oracle = PrecomputedOracle::from_pairs(
///     [(b"hot".to_vec(), 0.9), (b"cold".to_vec(), 0.1)],
///     0.0, // default for unseen keys
/// );
/// assert_eq!(oracle.score(b"hot"), 0.9);
/// assert_eq!(oracle.score(b"unseen"), 0.0);
/// ```
#[derive(Clone, Debug, Default)]
pub struct PrecomputedOracle {
    scores: HashMap<Vec<u8>, Score>,
    default: Score,
}

impl PrecomputedOracle {
    /// Creates an empty oracle that returns `default` for every key until scores are added.
    pub fn new(default: Score) -> Self {
        Self {
            scores: HashMap::new(),
            default,
        }
    }

    /// Builds an oracle from `(key, score)` pairs, using `default` for unseen keys.
    pub fn from_pairs<I>(pairs: I, default: Score) -> Self
    where
        I: IntoIterator<Item = (Vec<u8>, Score)>,
    {
        Self {
            scores: pairs.into_iter().collect(),
            default,
        }
    }

    /// Inserts or updates the score for `key`.
    pub fn set(&mut self, key: &[u8], score: Score) {
        self.scores.insert(key.to_vec(), score);
    }

    /// The default score returned for keys with no stored value.
    #[inline]
    pub fn default_score(&self) -> Score {
        self.default
    }

    /// Number of keys with an explicit stored score.
    #[inline]
    pub fn len(&self) -> usize {
        self.scores.len()
    }

    /// Whether no explicit scores are stored (every key returns the default).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.scores.is_empty()
    }
}

impl Oracle for PrecomputedOracle {
    #[inline]
    fn score(&self, key: &[u8]) -> Score {
        self.scores.get(key).copied().unwrap_or(self.default)
    }
}

/// Blanket impl so a reference to an oracle is itself an oracle — lets sketches accept
/// `&O` without taking ownership.
impl<O: Oracle + ?Sized> Oracle for &O {
    #[inline]
    fn score(&self, key: &[u8]) -> Score {
        (**self).score(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_oracle_scores() {
        let oracle = ClosureOracle::new(|k: &[u8]| k.len() as f64);
        assert_eq!(oracle.score(b"abc"), 3.0);
        assert_eq!(oracle.score(b""), 0.0);
    }

    #[test]
    fn precomputed_oracle_uses_default_for_unseen() {
        let oracle = PrecomputedOracle::from_pairs([(b"a".to_vec(), 1.0)], -1.0);
        assert_eq!(oracle.score(b"a"), 1.0);
        assert_eq!(oracle.score(b"missing"), -1.0);
    }

    #[test]
    fn precomputed_oracle_set_and_len() {
        let mut oracle = PrecomputedOracle::new(0.0);
        assert!(oracle.is_empty());
        oracle.set(b"x", 0.5);
        oracle.set(b"x", 0.7); // overwrite
        assert_eq!(oracle.len(), 1);
        assert_eq!(oracle.score(b"x"), 0.7);
    }

    #[test]
    fn reference_is_an_oracle() {
        // A learned sketch can take `&O: Oracle` generically.
        fn score_with<O: Oracle>(o: O, key: &[u8]) -> Score {
            o.score(key)
        }
        let oracle = ClosureOracle::new(|_: &[u8]| 42.0);
        assert_eq!(score_with(&oracle, b"k"), 42.0);
        // original still usable after passing &oracle
        assert_eq!(oracle.score(b"k"), 42.0);
    }

    #[test]
    fn oracle_is_object_safe() {
        let oracle = PrecomputedOracle::from_pairs([(b"k".to_vec(), 9.0)], 0.0);
        let dynamic: &dyn Oracle = &oracle;
        assert_eq!(dynamic.score(b"k"), 9.0);
    }
}
