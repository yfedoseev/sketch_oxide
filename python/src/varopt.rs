//! Python bindings for VarOpt Sampling

use pyo3::prelude::*;
use pyo3::types::PyList;
use sketch_oxide::sampling::VarOptSampling as RustVarOptSampling;

/// VarOpt Sampling for variance-optimal weighted samples (Cohen 2014)
///
/// Maintains a weighted sample where higher-weight items have higher
/// probability of inclusion. Heavy items are always included.
///
/// Args:
///     k (int): Maximum sample size
///     seed (int, optional): Random seed for reproducibility
///
/// Example:
///     >>> sampler = VarOptSampling(k=10)
///     >>> sampler.update("high_value_tx", 10000.0)  # Always included
///     >>> for i in range(1000):
///     ...     sampler.update(f"tx_{i}", 1.0)  # Randomly sampled
///     >>> sample = sampler.sample()
///     >>> # Heavy items always present, light items sampled
///
/// Notes:
///     - Heavy items (weight >= threshold) always in sample
///     - Light items probabilistically sampled
///     - Items are stored as strings (use str() for other types)
///     - Use for network traffic analysis, transaction monitoring
#[pyclass(module = "sketch_oxide")]
pub struct VarOptSampling {
    inner: RustVarOptSampling<String>,
}

#[pymethods]
impl VarOptSampling {
    /// Create a new VarOpt Sampler
    ///
    /// Args:
    ///     k: Maximum sample size
    ///     seed: Optional random seed for reproducibility
    ///
    /// Raises:
    ///     ValueError: If k is 0
    #[new]
    #[pyo3(signature = (k, seed=None))]
    fn new(k: usize, seed: Option<u64>) -> PyResult<Self> {
        let inner = match seed {
            Some(s) => RustVarOptSampling::with_seed(k, s),
            None => RustVarOptSampling::new(k),
        };
        inner
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Add a weighted item
    ///
    /// Args:
    ///     item: Item to add (converted to string)
    ///     weight: Positive weight (higher = more likely to be sampled)
    ///
    /// Raises:
    ///     ValueError: If weight is not positive
    fn update(&mut self, item: &str, weight: f64) -> PyResult<()> {
        if weight <= 0.0 || !weight.is_finite() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Weight must be positive and finite",
            ));
        }
        self.inner.update(item.to_string(), weight);
        Ok(())
    }

    /// Get the current sample as list of (item, weight) tuples
    ///
    /// Returns:
    ///     list: List of (item, weight) tuples
    fn sample<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let items: Vec<(&String, f64)> = self
            .inner
            .sample()
            .iter()
            .map(|wi| (&wi.item, wi.weight))
            .collect();
        PyList::new_bound(py, items)
    }

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of items in sample
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Maximum capacity
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Total items seen
    fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Current threshold for heavy vs light items
    fn threshold(&self) -> f64 {
        self.inner.threshold()
    }

    /// Total weight in sample
    fn total_weight(&self) -> f64 {
        self.inner.total_weight()
    }

    /// Clear the sampler
    fn clear(&mut self) {
        self.inner.clear();
    }

    fn __repr__(&self) -> String {
        format!(
            "VarOptSampling(k={}, count={})",
            self.inner.capacity(),
            self.inner.count()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "VarOptSampling: {} items in sample, {} items seen, threshold={:.2}",
            self.inner.len(),
            self.inner.count(),
            self.inner.threshold()
        )
    }
}
