//! Python bindings for Elastic Sketch frequency estimation

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use sketch_oxide::frequency::ElasticSketch as RustElasticSketch;
use sketch_oxide::Sketch;

/// Elastic Sketch for frequency estimation and heavy hitter detection
///
/// A state-of-the-art algorithm that combines Count-Min Sketch efficiency with elastic
/// counters that adapt based on observed frequency distribution. Provides accurate frequency
/// estimates for items in data streams with minimal space overhead.
///
/// Args:
///     bucket_count (int): Number of buckets per row (will be rounded to next power of 2).
///         - 256-512: Low memory (small datasets)
///         - 512-1024: Balanced (recommended)
///         - 2048+: High accuracy (large datasets)
///     depth (int): Number of hash functions (2-5 recommended).
///         - 2-3: Balanced accuracy and speed
///         - 4-5: Higher accuracy
///
/// Example:
///     >>> sketch = ElasticSketch(bucket_count=512, depth=3)
///     >>> sketch.update("flow1", 100)
///     >>> sketch.update("flow2", 50)
///     >>> print(sketch.estimate("flow1"))  # >= 100
///     >>> hitters = sketch.heavy_hitters(threshold=30)
///     >>> for item_hash, freq in hitters:
///     ...     print(f"Heavy hitter: {freq}")
///
/// Notes:
///     - Supports binary data (bytes) for item identification
///     - Optimized for network traffic and streaming applications
///     - Space: O(bucket_count * depth * bucket_size)
///     - Time: O(depth) per update/estimate operation
#[pyclass(module = "sketch_oxide")]
pub struct ElasticSketch {
    inner: RustElasticSketch,
}

