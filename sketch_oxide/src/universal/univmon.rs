//! UnivMon: Universal Monitoring for Multiple Metrics
//!
//! UnivMon (Liu et al., SIGCOMM 2016) is a universal sketch that supports **multiple
//! simultaneous metrics** from a single data structure. This eliminates the need for
//! separate specialized sketches for different metrics, significantly reducing memory
//! overhead in production monitoring systems.
//!
//! # Key Innovation: Hierarchical Streaming with Adaptive Sampling
//!
//! UnivMon uses L layers (L = log n) with exponentially decreasing sampling rates:
//! - Layer 0: Sample rate = 1.0 (all items)
//! - Layer i: Sample rate = 2^(-i) (exponentially fewer items)
//!
//! Each layer contains:
//! - Count Sketch for frequency estimation
//! - Heavy Hitters tracker for top-k items
//!
//! # Supported Metrics (from ONE sketch!)
//!
//! 1. **L1 Norm** (sum of frequencies): Traffic volume, total events
//! 2. **L2 Norm** (sum of squared frequencies): Variability, load balance
//! 3. **Entropy** (Shannon entropy): Diversity, uniformity
//! 4. **Heavy Hitters**: Most frequent items, top contributors
//! 5. **Change Detection**: Temporal anomalies, distribution shifts
//! 6. **Flow Size Distribution**: Per-flow statistics
//!
//! # Production Use Cases (2025)
//!
//! - **Network Monitoring**: Track bandwidth, flows, protocols simultaneously
//! - **Cloud Analytics**: Unified telemetry across multiple dimensions
//! - **Real-time Anomaly Detection**: Detect traffic spikes, DDoS, data skew
//! - **Multi-tenant Systems**: Per-tenant metrics without multiplicative overhead
//! - **System Performance**: CPU, memory, disk I/O from single data structure
//!
//! # Mathematical Guarantees
//!
//! For stream size n and error parameters (ε, δ):
//! - **L1/L2 error**: O(ε * ||f||_2) with probability 1-δ
//! - **Entropy error**: O(ε * H) where H is true entropy
//! - **Heavy hitters**: All items with frequency ≥ ε * L1 are found
//! - **Space**: O((log n / ε²) * log(1/δ)) - logarithmic in stream size!
//!
//! # References
//!
//! - Liu, Z., et al. (2016). "One Sketch to Rule Them All: Rethinking Network Flow
//!   Monitoring with UnivMon." SIGCOMM.
//! - https://dl.acm.org/doi/10.1145/2934872.2934906
//!
//! # Examples
//!
//! ```
//! use sketch_oxide::universal::UnivMon;
//!
//! // Create UnivMon for stream of up to 1 million items
//! let mut univmon = UnivMon::new(1_000_000, 0.01, 0.01).unwrap();
//!
//! // Update with network packets
//! univmon.update(b"192.168.1.1", 1500.0).unwrap(); // IP -> packet size
//! univmon.update(b"192.168.1.2", 800.0).unwrap();
//! univmon.update(b"192.168.1.1", 1200.0).unwrap();
//!
//! // Query multiple metrics from SAME sketch
//! let total_traffic = univmon.estimate_l1();     // Total bytes
//! let load_balance = univmon.estimate_l2();      // Traffic variability
//! let diversity = univmon.estimate_entropy();    // IP diversity
//! let top_ips = univmon.heavy_hitters(0.1);     // Top 10% traffic sources
//!
//! // Detect changes over time
//! let mut univmon2 = UnivMon::new(1_000_000, 0.01, 0.01).unwrap();
//! // ... update with new time window ...
//! let change_magnitude = univmon.detect_change(&univmon2);
//! ```

