//! Python bindings for Learned Bloom Filter - ML-Enhanced Membership Testing

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyList};
use sketch_oxide::membership::LearnedBloomFilter as RustLearnedBloomFilter;

/// LearnedBloomFilter: ML-Enhanced Membership Testing (EXPERIMENTAL)
///
/// **EXPERIMENTAL FEATURE** - Use with caution in production systems.
///
/// A Learned Bloom Filter uses machine learning to predict set membership,
/// achieving 70-80% memory reduction compared to standard Bloom filters.
/// The ML model learns patterns in the keys, allowing it to compress
/// information more efficiently than hash-based methods.
///
/// Args:
///     training_keys (list): Keys to train the model on (must be members)
///         - Must contain at least 10 keys
///         - Supports bytes, str, and int types
///     fpr (float): Target false positive rate (0.0 < fpr < 1.0)
///
/// Example:
///     >>> # Train on a dataset of URLs
///     >>> training_keys = [f"https://example.com/page{i}".encode() for i in range(1000)]
///     >>> lbf = LearnedBloomFilter(training_keys, fpr=0.01)
///     >>> assert lbf.contains(b"https://example.com/page500")
///     >>> # 70-80% memory savings vs standard Bloom filter!
///
/// How It Works:
///     1. Feature Extraction: Extract features from keys (hash patterns)
///     2. ML Model: Simple linear model learns key patterns
///     3. Backup Filter: Small Bloom filter guarantees zero false negatives
///     4. Query: Model predicts; backup filter ensures correctness
///
/// Memory Savings:
///     - Traditional Bloom: ~10 bits/element at 1% FPR
///     - Learned Bloom: ~3-4 bits/element (70-80% reduction!)
///     - Model is tiny (few KB), backup filter is small
///
/// Security Warning:
///     ML models can be adversarially attacked. Do NOT use in security-critical
///     applications where an attacker could craft keys to fool the model.
///
/// Key Advantages:
///     - 70-80% memory reduction vs standard Bloom filters
///     - Zero false negatives (guaranteed by backup filter)
///     - Deterministic training (reproducible results)
///     - Fast queries (model prediction is very fast)
///     - Works best with structured data (URLs, IPs, etc.)
///
/// Limitations:
///     - EXPERIMENTAL: Not battle-tested in production
///     - Vulnerable to adversarial attacks (security concern)
///     - Requires training data (static filter)
///     - Best for structured/patterned data
///     - Memory savings vary with data distribution
///
/// Performance:
///     - Training: O(n * d) where n = keys, d = feature dimension
///     - Query: O(d) for feature extraction + O(1) model prediction
///     - Memory: 3-4 bits/element (vs 10 bits for standard Bloom)
///
/// Notes:
///     - Model learns from training keys only (static after construction)
///     - Supports int, str, and bytes types
///     - No false negatives (all training keys will be found)
///     - May have false positives at configured FPR
#[pyclass(module = "sketch_oxide")]
pub struct LearnedBloomFilter {
    inner: RustLearnedBloomFilter,
}

