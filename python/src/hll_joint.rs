//! Python bindings for joint cardinality estimation over two HyperLogLog sketches.

use crate::hyperloglog::HyperLogLog;
use pyo3::prelude::*;
use sketch_oxide::cardinality::HyperLogLog as RustHyperLogLog;
use sketch_oxide::statistics::HllJointEstimator as RustHllJointEstimator;

/// HllJointEstimator — estimates joint set quantities (union, intersection, and
/// Jaccard similarity) between two :class:`HyperLogLog` sketches of equal
/// precision, via inclusion–exclusion on their register states.
///
/// Args:
///     a (HyperLogLog): the first sketch.
///     b (HyperLogLog): the second sketch (must have the same precision as `a`).
#[pyclass(module = "sketch_oxide")]
pub struct HllJointEstimator {
    a: RustHyperLogLog,
    b: RustHyperLogLog,
}

impl HllJointEstimator {
    /// Builds the (borrowing) core estimator over the stored clones. Precision was
    /// validated at construction, so this never fails.
    fn estimator(&self) -> RustHllJointEstimator<'_> {
        RustHllJointEstimator::new(&self.a, &self.b).expect("precision validated at construction")
    }
}

#[pymethods]
impl HllJointEstimator {
    #[new]
    fn new(a: &HyperLogLog, b: &HyperLogLog) -> PyResult<Self> {
        // Validate compatibility up front (clones are immutable afterwards).
        RustHllJointEstimator::new(&a.inner, &b.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            a: a.inner.clone(),
            b: b.inner.clone(),
        })
    }

    /// Estimated cardinality of `A`.
    fn cardinality_a(&self) -> f64 {
        self.estimator().cardinality_a()
    }

    /// Estimated cardinality of `B`.
    fn cardinality_b(&self) -> f64 {
        self.estimator().cardinality_b()
    }

    /// Estimated cardinality of `A ∪ B`.
    fn union(&self) -> f64 {
        self.estimator().union()
    }

    /// Estimated cardinality of `A ∩ B` (via inclusion–exclusion).
    fn intersection(&self) -> f64 {
        self.estimator().intersection()
    }

    /// Estimated Jaccard similarity `|A ∩ B| / |A ∪ B|`.
    fn jaccard(&self) -> f64 {
        self.estimator().jaccard()
    }

    fn __repr__(&self) -> String {
        let e = self.estimator();
        format!(
            "HllJointEstimator(union={:.0}, jaccard={:.3})",
            e.union(),
            e.jaccard()
        )
    }
}