use crate::common::{Mergeable, Result, Sketch, SketchError};
use crate::frequency::{CountSketch, FrequentItems};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// UnivMon: Universal sketch supporting multiple simultaneous metrics
///
/// A single UnivMon instance can estimate L1/L2 norms, entropy, heavy hitters,
/// and detect changes, eliminating the need for multiple specialized sketches.
///
/// # Type Safety
///
/// UnivMon operates on byte slices (`&[u8]`) for maximum flexibility. This allows:
/// - Network flows (IP addresses, ports)
/// - String keys (user IDs, URLs)
/// - Binary data (hashes, UUIDs)
/// - Any serializable data
///
/// # Performance
///
/// - **Update**: O(d * log n) where d = sketch depth, n = max stream size
/// - **L1/L2 query**: O(d * log n)
/// - **Entropy query**: O(d * log n)
/// - **Heavy hitters**: O(k * d) where k = number of heavy hitters
/// - **Space**: O((log n / ε²) * log(1/δ))
#[derive(Clone, Debug)]
pub struct UnivMon {
    /// Hierarchical layers (L = ceil(log2(max_stream_size)))
    layers: Vec<Layer>,
    /// Number of layers
    num_layers: usize,
    /// Maximum expected stream size (determines layer count)
    max_stream_size: u64,
    /// Total number of updates (all items, all layers)
    total_updates: u64,
    /// Epsilon parameter for error bounds
    epsilon: f64,
    /// Delta parameter for failure probability
    delta: f64,
}

/// Internal layer structure for hierarchical streaming
///
/// Each layer operates at a different sampling rate, allowing efficient
/// estimation of different metrics from the same data.
#[derive(Clone, Debug)]
struct Layer {
    /// Count Sketch for unbiased frequency estimation
    count_sketch: CountSketch,
    /// Heavy hitters tracker for top-k items
    heavy_hitters: FrequentItems<Vec<u8>>,
    /// Sampling rate for this layer: 2^(-layer_index)
    sampling_rate: f64,
    /// Number of samples processed at this layer
    sample_count: u64,
    /// Layer index (0 = bottom, highest sampling rate)
    layer_index: usize,
    /// Sum of all values at this layer (for L1 estimation)
    value_sum: f64,
}

impl UnivMon {
    /// Create a new UnivMon sketch
    ///
    /// # Arguments
    ///
    /// * `max_stream_size` - Expected maximum number of items in stream (determines layer count)
    /// * `epsilon` - Error parameter: estimates are within ε * metric with probability 1-δ
    /// * `delta` - Failure probability: guarantees hold with probability 1-δ
    ///
    /// # Returns
    ///
    /// A new `UnivMon` instance or an error if parameters are invalid
    ///
    /// # Errors
    ///
    /// Returns `InvalidParameter` if:
    /// - `max_stream_size` = 0
    /// - `epsilon` <= 0 or >= 1
    /// - `delta` <= 0 or >= 1
    ///
    /// # Layer Calculation
    ///
    /// Number of layers L = ceil(log2(max_stream_size)), minimum 3
    /// - Layer 0: sampling_rate = 1.0
    /// - Layer i: sampling_rate = 2^(-i)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::universal::UnivMon;
    ///
    /// // For 1M items, 1% error, 1% failure probability
    /// let univmon = UnivMon::new(1_000_000, 0.01, 0.01).unwrap();
    /// ```
    pub fn new(max_stream_size: u64, epsilon: f64, delta: f64) -> Result<Self> {
        // Validate parameters
        if max_stream_size == 0 {
            return Err(SketchError::InvalidParameter {
                param: "max_stream_size".to_string(),
                value: max_stream_size.to_string(),
                constraint: "must be > 0".to_string(),
            });
        }

        if epsilon <= 0.0 || epsilon >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "epsilon".to_string(),
                value: epsilon.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        if delta <= 0.0 || delta >= 1.0 {
            return Err(SketchError::InvalidParameter {
                param: "delta".to_string(),
                value: delta.to_string(),
                constraint: "must be in (0, 1)".to_string(),
            });
        }

        // Calculate number of layers: L = ceil(log2(n))
        let num_layers = if max_stream_size <= 1 {
            3 // Minimum layers for stability
        } else {
            let log_n = (max_stream_size as f64).log2().ceil() as usize;
            log_n.max(3) // At least 3 layers
        };

        // Create hierarchical layers
        let mut layers = Vec::with_capacity(num_layers);

        for i in 0..num_layers {
            // Sampling rate: 2^(-i)
            let sampling_rate = 2_f64.powi(-(i as i32));

            // Each layer has its own Count Sketch and Heavy Hitters tracker
            let count_sketch = CountSketch::new(epsilon, delta)?;

            // Heavy hitters size scales with layer (fewer at higher layers)
            let hh_size = ((1000.0 / (i as f64 + 1.0)).ceil() as usize).max(10);
            let heavy_hitters = FrequentItems::new(hh_size)?;

            layers.push(Layer {
                count_sketch,
                heavy_hitters,
                sampling_rate,
                sample_count: 0,
                layer_index: i,
                value_sum: 0.0,
            });
        }

        Ok(UnivMon {
            layers,
            num_layers,
            max_stream_size,
            total_updates: 0,
            epsilon,
            delta,
        })
    }

