//! Python bindings for Reservoir Sampling

use pyo3::prelude::*;
use pyo3::types::PyList;
use sketch_oxide::sampling::ReservoirSampling as RustReservoirSampling;

/// Reservoir Sampling for uniform random samples (Vitter 1985)
///
/// Maintains a uniform random sample of k items from a stream of unknown length.
/// Every item has equal probability k/n of being in the sample.
///
/// Args:
///     k (int): Size of the reservoir (number of items to sample)
///     seed (int, optional): Random seed for reproducibility
///
/// Example:
///     >>> reservoir = ReservoirSampling(k=10)
///     >>> for i in range(1000):
///     ...     reservoir.update(f"item_{i}")
///     >>> sample = reservoir.sample()
///     >>> print(f"Got {len(sample)} items from 1000")  # Always 10
///
/// Notes:
///     - O(1) update time
///     - Each item has k/n probability of being in sample
///     - Items are stored as strings (use str() for other types)
///     - Use for log sampling, A/B testing, data quality checks
#[pyclass(module = "sketch_oxide")]
pub struct ReservoirSampling {
    inner: RustReservoirSampling<String>,
}

#[pymethods]
impl ReservoirSampling {
    /// Create a new Reservoir Sampler
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
            Some(s) => RustReservoirSampling::with_seed(k, s),
            None => RustReservoirSampling::new(k),
        };
        inner
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Add an item to the reservoir
    ///
    /// Args:
    ///     item: Item to potentially sample (converted to string)
    fn update(&mut self, item: &str) {
        self.inner.update(item.to_string());
    }

    /// Get the current sample
    ///
    /// Returns:
    ///     list: List of sampled items (at most k items)
    fn sample<'py>(&self, py: Python<'py>) -> Bound<'py, PyList> {
        let items: Vec<&String> = self.inner.sample().iter().collect();
        PyList::new_bound(py, items)
    }

    /// Check if the reservoir is empty
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of items in the reservoir
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Maximum capacity of the reservoir
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Total number of items seen
    fn count(&self) -> u64 {
        self.inner.count()
    }

    /// Inclusion probability (k/n)
    ///
    /// Returns:
    ///     float: Probability that any given item is in the sample
    fn inclusion_probability(&self) -> f64 {
        self.inner.inclusion_probability()
    }

    /// Clear the reservoir
    fn clear(&mut self) {
        self.inner.clear();
    }

    fn __repr__(&self) -> String {
        format!(
            "ReservoirSampling(k={}, count={})",
            self.inner.capacity(),
            self.inner.count()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "ReservoirSampling: {} items in reservoir, {} items seen",
            self.inner.len(),
            self.inner.count()
        )
    }
}