#[pymethods]
impl ElasticSketch {
    /// Create a new Elastic Sketch with default elastic ratio (0.2)
    ///
    /// Args:
    ///     bucket_count (int): Number of buckets per row
    ///     depth (int): Number of hash functions
    ///
    /// Returns:
    ///     ElasticSketch: A new sketch instance
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    #[new]
    fn new(bucket_count: usize, depth: usize) -> PyResult<Self> {
        RustElasticSketch::new(bucket_count, depth)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Create a new Elastic Sketch with custom elastic ratio
    ///
    /// Args:
    ///     bucket_count (int): Number of buckets per row
    ///     depth (int): Number of hash functions
    ///     elastic_ratio (float): Elastic expansion ratio (0.0 to 1.0)
    ///         - 0.0-0.3: More aggressive optimization, better space efficiency
    ///         - 0.5: Balanced (default is 0.2)
    ///         - 0.7-1.0: More conservative, better accuracy
    ///
    /// Returns:
    ///     ElasticSketch: A new sketch instance with custom parameters
    ///
    /// Raises:
    ///     ValueError: If parameters are invalid
    #[staticmethod]
    fn with_elastic_ratio(bucket_count: usize, depth: usize, elastic_ratio: f64) -> PyResult<Self> {
        RustElasticSketch::with_elastic_ratio(bucket_count, depth, elastic_ratio)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Update the sketch with an item and its count
    ///
    /// Args:
    ///     item (bytes): The item identifier (flow ID, URL, feature, etc.)
    ///     count (int): The frequency/weight to add (default 1)
    ///
    /// Example:
    ///     >>> sketch = ElasticSketch(512, 3)
    ///     >>> sketch.update(b"flow1", 1)
    ///     >>> sketch.update(b"flow1", 2)  # Can add multiple counts
    fn update(&mut self, item: &[u8], count: u64) {
        self.inner.update(item, count);
    }

    /// Estimate the frequency of an item
    ///
    /// Args:
    ///     item (bytes): The item to query
    ///
    /// Returns:
    ///     int: Estimated frequency (0 if item not found in sketch)
    ///
    /// Example:
    ///     >>> sketch = ElasticSketch(512, 3)
    ///     >>> sketch.update(b"item", 5)
    ///     >>> assert sketch.estimate(b"item") == 5
    fn estimate(&self, item: &[u8]) -> u64 {
        self.inner.estimate(item)
    }

    /// Find all items with frequency >= threshold
    ///
    /// Returns items sorted by frequency in descending order.
    ///
    /// Args:
    ///     threshold (int): Minimum frequency threshold
    ///
    /// Returns:
    ///     list[tuple[int, int]]: List of (item_hash, frequency) tuples
    ///
    /// Example:
    ///     >>> sketch = ElasticSketch(512, 3)
    ///     >>> sketch.update(b"heavy1", 100)
    ///     >>> sketch.update(b"light", 1)
    ///     >>> hitters = sketch.heavy_hitters(threshold=50)
    ///     >>> assert len(hitters) >= 1
    fn heavy_hitters(&self, threshold: u64) -> PyResult<Vec<(u64, u64)>> {
        Ok(self.inner.heavy_hitters(threshold))
    }

    /// Merge another Elastic Sketch into this one
    ///
    /// Both sketches must have compatible parameters (same bucket_count, depth, elastic_ratio).
    ///
    /// Args:
    ///     other (ElasticSketch): The sketch to merge into this one
    ///
    /// Raises:
    ///     ValueError: If sketches are incompatible
    ///
    /// Example:
    ///     >>> sketch1 = ElasticSketch(512, 3)
    ///     >>> sketch2 = ElasticSketch(512, 3)
    ///     >>> sketch1.update(b"item", 5)
    ///     >>> sketch2.update(b"item", 3)
    ///     >>> sketch1.merge(sketch2)
    ///     >>> assert sketch1.estimate(b"item") == 8
    fn merge(&mut self, other: &ElasticSketch) -> PyResult<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Clear all state from the sketch
    ///
    /// Resets all buckets and counters to empty state.
    ///
    /// Example:
    ///     >>> sketch = ElasticSketch(512, 3)
    ///     >>> sketch.update(b"item", 5)
    ///     >>> sketch.reset()
    ///     >>> assert sketch.is_empty()
    fn reset(&mut self) {
        self.inner.reset();
    }

    /// Get the number of buckets per row
    ///
    /// Returns:
    ///     int: Bucket count
    fn bucket_count(&self) -> usize {
        self.inner.bucket_count()
    }

    /// Get the depth (number of hash functions)
    ///
    /// Returns:
    ///     int: Depth parameter
    fn depth(&self) -> usize {
        self.inner.depth()
    }

    /// Get the elastic ratio
    ///
    /// Returns:
    ///     float: Elastic expansion ratio (0.0 to 1.0)
    fn elastic_ratio(&self) -> f64 {
        self.inner.elastic_ratio()
    }

    /// Get the total count of all updates
    ///
    /// Returns:
    ///     int: Sum of all update counts
    fn total_count(&self) -> u64 {
        self.inner.total_count()
    }

    /// Check if the sketch is empty
    ///
    /// Returns:
    ///     bool: True if no items have been added
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get memory usage in bytes
    ///
    /// Returns:
    ///     int: Approximate memory usage
    ///
    /// Example:
    ///     >>> sketch = ElasticSketch(512, 3)
    ///     >>> mem = sketch.memory_usage()
    ///     >>> print(f"Memory: {mem} bytes")
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Serialize the sketch to bytes
    ///
    /// Returns:
    ///     bytes: Serialized sketch data
    ///
    /// Example:
    ///     >>> sketch = ElasticSketch(512, 3)
    ///     >>> sketch.update(b"item", 5)
    ///     >>> data = sketch.serialize()
    ///     >>> restored = ElasticSketch.deserialize(data)
    fn serialize<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new_bound(py, &self.inner.serialize())
    }

    /// Deserialize a sketch from bytes
    ///
    /// Args:
    ///     data (bytes): Serialized sketch data
    ///
    /// Returns:
    ///     ElasticSketch: Restored sketch instance
    ///
    /// Raises:
    ///     ValueError: If data is corrupted or invalid
    #[staticmethod]
    fn deserialize(data: &[u8]) -> PyResult<Self> {
        RustElasticSketch::deserialize(data)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "ElasticSketch(bucket_count={}, depth={}, elastic_ratio={:.2}, total_count={})",
            self.inner.bucket_count(),
            self.inner.depth(),
            self.inner.elastic_ratio(),
            self.inner.total_count()
        )
    }

    fn __str__(&self) -> String {
        format!(
            "ElasticSketch({}x{} with ratio {:.2})",
            self.inner.depth(),
            self.inner.bucket_count(),
            self.inner.elastic_ratio()
        )
    }
}