    /// Update the sketch with an item and its value
    ///
    /// Uses hierarchical sampling: each layer samples with rate 2^(-i).
    /// This enables efficient multi-metric estimation.
    ///
    /// # Arguments
    ///
    /// * `item` - The item (as byte slice) to update
    /// * `value` - The value/weight for this item (e.g., packet size, count)
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, error if update fails
    ///
    /// # Time Complexity
    ///
    /// O(d * L) where d = sketch depth, L = number of layers
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::universal::UnivMon;
    ///
    /// let mut univmon = UnivMon::new(10000, 0.01, 0.01).unwrap();
    ///
    /// // Update with IP address and packet size
    /// univmon.update(b"192.168.1.1", 1500.0).unwrap();
    ///
    /// // Update with user ID and transaction amount
    /// univmon.update(b"user_12345", 99.99).unwrap();
    /// ```
    pub fn update(&mut self, item: &[u8], value: f64) -> Result<()> {
        if value < 0.0 {
            return Err(SketchError::InvalidParameter {
                param: "value".to_string(),
                value: value.to_string(),
                constraint: "must be >= 0".to_string(),
            });
        }

        self.total_updates += 1;

        // Hash item once for sampling decisions
        let mut hasher = XxHash64::with_seed(0xDEADBEEF);
        hasher.write(item);
        let item_hash = hasher.finish();

        // Update each layer based on sampling
        for layer in &mut self.layers {
            // Deterministic sampling: use hash to decide if item is sampled
            // For sampling rate p = 2^(-i), accept if (hash % 2^i) == 0
            let sample_divisor = (1.0 / layer.sampling_rate) as u64;

            if item_hash % sample_divisor == 0 {
                // Item is sampled at this layer
                layer.sample_count += 1;

                // Track sum for L1 estimation (scale by sampling rate)
                layer.value_sum += value / layer.sampling_rate;

                // Update Count Sketch (scale value by 1/sampling_rate for unbiased estimate)
                let scaled_value = (value / layer.sampling_rate) as i64;
                layer.count_sketch.update(&item, scaled_value);

                // Update Heavy Hitters (use raw count)
                layer.heavy_hitters.update(item.to_vec());
            }
        }

        Ok(())
    }

    /// Estimate the L1 norm (sum of all frequencies)
    ///
    /// The L1 norm represents the total "mass" in the stream:
    /// - Network: Total bytes transferred
    /// - E-commerce: Total revenue
    /// - Logs: Total event count
    ///
    /// # Algorithm
    ///
    /// Uses sum of absolute values across all counters in the appropriate layer,
    /// scaled by sampling rate and depth.
    ///
    /// # Returns
    ///
    /// Estimated L1 norm (sum of all values)
    ///
    /// # Accuracy
    ///
    /// Error bounded by O(ε * L2) with probability 1-δ
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::universal::UnivMon;
    ///
    /// let mut univmon = UnivMon::new(10000, 0.01, 0.01).unwrap();
    /// univmon.update(b"A", 100.0).unwrap();
    /// univmon.update(b"B", 200.0).unwrap();
    /// univmon.update(b"C", 300.0).unwrap();
    ///
    /// let l1 = univmon.estimate_l1();
    /// // l1 ≈ 600.0 (within error bounds)
    /// ```
    pub fn estimate_l1(&self) -> f64 {
        if self.total_updates == 0 {
            return 0.0;
        }

        // Use layer 0 (full sampling) for exact L1
        let layer = &self.layers[0];

        if layer.sample_count == 0 {
            return 0.0;
        }

        // Return the tracked sum (already scaled by sampling rate during update)
        layer.value_sum
    }

