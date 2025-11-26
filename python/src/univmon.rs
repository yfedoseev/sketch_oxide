//! Python bindings for UnivMon - Universal Monitoring for Multiple Metrics

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::universal::UnivMon as RustUnivMon;

/// UnivMon: Universal Monitoring for Multiple Metrics (SIGCOMM 2016)
///
/// Revolutionary sketch that supports **6 simultaneous metrics** from a single
/// data structure: L1 norm, L2 norm, entropy, heavy hitters, change detection,
/// and flow size distribution. Eliminates the need for multiple specialized sketches.
///
/// Args:
///     max_stream_size (int): Expected maximum number of items (determines layers)
///     epsilon (float): Error parameter (0 < ε < 1, smaller = more accurate)
///     delta (float): Failure probability (0 < δ < 1, smaller = higher confidence)
///
/// Example:
///     >>> # Create UnivMon for 1M items
///     >>> um = UnivMon(max_stream_size=1_000_000, epsilon=0.01, delta=0.01)
///     >>> # Update with IP addresses and packet sizes
///     >>> um.update(b"192.168.1.1", 1500.0)
///     >>> um.update(b"192.168.1.2", 800.0)
///     >>> um.update(b"192.168.1.1", 1200.0)
///     >>> # Query MULTIPLE metrics from SAME sketch
///     >>> total_bytes = um.estimate_l1()      # Total traffic
///     >>> variability = um.estimate_l2()      # Load balance
///     >>> diversity = um.estimate_entropy()   # IP diversity
///     >>> top_ips = um.heavy_hitters(0.1)    # Top 10% sources
///
/// Key Innovation:
///     One sketch replaces 6+ specialized sketches, saving 5-10x memory
///
/// Supported Metrics (from ONE sketch!):
///     1. L1 Norm: Total traffic volume, sum of frequencies
///     2. L2 Norm: Traffic variability, load balance indicator
///     3. Entropy: Distribution diversity, uniformity measure
///     4. Heavy Hitters: Top-k most frequent items
///     5. Change Detection: Temporal anomalies, distribution shifts
///     6. Flow Distribution: Per-flow statistics
///
/// Production Use Cases:
///     - Network monitoring: Track bandwidth, flows, protocols simultaneously
///     - Cloud analytics: Unified telemetry across multiple dimensions
///     - Anomaly detection: Real-time traffic spikes, DDoS, data skew
///     - Multi-tenant systems: Per-tenant metrics without overhead
///     - System performance: CPU, memory, disk I/O from single structure
///
/// Performance:
///     - Update: O(d * log n) where d = sketch depth
///     - Query: O(d * log n) per metric
///     - Space: O((log n / ε²) * log(1/δ)) - logarithmic in stream size!
///     - Memory: 5-10x better than separate sketches
///
/// Mathematical Guarantees:
///     - L1/L2 error: O(ε * ||f||_2) with probability 1-δ
///     - Entropy error: O(ε * H) where H is true entropy
///     - Heavy hitters: All items ≥ ε * L1 are found
///
/// Notes:
///     - Uses hierarchical layers with exponential sampling
///     - Layer i has sampling rate 2^(-i)
///     - Supports byte slices for network flows, strings, binary data
///     - Can compare sketches for change detection
#[pyclass(module = "sketch_oxide")]
pub struct UnivMon {
    inner: RustUnivMon,
}

