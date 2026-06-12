//! Python bindings for the coded-symbol (practical) Rateless IBLT.

use pyo3::prelude::*;
use sketch_oxide::reconciliation::RatelessIblt as RustRatelessIblt;

/// RatelessIbltCoded — the practical, coded-symbol Rateless IBLT for streaming set
/// reconciliation over `u64` keys. The sender emits an unbounded sequence of
/// coded symbols; the receiver consumes just enough to peel out the symmetric
/// difference, so no symmetric-difference size estimate is needed up front.
///
/// For the common one-shot case use the static :meth:`reconcile` helper. The
/// per-symbol streaming transport (`CodedSymbol`) is intentionally not exposed.
#[pyclass(module = "sketch_oxide")]
pub struct RatelessIbltCoded {
    inner: RustRatelessIblt,
}

#[pymethods]
impl RatelessIbltCoded {
    #[new]
    fn new() -> Self {
        Self {
            inner: RustRatelessIblt::new(),
        }
    }

    /// Adds an integer key to the local set.
    fn insert(&mut self, sym: u64) {
        self.inner.insert(sym);
    }

    /// Number of coded symbols produced so far.
    fn produced(&self) -> u64 {
        self.inner.produced()
    }

    /// One-shot reconciliation: given local set `a`, remote set `b`, and a coded-
    /// symbol budget `num_coded`, returns `(a_only, b_only)` or None if the budget
    /// was insufficient to peel the difference.
    #[staticmethod]
    fn reconcile(a: Vec<u64>, b: Vec<u64>, num_coded: usize) -> Option<(Vec<u64>, Vec<u64>)> {
        RustRatelessIblt::reconcile(&a, &b, num_coded)
    }

    fn __repr__(&self) -> String {
        format!("RatelessIbltCoded(produced={})", self.inner.produced())
    }
}