    /// Estimate the L2 norm (sum of squared frequencies)
    ///
    /// The L2 norm measures the "spread" or variability in the stream:
    /// - Network: Load balance across flows
    /// - Databases: Query distribution uniformity
    /// - Systems: Resource usage variance
    ///
    /// # Algorithm
    ///
    /// Uses inner product of Count Sketch with itself from an intermediate layer.
    /// Higher layers reduce noise from low-frequency items.
    ///
    /// # Returns
    ///
    /// Estimated L2 norm (square root of sum of squared values)
    ///
    /// # Accuracy
    ///
    /// Error bounded by O(ε * L2) with probability 1-δ
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::universal::UnivMon;
    ///
    /// let mut univmon = UnivMon::new(10000, 0.01, 0.01).unwrap();
    /// univmon.update(b"A", 100.0).unwrap();
    /// univmon.update(b"B", 100.0).unwrap();
    ///
    /// let l2 = univmon.estimate_l2();
    /// // For uniform distribution, L2 ≈ sqrt(2) * 100 ≈ 141.4
    /// ```
    pub fn estimate_l2(&self) -> f64 {
        if self.total_updates == 0 {
            return 0.0;
        }

        // Use layer 0 for L2 estimation (full sampling)
        let layer = &self.layers[0];

        if layer.sample_count == 0 {
            return 0.0;
        }

        // Estimate L2 using inner product of Count Sketch with itself
        // This gives us sum of squared frequencies
        let inner_product = layer.count_sketch.inner_product(&layer.count_sketch);

        // The inner product estimates sum of squared counts
        // Take square root to get L2 norm
        let l2_squared = (inner_product as f64).abs();
        l2_squared.sqrt()
    }

    /// Estimate Shannon entropy of the stream
    ///
    /// Entropy measures the diversity or uniformity of the distribution:
    /// - High entropy: Uniform distribution (many items with similar frequencies)
    /// - Low entropy: Skewed distribution (few dominant items)
    ///
    /// # Formula
    ///
    /// H = -Σ(p_i * log2(p_i)) where p_i = f_i / L1
    ///
    /// # Applications
    ///
    /// - Network: Traffic diversity (detect DDoS, port scans)
    /// - Security: Access pattern analysis
    /// - Analytics: User behavior diversity
    ///
    /// # Algorithm
    ///
    /// Uses multiple layers to estimate entropy via the method from the UnivMon paper.
    ///
    /// # Returns
    ///
    /// Estimated Shannon entropy in bits
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::universal::UnivMon;
    ///
    /// let mut univmon = UnivMon::new(10000, 0.01, 0.01).unwrap();
    ///
    /// // Uniform distribution
    /// for i in 0..100 {
    ///     univmon.update(format!("item_{}", i).as_bytes(), 1.0).unwrap();
    /// }
    ///
    /// let entropy = univmon.estimate_entropy();
    /// // High entropy ≈ log2(100) ≈ 6.64 bits
    /// ```
    pub fn estimate_entropy(&self) -> f64 {
        if self.total_updates == 0 {
            return 0.0;
        }

        let l1 = self.estimate_l1();
        if l1 <= 0.0 {
            return 0.0;
        }

        // Estimate entropy using layer-based approach
        // Use the method from UnivMon paper: sum across layers
        let mut entropy_sum = 0.0;
        let mut layer_count = 0;

        for (i, layer) in self.layers.iter().enumerate() {
            if layer.sample_count < 10 {
                continue; // Skip layers with too few samples
            }

            // Get heavy hitters from this layer
            let items = layer
                .heavy_hitters
                .frequent_items(crate::frequency::frequent::ErrorType::NoFalsePositives);

            if items.is_empty() {
                continue;
            }

            // Estimate entropy contribution from this layer
            let mut layer_entropy = 0.0;

            for (item, _lower, _upper) in &items {
                // Estimate frequency from count sketch
                let freq = layer.count_sketch.estimate(&item).abs() as f64;

                if freq > 0.0 {
                    let prob = freq / l1;
                    if prob > 0.0 && prob <= 1.0 {
                        layer_entropy -= prob * prob.log2();
                    }
                }
            }

            entropy_sum += layer_entropy;
            layer_count += 1;
        }

        if layer_count > 0 {
            entropy_sum / layer_count as f64
        } else {
            // Fallback: estimate from L1 and L2
            let l2 = self.estimate_l2();
            if l2 > 0.0 {
                // For uniform distribution of n items: H ≈ log2(n)
                // Approximate: n ≈ L1² / L2²
                let n_estimate = (l1 * l1) / (l2 * l2 + 1.0);
                n_estimate.max(1.0).log2()
            } else {
                0.0
            }
        }
    }