#[pymethods]
impl UnivMon {
    /// Create a new UnivMon sketch
    ///
    /// Args:
    ///     max_stream_size: Expected maximum items (determines layer count)
    ///     epsilon: Error parameter (0 < ε < 1, e.g., 0.01 for 1% error)
    ///     delta: Failure probability (0 < δ < 1, e.g., 0.01 for 99% confidence)
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    ///
    /// Example:
    ///     >>> um = UnivMon(max_stream_size=1_000_000, epsilon=0.01, delta=0.01)
    #[new]
    fn new(max_stream_size: u64, epsilon: f64, delta: f64) -> PyResult<Self> {
        RustUnivMon::new(max_stream_size, epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with an item and its value
    ///
    /// Uses hierarchical sampling across layers for efficient multi-metric estimation.
    ///
    /// Args:
    ///     item: Item to update (str, bytes, or int)
    ///     value: Value/weight for this item (e.g., packet size, count, amount)
    ///
    /// Raises:
    ///     ValueError: If value is negative
    ///
    /// Example:
    ///     >>> um = UnivMon(1_000_000, 0.01, 0.01)
    ///     >>> um.update(b"192.168.1.1", 1500.0)  # IP and packet size
    ///     >>> um.update("user_123", 99.99)       # User and transaction
    ///     >>> um.update(12345, 1.0)              # Item ID and count
    fn update(&mut self, item: &Bound<'_, PyAny>, value: f64) -> PyResult<()> {
        if let Ok(val) = item.extract::<i64>() {
            self.inner
                .update(&val.to_le_bytes(), value)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(val) = item.extract::<u64>() {
            self.inner
                .update(&val.to_le_bytes(), value)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(val) = item.extract::<String>() {
            self.inner
                .update(val.as_bytes(), value)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else if let Ok(b) = item.downcast::<PyBytes>() {
            self.inner
                .update(b.as_bytes(), value)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item must be int, str, or bytes",
            ))
        }
    }

    /// Estimate the L1 norm (sum of all frequencies)
    ///
    /// The L1 norm represents total "mass" in the stream.
    ///
    /// Returns:
    ///     float: Estimated L1 norm
    ///
    /// Applications:
    ///     - Network: Total bytes transferred
    ///     - E-commerce: Total revenue
    ///     - Logs: Total event count
    ///
    /// Example:
    ///     >>> um = UnivMon(10000, 0.01, 0.01)
    ///     >>> um.update(b"A", 100.0)
    ///     >>> um.update(b"B", 200.0)
    ///     >>> l1 = um.estimate_l1()  # ≈ 300.0
    fn estimate_l1(&self) -> f64 {
        self.inner.estimate_l1()
    }

    /// Estimate the L2 norm (sum of squared frequencies)
    ///
    /// The L2 norm measures spread/variability in the distribution.
    ///
    /// Returns:
    ///     float: Estimated L2 norm
    ///
    /// Applications:
    ///     - Network: Load balance across flows
    ///     - Databases: Query distribution uniformity
    ///     - Systems: Resource usage variance
    ///
    /// Example:
    ///     >>> um = UnivMon(10000, 0.01, 0.01)
    ///     >>> um.update(b"A", 100.0)
    ///     >>> um.update(b"B", 100.0)
    ///     >>> l2 = um.estimate_l2()  # ≈ 141.4 (sqrt(2) * 100)
    fn estimate_l2(&self) -> f64 {
        self.inner.estimate_l2()
    }

    /// Estimate Shannon entropy of the stream
    ///
    /// Entropy measures diversity/uniformity of the distribution.
    /// High entropy = uniform, Low entropy = skewed.
    ///
    /// Returns:
    ///     float: Estimated Shannon entropy in bits
    ///
    /// Applications:
    ///     - Network: Traffic diversity (detect DDoS, port scans)
    ///     - Security: Access pattern analysis
    ///     - Analytics: User behavior diversity
    ///
    /// Example:
    ///     >>> um = UnivMon(10000, 0.01, 0.01)
    ///     >>> for i in range(100):
    ///     ...     um.update(f"item_{i}".encode(), 1.0)
    ///     >>> entropy = um.estimate_entropy()  # ≈ 6.64 bits (log2(100))
    fn estimate_entropy(&self) -> f64 {
        self.inner.estimate_entropy()
    }

    /// Find heavy hitters (most frequent items)
    ///
    /// Returns items with frequency ≥ threshold * L1.
    ///
    /// Args:
    ///     threshold: Frequency threshold as fraction of L1 (e.g., 0.1 for top 10%)
    ///
    /// Returns:
    ///     list: List of (item_bytes, frequency) tuples, sorted by frequency descending
    ///
    /// Guarantee:
    ///     No false negatives - all items ≥ threshold are returned
    ///
    /// Example:
    ///     >>> um = UnivMon(10000, 0.01, 0.01)
    ///     >>> for _ in range(100):
    ///     ...     um.update(b"popular", 1.0)
    ///     >>> for _ in range(10):
    ///     ...     um.update(b"rare", 1.0)
    ///     >>> heavy = um.heavy_hitters(0.5)  # Items with >50% of traffic
    ///     >>> assert heavy[0][0] == b"popular"
    fn heavy_hitters(&self, threshold: f64) -> Vec<(Py<PyBytes>, f64)> {
        let results = self.inner.heavy_hitters(threshold);
        Python::with_gil(|py| {
            results
                .into_iter()
                .map(|(item, freq)| (PyBytes::new_bound(py, &item).into(), freq))
                .collect()
        })
    }

    /// Detect changes between two UnivMon sketches
    ///
    /// Measures magnitude of distribution change between time windows or data sources.
    ///
    /// Args:
    ///     other: Another UnivMon sketch to compare
    ///
    /// Returns:
    ///     float: Change magnitude (non-negative)
    ///         - 0.0: Identical distributions
    ///         - <1.0: Minor changes
    ///         - >10.0: Significant distribution shift
    ///
    /// Applications:
    ///     - Network: Traffic anomaly detection
    ///     - Security: Attack detection (DDoS, port scans)
    ///     - Systems: Performance degradation alerts
    ///
    /// Example:
    ///     >>> baseline = UnivMon(10000, 0.01, 0.01)
    ///     >>> current = UnivMon(10000, 0.01, 0.01)
    ///     >>> # ... update both with different data ...
    ///     >>> change = baseline.detect_change(current)
    ///     >>> if change > 10.0:
    ///     ...     print("Alert: Significant change detected!")
    fn detect_change(&self, other: &UnivMon) -> f64 {
        self.inner.detect_change(&other.inner)
    }

    /// Get statistics about the UnivMon structure
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - num_layers: Number of hierarchical layers
    ///         - samples_processed: Total samples processed
    ///         - total_memory: Total memory usage in bytes
    ///         - epsilon: Error parameter
    ///         - delta: Failure probability
    ///         - max_stream_size: Maximum expected stream size
    ///
    /// Example:
    ///     >>> um = UnivMon(1_000_000, 0.01, 0.01)
    ///     >>> stats = um.stats()
    ///     >>> print(f"Layers: {stats['num_layers']}, Updates: {stats['samples_processed']}")
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("num_layers", stats.num_layers)?;
            dict.set_item("samples_processed", stats.samples_processed)?;
            dict.set_item("total_memory", stats.total_memory)?;
            dict.set_item("epsilon", self.inner.epsilon())?;
            dict.set_item("delta", self.inner.delta())?;
            dict.set_item("max_stream_size", self.inner.max_stream_size())?;
            Ok(dict.into())
        })
    }

    fn __repr__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "UnivMon(layers={}, updates={}, epsilon={:.3}, delta={:.3})",
            stats.num_layers,
            stats.samples_processed,
            self.inner.epsilon(),
            self.inner.delta()
        )
    }

    fn __str__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "UnivMon({} layers, {} updates)",
            stats.num_layers, stats.samples_processed
        )
    }
}