#[pymethods]
impl LearnedBloomFilter {
    /// Create a new Learned Bloom Filter
    ///
    /// Args:
    ///     training_keys: List of keys to train on (bytes, str, or int)
    ///     fpr: Target false positive rate (0.0 < fpr < 1.0)
    ///
    /// Raises:
    ///     ValueError: If training_keys is empty or too small (<10 keys)
    ///     ValueError: If fpr is invalid
    ///     TypeError: If keys are not bytes, str, or int
    ///
    /// Example:
    ///     >>> keys = [f"key{i}".encode() for i in range(1000)]
    ///     >>> lbf = LearnedBloomFilter(keys, fpr=0.01)
    #[new]
    fn new(training_keys: &Bound<'_, PyList>, fpr: f64) -> PyResult<Self> {
        // Convert Python list to Vec<Vec<u8>>
        let mut keys: Vec<Vec<u8>> = Vec::new();

        for item in training_keys {
            if let Ok(b) = item.downcast::<PyBytes>() {
                keys.push(b.as_bytes().to_vec());
            } else if let Ok(s) = item.extract::<String>() {
                keys.push(s.into_bytes());
            } else if let Ok(i) = item.extract::<i64>() {
                keys.push(i.to_le_bytes().to_vec());
            } else if let Ok(u) = item.extract::<u64>() {
                keys.push(u.to_le_bytes().to_vec());
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "Training keys must be bytes, str, or int",
                ));
            }
        }

        RustLearnedBloomFilter::new(&keys, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if a key might be in the filter
    ///
    /// Args:
    ///     key: Key to check (bytes, str, or int)
    ///
    /// Returns:
    ///     bool: True if might be present (or false positive),
    ///           False if definitely not present
    ///
    /// Guarantees:
    ///     - Zero false negatives: All training keys return True
    ///     - False positive rate approximately matches target FPR
    ///
    /// Example:
    ///     >>> keys = [b"hello", b"world"]
    ///     >>> # Need at least 10 keys for training
    ///     >>> keys += [f"key{i}".encode() for i in range(10)]
    ///     >>> lbf = LearnedBloomFilter(keys, fpr=0.01)
    ///     >>> assert lbf.contains(b"hello")  # True positive
    ///     >>> lbf.contains(b"other")  # May be false positive
    fn contains(&self, key: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(b) = key.downcast::<PyBytes>() {
            Ok(self.inner.contains(b.as_bytes()))
        } else if let Ok(s) = key.extract::<String>() {
            Ok(self.inner.contains(s.as_bytes()))
        } else if let Ok(i) = key.extract::<i64>() {
            Ok(self.inner.contains(&i.to_le_bytes()))
        } else if let Ok(u) = key.extract::<u64>() {
            Ok(self.inner.contains(&u.to_le_bytes()))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Key must be bytes, str, or int",
            ))
        }
    }

    /// Get memory usage in bytes
    ///
    /// Includes model weights, backup filter, and feature extractor metadata.
    ///
    /// Returns:
    ///     int: Total memory usage in bytes
    ///
    /// Example:
    ///     >>> keys = [f"key{i}".encode() for i in range(1000)]
    ///     >>> lbf = LearnedBloomFilter(keys, fpr=0.01)
    ///     >>> mem = lbf.memory_usage()
    ///     >>> print(f"Memory: {mem} bytes ({mem * 8 / len(keys):.1f} bits/key)")
    fn memory_usage(&self) -> usize {
        self.inner.memory_usage()
    }

    /// Get the target false positive rate
    ///
    /// Returns:
    ///     float: Target FPR
    fn fpr(&self) -> f64 {
        self.inner.fpr()
    }

    /// Get statistics about the filter
    ///
    /// Returns:
    ///     dict: Dictionary with statistics
    ///         - model_accuracy: Model accuracy on training data
    ///         - backup_fpr: Backup filter false positive rate
    ///         - memory_bits: Total memory in bits
    ///         - false_negative_rate: Always 0.0 (guaranteed)
    ///
    /// Example:
    ///     >>> keys = [f"key{i}".encode() for i in range(1000)]
    ///     >>> lbf = LearnedBloomFilter(keys, fpr=0.01)
    ///     >>> stats = lbf.stats()
    ///     >>> print(f"Memory: {stats['memory_bits'] // 8} bytes")
    ///     >>> print(f"Model accuracy: {stats['model_accuracy']:.1%}")
    ///     >>> print(f"FNR: {stats['false_negative_rate']}")  # Always 0.0
    fn stats(&self) -> PyResult<Py<PyAny>> {
        let stats = self.inner.stats();
        Python::with_gil(|py| {
            let dict = pyo3::types::PyDict::new_bound(py);
            dict.set_item("model_accuracy", stats.model_accuracy)?;
            dict.set_item("backup_fpr", stats.backup_fpr)?;
            dict.set_item("memory_bits", stats.memory_bits)?;
            dict.set_item("false_negative_rate", stats.false_negative_rate)?;
            Ok(dict.into())
        })
    }

    fn __repr__(&self) -> String {
        let mem = self.inner.memory_usage();
        let fpr = self.inner.fpr();
        format!("LearnedBloomFilter(fpr={:.3}, memory={}B)", fpr, mem)
    }

    fn __str__(&self) -> String {
        let mem = self.inner.memory_usage();
        format!("LearnedBloomFilter({} bytes, EXPERIMENTAL)", mem)
    }
}