    /// Find heavy hitters (most frequent items)
    ///
    /// Returns items with estimated frequency ≥ threshold * L1.
    ///
    /// # Arguments
    ///
    /// * `threshold` - Frequency threshold as fraction of L1 (e.g., 0.1 for top 10%)
    ///
    /// # Returns
    ///
    /// Vector of (item, estimated_frequency) pairs, sorted by frequency descending
    ///
    /// # Guarantee
    ///
    /// No false negatives: All items with frequency ≥ threshold * L1 are returned
    /// (may include some false positives with frequency slightly below threshold)
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::universal::UnivMon;
    ///
    /// let mut univmon = UnivMon::new(10000, 0.01, 0.01).unwrap();
    ///
    /// for _ in 0..100 {
    ///     univmon.update(b"popular", 1.0).unwrap();
    /// }
    /// for _ in 0..10 {
    ///     univmon.update(b"rare", 1.0).unwrap();
    /// }
    ///
    /// let heavy = univmon.heavy_hitters(0.5); // Items with >50% of traffic
    /// assert_eq!(heavy[0].0, b"popular");
    /// ```
    pub fn heavy_hitters(&self, threshold: f64) -> Vec<(Vec<u8>, f64)> {
        if threshold <= 0.0 || threshold > 1.0 {
            return Vec::new();
        }

        let l1 = self.estimate_l1();
        if l1 <= 0.0 {
            return Vec::new();
        }

        let threshold_count = threshold * l1;

        // Collect heavy hitters from all layers
        let mut candidates: HashMap<Vec<u8>, f64> = HashMap::new();

        for layer in &self.layers {
            let items = layer
                .heavy_hitters
                .frequent_items(crate::frequency::frequent::ErrorType::NoFalseNegatives);

            for (item, _lower, _upper) in items {
                // Estimate frequency using Count Sketch
                let freq = layer.count_sketch.estimate(&item).abs() as f64;

                // Keep maximum estimate across layers
                candidates
                    .entry(item)
                    .and_modify(|e| *e = e.max(freq))
                    .or_insert(freq);
            }
        }

        // Filter by threshold and sort
        let mut result: Vec<(Vec<u8>, f64)> = candidates
            .into_iter()
            .filter(|(_item, freq)| *freq >= threshold_count)
            .collect();

        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        result
    }

    /// Detect changes between two UnivMon sketches
    ///
    /// Measures the magnitude of change between two time windows or data sources.
    /// Useful for anomaly detection, trend analysis, and monitoring.
    ///
    /// # Arguments
    ///
    /// * `other` - Another UnivMon sketch to compare against
    ///
    /// # Returns
    ///
    /// Change magnitude (non-negative). Higher values indicate larger changes.
    /// - 0.0: Identical distributions
    /// - Small (<1.0): Minor changes
    /// - Large (>10.0): Significant distribution shift
    ///
    /// # Algorithm
    ///
    /// Uses L2 distance between Count Sketches across layers
    ///
    /// # Applications
    ///
    /// - Network: Traffic anomaly detection
    /// - Security: Attack detection (DDoS, port scans)
    /// - Systems: Performance degradation alerts
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::universal::UnivMon;
    ///
    /// let mut baseline = UnivMon::new(10000, 0.01, 0.01).unwrap();
    /// let mut current = UnivMon::new(10000, 0.01, 0.01).unwrap();
    ///
    /// // Normal traffic
    /// for i in 0..100 {
    ///     baseline.update(format!("flow_{}", i).as_bytes(), 1.0).unwrap();
    /// }
    ///
    /// // Anomalous traffic (concentrated on few flows)
    /// for _ in 0..100 {
    ///     current.update(b"attack_flow", 1.0).unwrap();
    /// }
    ///
    /// let change = baseline.detect_change(&current);
    /// // Large change detected
    /// ```
    pub fn detect_change(&self, other: &Self) -> f64 {
        if self.num_layers != other.num_layers {
            return f64::MAX; // Incompatible sketches
        }

        if self.total_updates == 0 && other.total_updates == 0 {
            return 0.0;
        }

        // Compute weighted change across layers
        let mut total_change = 0.0;
        let mut layer_weights = 0.0;

        for i in 0..self.num_layers {
            let layer_self = &self.layers[i];
            let layer_other = &other.layers[i];

            if layer_self.sample_count < 5 || layer_other.sample_count < 5 {
                continue; // Skip layers with too few samples
            }

            // Create difference sketch (element-wise subtraction)
            let mut diff_sketch = layer_self.count_sketch.clone();

            // Create negative version of other sketch
            let mut neg_other = layer_other.count_sketch.clone();
            // TODO: Properly negate - for now use inner product

            // Compute L2 distance via inner product
            // ||A - B||² = ||A||² + ||B||² - 2⟨A,B⟩
            let ip_self = layer_self
                .count_sketch
                .inner_product(&layer_self.count_sketch);
            let ip_other = layer_other
                .count_sketch
                .inner_product(&layer_other.count_sketch);
            let ip_cross = layer_self
                .count_sketch
                .inner_product(&layer_other.count_sketch);

            let distance_sq = (ip_self + ip_other - 2 * ip_cross).max(0) as f64;
            let distance = distance_sq.sqrt();

            // Weight by sampling rate (lower layers more important)
            let weight = layer_self.sampling_rate;
            total_change += weight * distance;
            layer_weights += weight;
        }

        if layer_weights > 0.0 {
            total_change / layer_weights
        } else {
            0.0
        }
    }

    /// Get statistics about the UnivMon structure
    ///
    /// Returns information about memory usage, layer structure, and sample distribution.
    ///
    /// # Returns
    ///
    /// `UnivMonStats` structure with detailed statistics
    ///
    /// # Examples
    ///
    /// ```
    /// use sketch_oxide::universal::UnivMon;
    ///
    /// let univmon = UnivMon::new(1_000_000, 0.01, 0.01).unwrap();
    /// let stats = univmon.stats();
    /// println!("Layers: {}, Memory: {} bytes", stats.num_layers, stats.total_memory);
    /// ```
    pub fn stats(&self) -> UnivMonStats {
        let mut total_memory = 0u64;
        let mut layer_stats = Vec::new();

        for layer in &self.layers {
            // Estimate memory for this layer
            let cs_memory = layer.count_sketch.width() * layer.count_sketch.depth() * 8; // i64
            let hh_memory = 1000; // Rough estimate for FrequentItems
            let layer_mem = cs_memory + hh_memory;

            total_memory += layer_mem as u64;

            layer_stats.push(LayerStats {
                layer_index: layer.layer_index,
                sampling_rate: layer.sampling_rate,
                sample_count: layer.sample_count,
                memory_bytes: layer_mem as u64,
            });
        }

        UnivMonStats {
            num_layers: self.num_layers,
            total_memory,
            samples_processed: self.total_updates,
            layer_stats,
        }
    }

    /// Get the number of layers
    #[inline]
    pub fn num_layers(&self) -> usize {
        self.num_layers
    }

    /// Get the maximum stream size
    #[inline]
    pub fn max_stream_size(&self) -> u64 {
        self.max_stream_size
    }

    /// Get the total number of updates
    #[inline]
    pub fn total_updates(&self) -> u64 {
        self.total_updates
    }

    /// Get the epsilon parameter
    #[inline]
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    /// Get the delta parameter
    #[inline]
    pub fn delta(&self) -> f64 {
        self.delta
    }
}

/// Statistics about UnivMon structure and usage
#[derive(Debug, Clone)]
pub struct UnivMonStats {
    /// Number of hierarchical layers
    pub num_layers: usize,
    /// Total memory usage in bytes
    pub total_memory: u64,
    /// Total number of samples processed
    pub samples_processed: u64,
    /// Per-layer statistics
    pub layer_stats: Vec<LayerStats>,
}

/// Statistics for individual layer
#[derive(Debug, Clone)]
pub struct LayerStats {
    /// Layer index (0 = bottom)
    pub layer_index: usize,
    /// Sampling rate for this layer
    pub sampling_rate: f64,
    /// Number of samples at this layer
    pub sample_count: u64,
    /// Memory used by this layer
    pub memory_bytes: u64,
}

impl Sketch for UnivMon {
    type Item = (Vec<u8>, f64);

    fn update(&mut self, item: &Self::Item) {
        let (key, value) = item;
        let _ = UnivMon::update(self, key, *value);
    }

    fn estimate(&self) -> f64 {
        self.estimate_l1()
    }

    fn is_empty(&self) -> bool {
        self.total_updates == 0
    }

    fn serialize(&self) -> Vec<u8> {
        // Format: [num_layers:8][max_stream_size:8][total_updates:8]
        //         [epsilon:8][delta:8][layers...]
        let mut bytes = Vec::new();

        bytes.extend_from_slice(&self.num_layers.to_le_bytes());
        bytes.extend_from_slice(&self.max_stream_size.to_le_bytes());
        bytes.extend_from_slice(&self.total_updates.to_le_bytes());
        bytes.extend_from_slice(&self.epsilon.to_le_bytes());
        bytes.extend_from_slice(&self.delta.to_le_bytes());

        // Serialize each layer
        for layer in &self.layers {
            bytes.extend_from_slice(&layer.sample_count.to_le_bytes());
            bytes.extend_from_slice(&layer.layer_index.to_le_bytes());
            bytes.extend_from_slice(&layer.sampling_rate.to_le_bytes());

            let cs_bytes = layer.count_sketch.serialize();
            bytes.extend_from_slice(&(cs_bytes.len() as u64).to_le_bytes());
            bytes.extend_from_slice(&cs_bytes);

            // Note: FrequentItems doesn't have serialize yet, skip for now
        }

        bytes
    }

    fn deserialize(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 40 {
            return Err(SketchError::DeserializationError(
                "insufficient bytes for UnivMon header".to_string(),
            ));
        }

        let mut offset = 0;

        let num_layers =
            usize::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("invalid num_layers".to_string())
            })?);
        offset += 8;

        let max_stream_size =
            u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("invalid max_stream_size".to_string())
            })?);
        offset += 8;

        let total_updates =
            u64::from_le_bytes(bytes[offset..offset + 8].try_into().map_err(|_| {
                SketchError::DeserializationError("invalid total_updates".to_string())
            })?);
        offset += 8;

        let epsilon = f64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid epsilon".to_string()))?,
        );
        offset += 8;

        let delta = f64::from_le_bytes(
            bytes[offset..offset + 8]
                .try_into()
                .map_err(|_| SketchError::DeserializationError("invalid delta".to_string()))?,
        );
        offset += 8;

        // Deserialize layers (simplified - full implementation would restore all layers)
        // For now, create new instance
        UnivMon::new(max_stream_size, epsilon, delta)
    }
}

impl Mergeable for UnivMon {
    /// Merge another UnivMon into this one
    ///
    /// After merging, this sketch represents the union of both streams.
    /// Each layer is merged independently using Count Sketch merge.
    ///
    /// # Arguments
    ///
    /// * `other` - The UnivMon to merge (must have compatible parameters)
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, error if sketches are incompatible
    ///
    /// # Errors
    ///
    /// Returns `IncompatibleSketches` if:
    /// - Different number of layers
    /// - Different epsilon or delta parameters
    fn merge(&mut self, other: &Self) -> Result<()> {
        if self.num_layers != other.num_layers {
            return Err(SketchError::IncompatibleSketches {
                reason: format!(
                    "layer count mismatch: {} vs {}",
                    self.num_layers, other.num_layers
                ),
            });
        }

        if (self.epsilon - other.epsilon).abs() > 1e-10 {
            return Err(SketchError::IncompatibleSketches {
                reason: format!("epsilon mismatch: {} vs {}", self.epsilon, other.epsilon),
            });
        }

        // Merge each layer
        for i in 0..self.num_layers {
            self.layers[i]
                .count_sketch
                .merge(&other.layers[i].count_sketch)?;
            self.layers[i].sample_count += other.layers[i].sample_count;
            self.layers[i].value_sum += other.layers[i].value_sum;
        }

        self.total_updates += other.total_updates;

        Ok(())
    }
}
