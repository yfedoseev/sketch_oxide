#![allow(non_snake_case)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use sketch_oxide::cardinality::CpcSketch as RustCpcSketch;
use sketch_oxide::cardinality::HyperLogLog as RustHyperLogLog;
use sketch_oxide::cardinality::QSketch as RustQSketch;
use sketch_oxide::cardinality::ThetaSketch as RustThetaSketch;
use sketch_oxide::cardinality::UltraLogLog as RustUltraLogLog;
use sketch_oxide::common::RangeFilter;
use sketch_oxide::frequency::ConservativeCountMin as RustConservativeCountMin;
use sketch_oxide::frequency::CountMinSketch as RustCountMinSketch;
use sketch_oxide::frequency::CountSketch as RustCountSketch;
use sketch_oxide::frequency::HeavyKeeper as RustHeavyKeeper;
use sketch_oxide::frequency::NitroSketch as RustNitroSketch;
use sketch_oxide::frequency::SpaceSaving as RustSpaceSaving;
use sketch_oxide::frequency::{ErrorType as RustErrorType, FrequentItems as RustFrequentItems};
use sketch_oxide::membership::{
    BinaryFuseFilter as RustBinaryFuseFilter, BlockedBloomFilter as RustBlockedBloomFilter,
    BloomFilter as RustBloomFilter, CountingBloomFilter as RustCountingBloomFilter,
    CuckooFilter as RustCuckooFilter, LearnedBloomFilter as RustLearnedBloomFilter,
    RibbonFilter as RustRibbonFilter, StableBloomFilter as RustStableBloomFilter,
    VacuumFilter as RustVacuumFilter,
};
use sketch_oxide::quantiles::{
    DDSketch as RustDDSketch, KllSketch as RustKllSketch, ReqMode, ReqSketch as RustReqSketch,
    SplineSketch as RustSplineSketch, TDigest as RustTDigest,
};
use sketch_oxide::range_filters::{
    Grafite as RustGrafite, MementoFilter as RustMementoFilter, GRF as RustGRF,
};
use sketch_oxide::reconciliation::RatelessIBLT as RustRatelessIBLT;
use sketch_oxide::streaming::SlidingHyperLogLog as RustSlidingHyperLogLog;
use sketch_oxide::universal::UnivMon as RustUnivMon;
use sketch_oxide::{Mergeable, Reconcilable, Sketch};
use std::hash::{Hash, Hasher};
use twox_hash::XxHash64;

/// HyperLogLog cardinality estimator
///
/// Provides ~1.04/sqrt(m) standard error where m = 2^precision
///
/// # Example
/// ```javascript
/// const { HyperLogLog } = require('@sketch-oxide/node');
/// const hll = new HyperLogLog(14);
/// hll.update(Buffer.from('item1'));
/// hll.update(Buffer.from('item2'));
/// console.log(hll.estimate()); // ~2
/// ```
#[napi]
pub struct HyperLogLog {
    inner: RustHyperLogLog,
}

#[napi]
impl HyperLogLog {
    /// Create a new HyperLogLog with given precision
    ///
    /// # Arguments
    /// * `precision` - Number of bits for the hash (4-16 recommended, typical 12-14)
    ///
    /// # Returns
    /// A new HyperLogLog instance
    ///
    /// # Throws
    /// - If precision is out of valid range (4-16)
    ///
    /// # Example
    /// ```javascript
    /// const hll = new HyperLogLog(14);
    /// ```
    #[napi(constructor)]
    pub fn new(precision: u8) -> Result<Self> {
        RustHyperLogLog::new(precision)
            .map(|inner| HyperLogLog { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("HyperLogLog creation failed: {}", e),
                )
            })
    }

    /// Add an item to the sketch
    ///
    /// # Arguments
    /// * `item` - Binary data to add
    ///
    /// # Example
    /// ```javascript
    /// hll.update(Buffer.from('item1'));
    /// hll.update(Buffer.from('hello world'));
    /// ```
    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        // Note: NAPI Buffer needs to be converted to Vec for the Rust trait
        // Future optimization: Add async_batch() method for better throughput
        let data: Vec<u8> = item.to_vec();
        self.inner.update(&data);
        Ok(())
    }

    /// Get current cardinality estimate
    ///
    /// # Returns
    /// Estimated number of unique items
    ///
    /// # Example
    /// ```javascript
    /// const estimate = hll.estimate();
    /// console.log(estimate); // e.g., 1000000
    /// ```
    #[napi]
    pub fn estimate(&self) -> Result<f64> {
        Ok(self.inner.estimate())
    }

    /// Merge another HyperLogLog sketch into this one
    ///
    /// # Arguments
    /// * `other` - Another HyperLogLog with same precision
    ///
    /// # Throws
    /// - If precisions don't match
    ///
    /// # Example
    /// ```javascript
    /// const hll1 = new HyperLogLog(14);
    /// const hll2 = new HyperLogLog(14);
    /// hll1.update(Buffer.from('a'));
    /// hll2.update(Buffer.from('b'));
    /// hll1.merge(hll2);
    /// console.log(hll1.estimate()); // ~2
    /// ```
    #[napi]
    pub fn merge(&mut self, other: &HyperLogLog) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Create a new sketch instance (reset)
    /// Note: In Rust, we create a new instance instead of resetting
    /// Use `new HyperLogLog(precision)` instead
    /// Keeping this method for API compatibility
    #[napi]
    pub fn reset(&mut self) -> Result<()> {
        // Reset by creating new instance - done in JavaScript side
        Ok(())
    }

    /// Get the precision level
    ///
    /// # Example
    /// ```javascript
    /// const hll = new HyperLogLog(14);
    /// console.log(hll.precision()); // 14
    /// ```
    #[napi]
    pub fn precision(&self) -> Result<u8> {
        Ok(self.inner.precision())
    }

    /// Serialize the sketch to binary format
    ///
    /// # Returns
    /// Binary representation suitable for storage/transmission
    ///
    /// # Example
    /// ```javascript
    /// const data = hll.serialize();
    /// fs.writeFileSync('hll.bin', data);
    /// ```
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        let bytes = self.inner.serialize();
        Ok(Buffer::from(bytes))
    }

    /// Deserialize from binary format
    ///
    /// # Arguments
    /// * `data` - Binary data from serialize()
    ///
    /// # Example
    /// ```javascript
    /// const data = fs.readFileSync('hll.bin');
    /// const hll = HyperLogLog.deserialize(data);
    /// ```
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustHyperLogLog::from_bytes(&data)
            .map(|inner| HyperLogLog { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    ///
    /// # Example
    /// ```javascript
    /// console.log(hll.toString());
    /// // "HyperLogLog(precision=14, estimate=1000000)"
    /// ```
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "HyperLogLog(precision={}, estimate={:.0})",
            self.inner.precision(),
            self.inner.estimate()
        ))
    }
}

// =============================================================================
// UltraLogLog - State-of-the-art cardinality estimation (VLDB 2024)
// =============================================================================

/// UltraLogLog cardinality estimator - 28% more space-efficient than HyperLogLog
///
/// UltraLogLog is a state-of-the-art algorithm from VLDB 2024 that provides
/// the same accuracy as HyperLogLog while using 28% less memory.
///
/// # Example
/// ```javascript
/// const { UltraLogLog } = require('@sketch-oxide/node');
/// const ull = new UltraLogLog(12);
/// ull.update(Buffer.from('item1'));
/// ull.update(Buffer.from('item2'));
/// console.log(ull.estimate()); // ~2
/// ```
#[napi]
pub struct UltraLogLog {
    inner: RustUltraLogLog,
}

#[napi]
impl UltraLogLog {
    /// Create a new UltraLogLog with given precision
    ///
    /// # Arguments
    /// * `precision` - Number of bits for the hash (4-18, typical 12-14)
    ///   - precision 4: 16 registers, 16 bytes
    ///   - precision 8: 256 registers, 256 bytes
    ///   - precision 12: 4096 registers, 4 KB (recommended)
    ///   - precision 16: 65536 registers, 64 KB
    ///   - precision 18: 262144 registers, 256 KB
    ///
    /// # Returns
    /// A new UltraLogLog instance
    ///
    /// # Throws
    /// - If precision is out of valid range (4-18)
    ///
    /// # Example
    /// ```javascript
    /// const ull = new UltraLogLog(12);
    /// ```
    #[napi(constructor)]
    pub fn new(precision: u8) -> Result<Self> {
        RustUltraLogLog::new(precision)
            .map(|inner| UltraLogLog { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("UltraLogLog creation failed: {}", e),
                )
            })
    }

    /// Add an item to the sketch
    ///
    /// # Arguments
    /// * `item` - Binary data to add
    ///
    /// # Example
    /// ```javascript
    /// ull.update(Buffer.from('item1'));
    /// ull.update(Buffer.from('hello world'));
    /// ```
    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.add(&data);
        Ok(())
    }

    /// Get current cardinality estimate
    ///
    /// # Returns
    /// Estimated number of unique items
    ///
    /// # Example
    /// ```javascript
    /// const estimate = ull.estimate();
    /// console.log(estimate); // e.g., 1000000
    /// ```
    #[napi]
    pub fn estimate(&self) -> Result<f64> {
        Ok(self.inner.estimate())
    }

    /// Get the cardinality estimate (alias for estimate)
    ///
    /// # Returns
    /// Estimated number of unique items
    ///
    /// # Example
    /// ```javascript
    /// const cardinality = ull.cardinality();
    /// ```
    #[napi]
    pub fn cardinality(&self) -> Result<f64> {
        Ok(self.inner.cardinality())
    }

    /// Merge another UltraLogLog sketch into this one
    ///
    /// # Arguments
    /// * `other` - Another UltraLogLog with same precision
    ///
    /// # Throws
    /// - If precisions don't match
    ///
    /// # Example
    /// ```javascript
    /// const ull1 = new UltraLogLog(12);
    /// const ull2 = new UltraLogLog(12);
    /// ull1.update(Buffer.from('a'));
    /// ull2.update(Buffer.from('b'));
    /// ull1.merge(ull2);
    /// console.log(ull1.estimate()); // ~2
    /// ```
    #[napi]
    pub fn merge(&mut self, other: &UltraLogLog) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Check if the sketch is empty (no items added)
    ///
    /// # Returns
    /// True if no items have been added
    ///
    /// # Example
    /// ```javascript
    /// const ull = new UltraLogLog(12);
    /// console.log(ull.isEmpty()); // true
    /// ull.update(Buffer.from('item'));
    /// console.log(ull.isEmpty()); // false
    /// ```
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    /// Serialize the sketch to binary format
    ///
    /// # Returns
    /// Binary representation suitable for storage/transmission
    ///
    /// # Example
    /// ```javascript
    /// const data = ull.serialize();
    /// fs.writeFileSync('ull.bin', data);
    /// ```
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        let bytes = self.inner.serialize();
        Ok(Buffer::from(bytes))
    }

    /// Deserialize from binary format
    ///
    /// # Arguments
    /// * `data` - Binary data from serialize()
    ///
    /// # Returns
    /// A new UltraLogLog instance
    ///
    /// # Throws
    /// - If data is invalid or corrupted
    ///
    /// # Example
    /// ```javascript
    /// const data = fs.readFileSync('ull.bin');
    /// const ull = UltraLogLog.deserialize(data);
    /// ```
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustUltraLogLog::deserialize(&data)
            .map(|inner| UltraLogLog { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    ///
    /// # Example
    /// ```javascript
    /// console.log(ull.toString());
    /// // "UltraLogLog(estimate=1000000)"
    /// ```
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "UltraLogLog(estimate={:.0})",
            self.inner.estimate()
        ))
    }
}

// =============================================================================
// CpcSketch - Most space-efficient cardinality estimator
// =============================================================================

/// CPC (Compressed Probabilistic Counting) Sketch
///
/// CPC is 30-40% more space-efficient than HyperLogLog for the same accuracy.
/// It achieves this through adaptive compression and multiple operational modes.
///
/// # Example
/// ```javascript
/// const { CpcSketch } = require('@sketch-oxide/node');
/// const cpc = new CpcSketch(11);
/// cpc.update(Buffer.from('item1'));
/// cpc.update(Buffer.from('item2'));
/// console.log(cpc.estimate()); // ~2
/// ```
#[napi]
pub struct CpcSketch {
    inner: RustCpcSketch,
}

#[napi]
impl CpcSketch {
    /// Create a new CPC sketch with given lg_k parameter
    ///
    /// # Arguments
    /// * `lgK` - Log2 of k parameter (4-26), higher = more accurate but more memory
    ///   - lgK 4: k=16, very high error (~20%)
    ///   - lgK 8: k=256, ~6% error
    ///   - lgK 11: k=2048, ~2% error (recommended default)
    ///   - lgK 12: k=4096, ~1.5% error
    ///   - lgK 16: k=65536, ~0.4% error
    ///
    /// # Returns
    /// A new CpcSketch instance
    ///
    /// # Throws
    /// - If lgK is out of valid range (4-26)
    ///
    /// # Example
    /// ```javascript
    /// const cpc = new CpcSketch(11);
    /// ```
    #[napi(constructor)]
    pub fn new(lg_k: u8) -> Result<Self> {
        RustCpcSketch::new(lg_k)
            .map(|inner| CpcSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("CpcSketch creation failed: {}", e),
                )
            })
    }

    /// Add an item to the sketch
    ///
    /// # Arguments
    /// * `item` - Binary data to add
    ///
    /// # Example
    /// ```javascript
    /// cpc.update(Buffer.from('item1'));
    /// cpc.update(Buffer.from('hello world'));
    /// ```
    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        // Hash the data to u64 for the CPC sketch
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        data.hash(&mut hasher);
        let hash = hasher.finish();
        Sketch::update(&mut self.inner, &hash);
        Ok(())
    }

    /// Get current cardinality estimate
    ///
    /// # Returns
    /// Estimated number of unique items
    ///
    /// # Example
    /// ```javascript
    /// const estimate = cpc.estimate();
    /// console.log(estimate); // e.g., 1000000
    /// ```
    #[napi]
    pub fn estimate(&self) -> Result<f64> {
        Ok(Sketch::estimate(&self.inner))
    }

    /// Get the lg_k parameter
    ///
    /// # Returns
    /// The lg_k value used to create this sketch
    ///
    /// # Example
    /// ```javascript
    /// const cpc = new CpcSketch(11);
    /// console.log(cpc.lgK()); // 11
    /// ```
    #[napi]
    pub fn lgK(&self) -> Result<u8> {
        Ok(self.inner.lg_k())
    }

    /// Get the current operational flavor/mode
    ///
    /// CPC uses different representations as cardinality grows:
    /// - Empty: No items observed
    /// - Sparse: Few items, space-efficient
    /// - Hybrid: Transitioning
    /// - Pinned: Dense uncompressed
    /// - Sliding: Dense compressed (maximum efficiency)
    ///
    /// # Returns
    /// String describing the current mode
    ///
    /// # Example
    /// ```javascript
    /// console.log(cpc.flavor()); // "Sparse"
    /// ```
    #[napi]
    pub fn flavor(&self) -> Result<String> {
        Ok(self.inner.flavor().to_string())
    }

    /// Clear the sketch to empty state
    ///
    /// # Example
    /// ```javascript
    /// cpc.clear();
    /// console.log(cpc.isEmpty()); // true
    /// ```
    #[napi]
    pub fn clear(&mut self) -> Result<()> {
        self.inner.clear();
        Ok(())
    }

    /// Check if the sketch is empty
    ///
    /// # Returns
    /// True if no items have been added
    ///
    /// # Example
    /// ```javascript
    /// const cpc = new CpcSketch(11);
    /// console.log(cpc.isEmpty()); // true
    /// ```
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(Sketch::is_empty(&self.inner))
    }

    /// Get the approximate size in bytes
    ///
    /// # Returns
    /// Approximate memory usage in bytes
    ///
    /// # Example
    /// ```javascript
    /// console.log(cpc.sizeBytes()); // e.g., 1024
    /// ```
    #[napi]
    pub fn sizeBytes(&self) -> Result<u32> {
        Ok(self.inner.size_bytes() as u32)
    }

    /// Merge another CpcSketch into this one
    ///
    /// # Arguments
    /// * `other` - Another CpcSketch with same lg_k
    ///
    /// # Throws
    /// - If lg_k values don't match
    ///
    /// # Example
    /// ```javascript
    /// const cpc1 = new CpcSketch(11);
    /// const cpc2 = new CpcSketch(11);
    /// cpc1.update(Buffer.from('a'));
    /// cpc2.update(Buffer.from('b'));
    /// cpc1.merge(cpc2);
    /// console.log(cpc1.estimate()); // ~2
    /// ```
    #[napi]
    pub fn merge(&mut self, other: &CpcSketch) -> Result<()> {
        Mergeable::merge(&mut self.inner, &other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Serialize the sketch to binary format
    ///
    /// # Returns
    /// Binary representation suitable for storage/transmission
    ///
    /// # Example
    /// ```javascript
    /// const data = cpc.serialize();
    /// fs.writeFileSync('cpc.bin', data);
    /// ```
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        let bytes = Sketch::serialize(&self.inner);
        Ok(Buffer::from(bytes))
    }

    /// Deserialize from binary format
    ///
    /// # Arguments
    /// * `data` - Binary data from serialize()
    ///
    /// # Returns
    /// A new CpcSketch instance
    ///
    /// # Throws
    /// - If data is invalid or corrupted
    ///
    /// # Example
    /// ```javascript
    /// const data = fs.readFileSync('cpc.bin');
    /// const cpc = CpcSketch.deserialize(data);
    /// ```
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustCpcSketch::from_bytes(&data)
            .map(|inner| CpcSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    ///
    /// # Example
    /// ```javascript
    /// console.log(cpc.toString());
    /// // "CpcSketch(lgK=11, flavor=Sparse, estimate=1000)"
    /// ```
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "CpcSketch(lgK={}, flavor={}, estimate={:.0})",
            self.inner.lg_k(),
            self.inner.flavor(),
            Sketch::estimate(&self.inner)
        ))
    }
}

// =============================================================================
// QSketch - Weighted cardinality estimation
// =============================================================================

/// QSketch for weighted cardinality estimation
///
/// QSketch maintains a probabilistic sample of weighted elements to estimate
/// the cardinality of weighted sets with bounded error. Unlike standard
/// cardinality sketches that treat all items equally, QSketch accounts for
/// item weights in the cardinality estimate.
///
/// # Example
/// ```javascript
/// const { QSketch } = require('@sketch-oxide/node');
/// const qs = new QSketch(256);
/// qs.update(Buffer.from('user1'), 100.0);
/// qs.update(Buffer.from('user2'), 250.0);
/// const [estimate, error] = qs.estimateWeightedCardinality();
/// console.log(`Weighted cardinality: ${estimate} +/- ${error}`);
/// ```
#[napi]
pub struct QSketch {
    inner: RustQSketch,
}

#[napi]
impl QSketch {
    /// Create a new QSketch with given maximum samples
    ///
    /// # Arguments
    /// * `maxSamples` - Maximum number of samples to maintain (minimum 32)
    ///
    /// # Returns
    /// A new QSketch instance
    ///
    /// # Throws
    /// - If maxSamples is less than 32
    ///
    /// # Example
    /// ```javascript
    /// const qs = new QSketch(256);
    /// ```
    #[napi(constructor)]
    pub fn new(max_samples: u32) -> Result<Self> {
        if max_samples < 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "maxSamples must be at least 32".to_string(),
            ));
        }
        Ok(QSketch {
            inner: RustQSketch::new(max_samples as usize),
        })
    }

    /// Create a new QSketch with a specific seed for reproducibility
    ///
    /// # Arguments
    /// * `maxSamples` - Maximum number of samples to maintain (minimum 32)
    /// * `seed` - Random seed for reproducibility
    ///
    /// # Returns
    /// A new QSketch instance
    ///
    /// # Example
    /// ```javascript
    /// const qs = QSketch.withSeed(256, 42n);
    /// ```
    #[napi(factory)]
    pub fn withSeed(max_samples: u32, seed: BigInt) -> Result<Self> {
        if max_samples < 32 {
            return Err(Error::new(
                Status::InvalidArg,
                "maxSamples must be at least 32".to_string(),
            ));
        }
        let (_, seed_val, _) = seed.get_u64();
        Ok(QSketch {
            inner: RustQSketch::with_seed(max_samples as usize, seed_val),
        })
    }

    /// Add a weighted item to the sketch
    ///
    /// # Arguments
    /// * `item` - Binary data to add
    /// * `weight` - Weight of the item (must be positive)
    ///
    /// # Throws
    /// - If weight is not positive or not finite
    ///
    /// # Example
    /// ```javascript
    /// qs.update(Buffer.from('user_123'), 100.0);
    /// qs.update(Buffer.from('user_456'), 250.0);
    /// ```
    #[napi]
    pub fn update(&mut self, item: Buffer, weight: f64) -> Result<()> {
        if weight <= 0.0 || !weight.is_finite() {
            return Err(Error::new(
                Status::InvalidArg,
                format!("Weight must be positive and finite, got {}", weight),
            ));
        }
        let data: Vec<u8> = item.to_vec();
        self.inner.update(&data, weight);
        Ok(())
    }

    /// Get the maximum number of samples this sketch can maintain
    ///
    /// # Returns
    /// Maximum sample count
    ///
    /// # Example
    /// ```javascript
    /// const qs = new QSketch(256);
    /// console.log(qs.maxSamples()); // 256
    /// ```
    #[napi]
    pub fn maxSamples(&self) -> Result<u32> {
        Ok(self.inner.max_samples() as u32)
    }

    /// Get the current number of samples in the sketch
    ///
    /// # Returns
    /// Current sample count
    ///
    /// # Example
    /// ```javascript
    /// console.log(qs.sampleCount()); // e.g., 150
    /// ```
    #[napi]
    pub fn sampleCount(&self) -> Result<u32> {
        Ok(self.inner.sample_count() as u32)
    }

    /// Get the sum of all weights added to the sketch
    ///
    /// # Returns
    /// Total weight of all items
    ///
    /// # Example
    /// ```javascript
    /// qs.update(Buffer.from('item1'), 10.5);
    /// qs.update(Buffer.from('item2'), 20.5);
    /// console.log(qs.totalWeight()); // 31.0
    /// ```
    #[napi]
    pub fn totalWeight(&self) -> Result<f64> {
        Ok(self.inner.total_weight())
    }

    /// Get the approximate number of distinct elements
    ///
    /// # Returns
    /// Estimated distinct element count
    ///
    /// # Example
    /// ```javascript
    /// const distinct = qs.estimateDistinctElements();
    /// console.log(distinct); // e.g., 1000
    /// ```
    #[napi]
    pub fn estimateDistinctElements(&self) -> Result<i64> {
        Ok(self.inner.estimate_distinct_elements() as i64)
    }

    /// Estimate the weighted cardinality with confidence bounds
    ///
    /// Returns an object with estimate and errorBound, where errorBound
    /// represents the radius of a 95% confidence interval.
    ///
    /// # Returns
    /// Object with { estimate: number, errorBound: number }
    ///
    /// # Example
    /// ```javascript
    /// const result = qs.estimateWeightedCardinality();
    /// console.log(`Estimate: ${result.estimate} +/- ${result.errorBound}`);
    /// ```
    #[napi]
    pub fn estimateWeightedCardinality(&self) -> Result<WeightedCardinalityResult> {
        let (estimate, error_bound) = self.inner.estimate_weighted_cardinality();
        Ok(WeightedCardinalityResult {
            estimate,
            error_bound,
        })
    }

    /// Get the basic cardinality estimate (without error bounds)
    ///
    /// # Returns
    /// Estimated cardinality
    ///
    /// # Example
    /// ```javascript
    /// const estimate = qs.estimate();
    /// ```
    #[napi]
    pub fn estimate(&self) -> Result<f64> {
        Ok(Sketch::estimate(&self.inner))
    }

    /// Check if the sketch is empty
    ///
    /// # Returns
    /// True if no items have been added
    ///
    /// # Example
    /// ```javascript
    /// const qs = new QSketch(256);
    /// console.log(qs.isEmpty()); // true
    /// ```
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(Sketch::is_empty(&self.inner))
    }

    /// Reset the sketch to empty state
    ///
    /// # Example
    /// ```javascript
    /// qs.reset();
    /// console.log(qs.isEmpty()); // true
    /// ```
    #[napi]
    pub fn reset(&mut self) -> Result<()> {
        self.inner.reset();
        Ok(())
    }

    /// Merge another QSketch into this one
    ///
    /// # Arguments
    /// * `other` - Another QSketch with same maxSamples
    ///
    /// # Throws
    /// - If maxSamples values don't match
    ///
    /// # Example
    /// ```javascript
    /// const qs1 = new QSketch(256);
    /// const qs2 = new QSketch(256);
    /// qs1.update(Buffer.from('a'), 100.0);
    /// qs2.update(Buffer.from('b'), 200.0);
    /// qs1.merge(qs2);
    /// ```
    #[napi]
    pub fn merge(&mut self, other: &QSketch) -> Result<()> {
        Mergeable::merge(&mut self.inner, &other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Serialize the sketch to binary format
    ///
    /// # Returns
    /// Binary representation suitable for storage/transmission
    ///
    /// # Example
    /// ```javascript
    /// const data = qs.serialize();
    /// fs.writeFileSync('qs.bin', data);
    /// ```
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        let bytes = Sketch::serialize(&self.inner);
        Ok(Buffer::from(bytes))
    }

    /// Deserialize from binary format
    ///
    /// # Arguments
    /// * `data` - Binary data from serialize()
    ///
    /// # Returns
    /// A new QSketch instance
    ///
    /// # Throws
    /// - If data is invalid or corrupted
    ///
    /// # Example
    /// ```javascript
    /// const data = fs.readFileSync('qs.bin');
    /// const qs = QSketch.deserialize(data);
    /// ```
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustQSketch::from_bytes(&data)
            .map(|inner| QSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    ///
    /// # Example
    /// ```javascript
    /// console.log(qs.toString());
    /// // "QSketch(maxSamples=256, samples=150, totalWeight=1500.0)"
    /// ```
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "QSketch(maxSamples={}, samples={}, totalWeight={:.1})",
            self.inner.max_samples(),
            self.inner.sample_count(),
            self.inner.total_weight()
        ))
    }
}

/// Result of weighted cardinality estimation
#[napi(object)]
pub struct WeightedCardinalityResult {
    /// The estimated weighted cardinality
    pub estimate: f64,
    /// The error bound (95% confidence interval radius)
    pub error_bound: f64,
}

// =============================================================================
// ThetaSketch - Set operations (union, intersection, difference)
// =============================================================================

/// Theta Sketch for cardinality estimation with set operations
///
/// ThetaSketch is the only sketch that supports intersection and difference
/// operations, making it ideal for computing Jaccard similarity, overlap
/// analysis, and set-based queries.
///
/// # Example
/// ```javascript
/// const { ThetaSketch } = require('@sketch-oxide/node');
/// const sketchA = new ThetaSketch(12);
/// const sketchB = new ThetaSketch(12);
///
/// // Add items
/// for (let i = 0; i < 100; i++) sketchA.update(Buffer.from(`item${i}`));
/// for (let i = 50; i < 150; i++) sketchB.update(Buffer.from(`item${i}`));
///
/// // Set operations
/// const union = sketchA.union(sketchB);
/// const intersection = sketchA.intersect(sketchB);
/// const difference = sketchA.difference(sketchB);
///
/// console.log(`Union: ${union.estimate()}`);        // ~150
/// console.log(`Intersection: ${intersection.estimate()}`); // ~50
/// console.log(`Difference: ${difference.estimate()}`);     // ~50
/// ```
#[napi]
pub struct ThetaSketch {
    inner: RustThetaSketch,
}

#[napi]
impl ThetaSketch {
    /// Create a new Theta Sketch with specified lg_k
    ///
    /// # Arguments
    /// * `lgK` - log2(k), determines accuracy and memory (4-26)
    ///   - k = 2^lgK (nominal entries)
    ///   - Memory: ~8k bytes
    ///   - Error: ~1/sqrt(k)
    ///   - lgK=12 (k=4096): ~1.6% error, 32KB (recommended)
    ///   - lgK=14 (k=16384): ~0.8% error, 128KB
    ///   - lgK=16 (k=65536): ~0.4% error, 512KB
    ///
    /// # Returns
    /// A new ThetaSketch instance
    ///
    /// # Throws
    /// - If lgK is out of valid range (4-26)
    ///
    /// # Example
    /// ```javascript
    /// const theta = new ThetaSketch(12);
    /// ```
    #[napi(constructor)]
    pub fn new(lg_k: u8) -> Result<Self> {
        RustThetaSketch::new(lg_k)
            .map(|inner| ThetaSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ThetaSketch creation failed: {}", e),
                )
            })
    }

    /// Create a Theta Sketch with custom seed
    ///
    /// Use the same seed for all sketches that will be combined in set operations.
    ///
    /// # Arguments
    /// * `lgK` - log2(k) parameter (4-26)
    /// * `seed` - Custom seed for hash function
    ///
    /// # Returns
    /// A new ThetaSketch instance
    ///
    /// # Example
    /// ```javascript
    /// const theta = ThetaSketch.withSeed(12, 12345n);
    /// ```
    #[napi(factory)]
    pub fn withSeed(lg_k: u8, seed: BigInt) -> Result<Self> {
        let (_, seed_val, _) = seed.get_u64();
        RustThetaSketch::with_seed(lg_k, seed_val)
            .map(|inner| ThetaSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ThetaSketch creation failed: {}", e),
                )
            })
    }

    /// Add an item to the sketch
    ///
    /// # Arguments
    /// * `item` - Binary data to add
    ///
    /// # Example
    /// ```javascript
    /// theta.update(Buffer.from('item1'));
    /// theta.update(Buffer.from('hello world'));
    /// ```
    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.update(&data);
        Ok(())
    }

    /// Get current cardinality estimate
    ///
    /// # Returns
    /// Estimated number of unique items
    ///
    /// # Example
    /// ```javascript
    /// const estimate = theta.estimate();
    /// console.log(estimate); // e.g., 1000
    /// ```
    #[napi]
    pub fn estimate(&self) -> Result<f64> {
        Ok(self.inner.estimate())
    }

    /// Check if the sketch is empty
    ///
    /// # Returns
    /// True if no items have been added
    ///
    /// # Example
    /// ```javascript
    /// const theta = new ThetaSketch(12);
    /// console.log(theta.isEmpty()); // true
    /// ```
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    /// Get the number of retained hash entries
    ///
    /// # Returns
    /// Number of hash entries currently stored
    ///
    /// # Example
    /// ```javascript
    /// console.log(theta.numRetained()); // e.g., 1000
    /// ```
    #[napi]
    pub fn numRetained(&self) -> Result<u32> {
        Ok(self.inner.num_retained() as u32)
    }

    /// Get the current theta value
    ///
    /// - u64::MAX means no sampling (exact mode)
    /// - Values < u64::MAX indicate sampling is active
    ///
    /// # Returns
    /// Current theta threshold as BigInt
    ///
    /// # Example
    /// ```javascript
    /// const theta = sketch.getTheta();
    /// ```
    #[napi]
    pub fn getTheta(&self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.get_theta()))
    }

    /// Get the nominal capacity (k = 2^lgK)
    ///
    /// # Returns
    /// Nominal capacity
    ///
    /// # Example
    /// ```javascript
    /// const theta = new ThetaSketch(12);
    /// console.log(theta.capacity()); // 4096
    /// ```
    #[napi]
    pub fn capacity(&self) -> Result<u32> {
        Ok(self.inner.capacity() as u32)
    }

    /// Compute union with another sketch: |A ∪ B|
    ///
    /// Returns a new sketch representing items in either A or B (or both).
    /// Both sketches must have the same lgK and seed.
    ///
    /// # Arguments
    /// * `other` - Another ThetaSketch
    ///
    /// # Returns
    /// New ThetaSketch with union result
    ///
    /// # Throws
    /// - If sketches have different lgK or seed
    ///
    /// # Example
    /// ```javascript
    /// const union = sketchA.union(sketchB);
    /// console.log(union.estimate()); // |A ∪ B|
    /// ```
    #[napi]
    pub fn union(&self, other: &ThetaSketch) -> Result<ThetaSketch> {
        self.inner
            .union(&other.inner)
            .map(|inner| ThetaSketch { inner })
            .map_err(|e| Error::new(Status::InvalidArg, format!("Union failed: {}", e)))
    }

    /// Compute intersection with another sketch: |A ∩ B|
    ///
    /// Returns a new sketch representing items in both A and B.
    /// Both sketches must have the same lgK and seed.
    ///
    /// # Arguments
    /// * `other` - Another ThetaSketch
    ///
    /// # Returns
    /// New ThetaSketch with intersection result
    ///
    /// # Throws
    /// - If sketches have different lgK or seed
    ///
    /// # Example
    /// ```javascript
    /// const intersection = sketchA.intersect(sketchB);
    /// console.log(intersection.estimate()); // |A ∩ B|
    /// ```
    #[napi]
    pub fn intersect(&self, other: &ThetaSketch) -> Result<ThetaSketch> {
        self.inner
            .intersect(&other.inner)
            .map(|inner| ThetaSketch { inner })
            .map_err(|e| Error::new(Status::InvalidArg, format!("Intersection failed: {}", e)))
    }

    /// Compute difference: |A - B| (items in A but not in B)
    ///
    /// Returns a new sketch representing items in A that are not in B.
    /// Both sketches must have the same lgK and seed.
    /// Note: A - B != B - A (not commutative)
    ///
    /// # Arguments
    /// * `other` - Another ThetaSketch
    ///
    /// # Returns
    /// New ThetaSketch with difference result
    ///
    /// # Throws
    /// - If sketches have different lgK or seed
    ///
    /// # Example
    /// ```javascript
    /// const difference = sketchA.difference(sketchB);
    /// console.log(difference.estimate()); // |A - B|
    /// ```
    #[napi]
    pub fn difference(&self, other: &ThetaSketch) -> Result<ThetaSketch> {
        self.inner
            .difference(&other.inner)
            .map(|inner| ThetaSketch { inner })
            .map_err(|e| Error::new(Status::InvalidArg, format!("Difference failed: {}", e)))
    }

    /// Compute Jaccard similarity: |A ∩ B| / |A ∪ B|
    ///
    /// Returns the Jaccard similarity coefficient between two sets,
    /// a value between 0 (no overlap) and 1 (identical sets).
    ///
    /// # Arguments
    /// * `other` - Another ThetaSketch
    ///
    /// # Returns
    /// Jaccard similarity coefficient (0.0 to 1.0)
    ///
    /// # Throws
    /// - If sketches have different lgK or seed
    ///
    /// # Example
    /// ```javascript
    /// const similarity = sketchA.jaccardSimilarity(sketchB);
    /// console.log(`Similarity: ${(similarity * 100).toFixed(1)}%`);
    /// ```
    #[napi]
    pub fn jaccardSimilarity(&self, other: &ThetaSketch) -> Result<f64> {
        let union = self
            .inner
            .union(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Union failed: {}", e)))?;
        let intersection = self
            .inner
            .intersect(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Intersection failed: {}", e)))?;

        let union_est = union.estimate();
        if union_est == 0.0 {
            return Ok(0.0);
        }
        Ok(intersection.estimate() / union_est)
    }

    /// Get string representation
    ///
    /// # Example
    /// ```javascript
    /// console.log(theta.toString());
    /// // "ThetaSketch(capacity=4096, retained=1000, estimate=1000)"
    /// ```
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "ThetaSketch(capacity={}, retained={}, estimate={:.0})",
            self.inner.capacity(),
            self.inner.num_retained(),
            self.inner.estimate()
        ))
    }
}

// =============================================================================
// FREQUENCY ESTIMATION SKETCHES
// =============================================================================

// ============================================================================
// COUNT-MIN SKETCH
// ============================================================================

/// Count-Min Sketch for frequency estimation
///
/// A space-efficient probabilistic data structure for estimating item frequencies
/// in a data stream. The sketch guarantees:
/// - Never underestimates (always returns count >= true count)
/// - Error bounded by epsilon*N with probability 1-delta (where N is total stream size)
///
/// # Example
/// ```javascript
/// const { CountMinSketch } = require('@sketch-oxide/node');
/// const cms = new CountMinSketch(0.01, 0.01);
/// cms.update(Buffer.from('apple'));
/// cms.update(Buffer.from('apple'));
/// cms.update(Buffer.from('banana'));
/// console.log(cms.estimate(Buffer.from('apple'))); // ~2
/// ```
#[napi]
pub struct CountMinSketch {
    inner: RustCountMinSketch,
}

#[napi]
impl CountMinSketch {
    /// Create a new Count-Min Sketch with specified error bounds
    ///
    /// # Arguments
    /// * `epsilon` - Error bound (0 < epsilon < 1): estimates are within epsilon*N of true value
    /// * `delta` - Failure probability (0 < delta < 1): guarantee holds with probability 1-delta
    ///
    /// # Returns
    /// A new CountMinSketch instance
    ///
    /// # Throws
    /// - If epsilon is not in (0, 1)
    /// - If delta is not in (0, 1)
    ///
    /// # Example
    /// ```javascript
    /// // 1% error bound, 1% failure probability
    /// const cms = new CountMinSketch(0.01, 0.01);
    /// ```
    #[napi(constructor)]
    pub fn new(epsilon: f64, delta: f64) -> Result<Self> {
        RustCountMinSketch::new(epsilon, delta)
            .map(|inner| CountMinSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("CountMinSketch creation failed: {}", e),
                )
            })
    }

    /// Add an item to the sketch (increment count by 1)
    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.update(&data);
        Ok(())
    }

    /// Estimate the frequency of an item
    #[napi]
    pub fn estimate(&self, item: Buffer) -> Result<i64> {
        let data: Vec<u8> = item.to_vec();
        Ok(self.inner.estimate(&data) as i64)
    }

    /// Merge another Count-Min Sketch into this one
    #[napi]
    pub fn merge(&mut self, other: &CountMinSketch) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Get the width of the sketch
    #[napi]
    pub fn width(&self) -> Result<i64> {
        Ok(self.inner.width() as i64)
    }

    /// Get the depth of the sketch
    #[napi]
    pub fn depth(&self) -> Result<i64> {
        Ok(self.inner.depth() as i64)
    }

    /// Get the epsilon parameter
    #[napi]
    pub fn epsilon(&self) -> Result<f64> {
        Ok(self.inner.epsilon())
    }

    /// Get the delta parameter
    #[napi]
    pub fn delta(&self) -> Result<f64> {
        Ok(self.inner.delta())
    }

    /// Serialize the sketch to binary format
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        let bytes = self.inner.serialize();
        Ok(Buffer::from(bytes))
    }

    /// Deserialize from binary format
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustCountMinSketch::deserialize(&data)
            .map(|inner| CountMinSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "CountMinSketch(width={}, depth={}, epsilon={}, delta={})",
            self.inner.width(),
            self.inner.depth(),
            self.inner.epsilon(),
            self.inner.delta()
        ))
    }
}

// ============================================================================
// COUNT SKETCH
// ============================================================================

/// Count Sketch for unbiased frequency estimation
///
/// Unlike Count-Min Sketch, Count Sketch:
/// - Can estimate both positive and negative frequencies (supports deletions)
/// - Has E[estimate] = true_count (unbiased)
/// - Provides L2 error guarantees
///
/// # Example
/// ```javascript
/// const { CountSketch } = require('@sketch-oxide/node');
/// const cs = new CountSketch(0.1, 0.01);
/// cs.update(Buffer.from('apple'), 5);
/// cs.update(Buffer.from('apple'), -2);  // Decrement by 2
/// console.log(cs.estimate(Buffer.from('apple'))); // ~3
/// ```
#[napi]
pub struct CountSketch {
    inner: RustCountSketch,
}

#[napi]
impl CountSketch {
    /// Create a new Count Sketch with specified error bounds
    #[napi(constructor)]
    pub fn new(epsilon: f64, delta: f64) -> Result<Self> {
        RustCountSketch::new(epsilon, delta)
            .map(|inner| CountSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("CountSketch creation failed: {}", e),
                )
            })
    }

    /// Update the sketch with an item and delta
    #[napi]
    pub fn update(&mut self, item: Buffer, delta: i64) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.update(&data, delta);
        Ok(())
    }

    /// Estimate the frequency of an item
    #[napi]
    pub fn estimate(&self, item: Buffer) -> Result<i64> {
        let data: Vec<u8> = item.to_vec();
        Ok(self.inner.estimate(&data))
    }

    /// Estimate inner product of two frequency vectors
    #[napi]
    pub fn innerProduct(&self, other: &CountSketch) -> Result<i64> {
        Ok(self.inner.inner_product(&other.inner))
    }

    /// Merge another Count Sketch into this one
    #[napi]
    pub fn merge(&mut self, other: &CountSketch) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    #[napi]
    pub fn width(&self) -> Result<i64> {
        Ok(self.inner.width() as i64)
    }

    #[napi]
    pub fn depth(&self) -> Result<i64> {
        Ok(self.inner.depth() as i64)
    }

    #[napi]
    pub fn epsilon(&self) -> Result<f64> {
        Ok(self.inner.epsilon())
    }

    #[napi]
    pub fn delta(&self) -> Result<f64> {
        Ok(self.inner.delta())
    }

    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        let bytes = self.inner.serialize();
        Ok(Buffer::from(bytes))
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustCountSketch::deserialize(&data)
            .map(|inner| CountSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "CountSketch(width={}, depth={}, epsilon={}, delta={})",
            self.inner.width(),
            self.inner.depth(),
            self.inner.epsilon(),
            self.inner.delta()
        ))
    }
}

// ============================================================================
// CONSERVATIVE COUNT-MIN SKETCH
// ============================================================================

/// Conservative Update Count-Min Sketch for improved frequency estimation
///
/// More accurate than standard Count-Min (up to 10x less overestimation)
/// but does NOT support deletions.
#[napi]
pub struct ConservativeCountMin {
    inner: RustConservativeCountMin,
}

#[napi]
impl ConservativeCountMin {
    #[napi(constructor)]
    pub fn new(epsilon: f64, delta: f64) -> Result<Self> {
        RustConservativeCountMin::new(epsilon, delta)
            .map(|inner| ConservativeCountMin { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ConservativeCountMin creation failed: {}", e),
                )
            })
    }

    #[napi(factory)]
    pub fn withDimensions(width: i64, depth: i64) -> Result<Self> {
        RustConservativeCountMin::with_dimensions(width as usize, depth as usize)
            .map(|inner| ConservativeCountMin { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ConservativeCountMin creation failed: {}", e),
                )
            })
    }

    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.update(&data);
        Ok(())
    }

    #[napi]
    pub fn updateCount(&mut self, item: Buffer, count: i64) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.update_count(&data, count as u64);
        Ok(())
    }

    #[napi]
    pub fn estimate(&self, item: Buffer) -> Result<i64> {
        let data: Vec<u8> = item.to_vec();
        Ok(self.inner.estimate(&data) as i64)
    }

    #[napi]
    pub fn merge(&mut self, other: &ConservativeCountMin) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    #[napi]
    pub fn width(&self) -> Result<i64> {
        Ok(self.inner.width() as i64)
    }

    #[napi]
    pub fn depth(&self) -> Result<i64> {
        Ok(self.inner.depth() as i64)
    }

    #[napi]
    pub fn epsilon(&self) -> Result<f64> {
        Ok(self.inner.epsilon())
    }

    #[napi]
    pub fn delta(&self) -> Result<f64> {
        Ok(self.inner.delta())
    }

    #[napi]
    pub fn totalCount(&self) -> Result<i64> {
        Ok(self.inner.total_count() as i64)
    }

    #[napi]
    pub fn memoryUsage(&self) -> Result<i64> {
        Ok(self.inner.memory_usage() as i64)
    }

    #[napi]
    pub fn clear(&mut self) -> Result<()> {
        self.inner.clear();
        Ok(())
    }

    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        let bytes = self.inner.to_bytes();
        Ok(Buffer::from(bytes))
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustConservativeCountMin::from_bytes(&data)
            .map(|inner| ConservativeCountMin { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "ConservativeCountMin(width={}, depth={}, epsilon={:.4}, delta={:.4})",
            self.inner.width(),
            self.inner.depth(),
            self.inner.epsilon(),
            self.inner.delta()
        ))
    }
}

// ============================================================================
// SPACE-SAVING SKETCH
// ============================================================================

/// Result object for Space-Saving heavy hitter queries
#[napi(object)]
pub struct HeavyHitterResult {
    pub key: String,
    pub lower_bound: i64,
    pub upper_bound: i64,
}

/// Space-Saving Sketch for Heavy Hitters Detection
///
/// Finds the most frequent items with guaranteed no false negatives.
#[napi]
pub struct SpaceSaving {
    inner: RustSpaceSaving<Vec<u8>>,
}

#[napi]
impl SpaceSaving {
    #[napi(constructor)]
    pub fn new(epsilon: f64) -> Result<Self> {
        RustSpaceSaving::new(epsilon)
            .map(|inner| SpaceSaving { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("SpaceSaving creation failed: {}", e),
                )
            })
    }

    #[napi(factory)]
    pub fn withCapacity(capacity: i64) -> Result<Self> {
        RustSpaceSaving::with_capacity(capacity as usize)
            .map(|inner| SpaceSaving { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("SpaceSaving creation failed: {}", e),
                )
            })
    }

    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.update(data);
        Ok(())
    }

    #[napi]
    pub fn estimate(&self, item: Buffer) -> Result<Option<HeavyHitterResult>> {
        let data: Vec<u8> = item.to_vec();
        match self.inner.estimate(&data) {
            Some((lower, upper)) => Ok(Some(HeavyHitterResult {
                key: hex::encode(&data),
                lower_bound: lower as i64,
                upper_bound: upper as i64,
            })),
            None => Ok(None),
        }
    }

    #[napi]
    pub fn heavyHitters(&self, threshold: f64) -> Result<Vec<HeavyHitterResult>> {
        let results = self.inner.heavy_hitters(threshold);
        Ok(results
            .into_iter()
            .map(|(key, lower, upper)| HeavyHitterResult {
                key: hex::encode(&key),
                lower_bound: lower as i64,
                upper_bound: upper as i64,
            })
            .collect())
    }

    #[napi]
    pub fn topK(&self, k: i64) -> Result<Vec<HeavyHitterResult>> {
        let results = self.inner.top_k(k as usize);
        Ok(results
            .into_iter()
            .map(|(key, lower, upper)| HeavyHitterResult {
                key: hex::encode(&key),
                lower_bound: lower as i64,
                upper_bound: upper as i64,
            })
            .collect())
    }

    #[napi]
    pub fn merge(&mut self, other: &SpaceSaving) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    #[napi]
    pub fn capacity(&self) -> Result<i64> {
        Ok(self.inner.capacity() as i64)
    }

    #[napi]
    pub fn streamLength(&self) -> Result<i64> {
        Ok(self.inner.stream_length() as i64)
    }

    #[napi]
    pub fn epsilon(&self) -> Result<f64> {
        Ok(self.inner.epsilon())
    }

    #[napi]
    pub fn numItems(&self) -> Result<i64> {
        Ok(self.inner.num_items() as i64)
    }

    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    #[napi]
    pub fn maxError(&self) -> Result<i64> {
        Ok(self.inner.max_error() as i64)
    }

    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "SpaceSaving(capacity={}, numItems={}, streamLength={}, epsilon={:.4})",
            self.inner.capacity(),
            self.inner.num_items(),
            self.inner.stream_length(),
            self.inner.epsilon()
        ))
    }
}

// ============================================================================
// FREQUENT ITEMS SKETCH
// ============================================================================

/// Error type for frequent items queries
#[napi]
pub enum FrequentItemsErrorType {
    NoFalsePositives,
    NoFalseNegatives,
}

/// Result object for FrequentItems queries
#[napi(object)]
pub struct FrequentItemResult {
    pub key: String,
    pub lower_bound: i64,
    pub upper_bound: i64,
}

/// Frequent Items sketch based on Misra-Gries algorithm
#[napi]
pub struct FrequentItems {
    inner: RustFrequentItems<Vec<u8>>,
}

#[napi]
impl FrequentItems {
    #[napi(constructor)]
    pub fn new(max_size: i64) -> Result<Self> {
        RustFrequentItems::new(max_size as usize)
            .map(|inner| FrequentItems { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("FrequentItems creation failed: {}", e),
                )
            })
    }

    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.update(data);
        Ok(())
    }

    #[napi]
    pub fn updateBy(&mut self, item: Buffer, count: i64) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner.update_by(data, count as u64);
        Ok(())
    }

    #[napi]
    pub fn getEstimate(&self, item: Buffer) -> Result<Option<FrequentItemResult>> {
        let data: Vec<u8> = item.to_vec();
        match self.inner.get_estimate(&data) {
            Some((lower, upper)) => Ok(Some(FrequentItemResult {
                key: hex::encode(&data),
                lower_bound: lower as i64,
                upper_bound: upper as i64,
            })),
            None => Ok(None),
        }
    }

    #[napi]
    pub fn frequentItems(
        &self,
        error_type: FrequentItemsErrorType,
    ) -> Result<Vec<FrequentItemResult>> {
        let rust_error_type = match error_type {
            FrequentItemsErrorType::NoFalsePositives => RustErrorType::NoFalsePositives,
            FrequentItemsErrorType::NoFalseNegatives => RustErrorType::NoFalseNegatives,
        };
        let results = self.inner.frequent_items(rust_error_type);
        Ok(results
            .into_iter()
            .map(|(key, lower, upper)| FrequentItemResult {
                key: hex::encode(&key),
                lower_bound: lower as i64,
                upper_bound: upper as i64,
            })
            .collect())
    }

    #[napi]
    pub fn merge(&mut self, other: &FrequentItems) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    #[napi]
    pub fn numItems(&self) -> Result<i64> {
        Ok(self.inner.num_items() as i64)
    }

    #[napi]
    pub fn maxSize(&self) -> Result<i64> {
        Ok(self.inner.max_size() as i64)
    }

    #[napi]
    pub fn offset(&self) -> Result<i64> {
        Ok(self.inner.offset() as i64)
    }

    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "FrequentItems(maxSize={}, numItems={}, offset={})",
            self.inner.max_size(),
            self.inner.num_items(),
            self.inner.offset()
        ))
    }
}

// =============================================================================
// MEMBERSHIP TESTING
// =============================================================================

/// Binary Fuse Filter for probabilistic membership testing
///
/// State-of-the-art filter with ~75% better space efficiency than Bloom filters.
/// This is an immutable data structure - build from a complete set of items.
#[napi]
pub struct BinaryFuseFilter {
    inner: RustBinaryFuseFilter,
}

#[napi]
impl BinaryFuseFilter {
    /// Create a Binary Fuse Filter from an array of BigInt items
    #[napi(factory)]
    pub fn fromItems(items: Vec<BigInt>, bits_per_entry: u8) -> Result<Self> {
        let u64_items: Vec<u64> = items
            .into_iter()
            .map(|bi| {
                let (_, value, _) = bi.get_u64();
                value
            })
            .collect();

        RustBinaryFuseFilter::from_items(u64_items.into_iter(), bits_per_entry)
            .map(|inner| BinaryFuseFilter { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("BinaryFuseFilter creation failed: {}", e),
                )
            })
    }

    /// Check if an item might be in the set
    #[napi]
    pub fn contains(&self, item: BigInt) -> bool {
        let (_, value, _) = item.get_u64();
        self.inner.contains(&value)
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn bitsPerEntry(&self) -> f64 {
        self.inner.bits_per_entry()
    }

    #[napi]
    pub fn estimatedFpr(&self) -> f64 {
        self.inner.estimated_fpr()
    }

    #[napi]
    pub fn serialize(&self) -> Buffer {
        Buffer::from(self.inner.serialize())
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustBinaryFuseFilter::deserialize(&data)
            .map(|inner| BinaryFuseFilter { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "BinaryFuseFilter(size={}, bits_per_entry={:.1}, fpr={:.4}%)",
            self.inner.len(),
            self.inner.bits_per_entry(),
            self.inner.estimated_fpr() * 100.0
        )
    }
}

/// Bloom Filter for probabilistic membership testing
///
/// Classic probabilistic data structure supporting dynamic insertions.
/// Zero false negatives guaranteed.
#[napi]
pub struct BloomFilter {
    inner: RustBloomFilter,
}

#[napi]
impl BloomFilter {
    #[napi(constructor)]
    pub fn new(n: u32, fpr: Option<f64>) -> Result<Self> {
        let fpr = fpr.unwrap_or(0.01);
        if n == 0 {
            return Err(Error::new(Status::InvalidArg, "n must be > 0"));
        }
        if fpr <= 0.0 || fpr >= 1.0 {
            return Err(Error::new(Status::InvalidArg, "fpr must be in (0, 1)"));
        }
        Ok(Self {
            inner: RustBloomFilter::new(n as usize, fpr),
        })
    }

    #[napi]
    pub fn insert(&mut self, key: Buffer) {
        self.inner.insert(&key);
    }

    #[napi]
    pub fn contains(&self, key: Buffer) -> bool {
        self.inner.contains(&key)
    }

    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[napi]
    pub fn mergeWith(&mut self, other: &BloomFilter) -> Result<()> {
        if self.inner.params() != other.inner.params() {
            return Err(Error::new(Status::InvalidArg, "Parameters must match"));
        }
        self.inner.merge(&other.inner);
        Ok(())
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn falsePositiveRate(&self) -> f64 {
        self.inner.false_positive_rate()
    }

    #[napi]
    pub fn memoryUsage(&self) -> u32 {
        self.inner.memory_usage() as u32
    }

    #[napi]
    pub fn serialize(&self) -> Buffer {
        Buffer::from(self.inner.to_bytes())
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustBloomFilter::from_bytes(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn toString(&self) -> String {
        let (n, m, k) = self.inner.params();
        format!(
            "BloomFilter(n={}, bits={}, hashes={}, fpr={:.4}%)",
            n,
            m,
            k,
            self.inner.false_positive_rate() * 100.0
        )
    }
}

/// Blocked Bloom Filter for cache-efficient membership testing
#[napi]
pub struct BlockedBloomFilter {
    inner: RustBlockedBloomFilter,
}

#[napi]
impl BlockedBloomFilter {
    #[napi(constructor)]
    pub fn new(n: u32, fpr: Option<f64>) -> Result<Self> {
        let fpr = fpr.unwrap_or(0.01);
        if n == 0 {
            return Err(Error::new(Status::InvalidArg, "n must be > 0"));
        }
        if fpr <= 0.0 || fpr >= 1.0 {
            return Err(Error::new(Status::InvalidArg, "fpr must be in (0, 1)"));
        }
        Ok(Self {
            inner: RustBlockedBloomFilter::new(n as usize, fpr),
        })
    }

    #[napi]
    pub fn insert(&mut self, key: Buffer) {
        self.inner.insert(&key);
    }

    #[napi]
    pub fn contains(&self, key: Buffer) -> bool {
        self.inner.contains(&key)
    }

    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[napi]
    pub fn mergeWith(&mut self, other: &BlockedBloomFilter) -> Result<()> {
        if self.inner.params() != other.inner.params() {
            return Err(Error::new(Status::InvalidArg, "Parameters must match"));
        }
        self.inner.merge(&other.inner);
        Ok(())
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn falsePositiveRate(&self) -> f64 {
        self.inner.false_positive_rate()
    }

    #[napi]
    pub fn memoryUsage(&self) -> u32 {
        self.inner.memory_usage() as u32
    }

    #[napi]
    pub fn serialize(&self) -> Buffer {
        Buffer::from(self.inner.to_bytes())
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustBlockedBloomFilter::from_bytes(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn toString(&self) -> String {
        let (n, num_blocks, k) = self.inner.params();
        format!(
            "BlockedBloomFilter(n={}, blocks={}, hashes={}, fpr={:.4}%)",
            n,
            num_blocks,
            k,
            self.inner.false_positive_rate() * 100.0
        )
    }
}

/// Counting Bloom Filter with deletion support
#[napi]
pub struct CountingBloomFilter {
    inner: RustCountingBloomFilter,
}

#[napi]
impl CountingBloomFilter {
    #[napi(constructor)]
    pub fn new(n: u32, fpr: Option<f64>) -> Result<Self> {
        let fpr = fpr.unwrap_or(0.01);
        if n == 0 {
            return Err(Error::new(Status::InvalidArg, "n must be > 0"));
        }
        if fpr <= 0.0 || fpr >= 1.0 {
            return Err(Error::new(Status::InvalidArg, "fpr must be in (0, 1)"));
        }
        Ok(Self {
            inner: RustCountingBloomFilter::new(n as usize, fpr),
        })
    }

    #[napi]
    pub fn insert(&mut self, key: Buffer) {
        self.inner.insert(&key);
    }

    #[napi]
    pub fn remove(&mut self, key: Buffer) -> bool {
        self.inner.remove(&key)
    }

    #[napi]
    pub fn contains(&self, key: Buffer) -> bool {
        self.inner.contains(&key)
    }

    #[napi]
    pub fn countEstimate(&self, key: Buffer) -> u8 {
        self.inner.count_estimate(&key)
    }

    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[napi]
    pub fn mergeWith(&mut self, other: &CountingBloomFilter) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn hasOverflow(&self) -> bool {
        self.inner.has_overflow()
    }

    #[napi]
    pub fn falsePositiveRate(&self) -> f64 {
        self.inner.false_positive_rate()
    }

    #[napi]
    pub fn memoryUsage(&self) -> u32 {
        self.inner.memory_usage() as u32
    }

    #[napi]
    pub fn serialize(&self) -> Buffer {
        Buffer::from(self.inner.to_bytes())
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustCountingBloomFilter::from_bytes(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "CountingBloomFilter(count={}, fpr={:.4}%)",
            self.inner.len(),
            self.inner.false_positive_rate() * 100.0
        )
    }
}

/// Cuckoo Filter for membership testing with deletions
#[napi]
pub struct CuckooFilter {
    inner: RustCuckooFilter,
}

#[napi]
impl CuckooFilter {
    #[napi(constructor)]
    pub fn new(capacity: u32) -> Result<Self> {
        RustCuckooFilter::new(capacity as usize)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
    }

    #[napi]
    pub fn insert(&mut self, key: Buffer) -> Result<()> {
        self.inner
            .insert(&key)
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn remove(&mut self, key: Buffer) -> bool {
        self.inner.remove(&key)
    }

    #[napi]
    pub fn contains(&self, key: Buffer) -> bool {
        self.inner.contains(&key)
    }

    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn capacity(&self) -> u32 {
        self.inner.capacity() as u32
    }

    #[napi]
    pub fn loadFactor(&self) -> f64 {
        self.inner.load_factor()
    }

    #[napi]
    pub fn memoryUsage(&self) -> u32 {
        self.inner.memory_usage() as u32
    }

    #[napi]
    pub fn serialize(&self) -> Buffer {
        Buffer::from(self.inner.to_bytes())
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustCuckooFilter::from_bytes(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "CuckooFilter(count={}, capacity={}, load={:.1}%)",
            self.inner.len(),
            self.inner.capacity(),
            self.inner.load_factor() * 100.0
        )
    }
}

/// Ribbon Filter for space-efficient membership testing
#[napi]
pub struct RibbonFilter {
    inner: RustRibbonFilter,
}

#[napi]
impl RibbonFilter {
    #[napi(constructor)]
    pub fn new(n: u32, fpr: Option<f64>) -> Result<Self> {
        let fpr = fpr.unwrap_or(0.01);
        if n == 0 {
            return Err(Error::new(Status::InvalidArg, "n must be > 0"));
        }
        if fpr <= 0.0 || fpr >= 1.0 {
            return Err(Error::new(Status::InvalidArg, "fpr must be in (0, 1)"));
        }
        Ok(Self {
            inner: RustRibbonFilter::new(n as usize, fpr),
        })
    }

    #[napi]
    pub fn insert(&mut self, key: Buffer) -> Result<()> {
        if self.inner.is_finalized() {
            return Err(Error::new(
                Status::InvalidArg,
                "Cannot insert after finalization",
            ));
        }
        self.inner.insert(&key);
        Ok(())
    }

    #[napi]
    pub fn build(&mut self) {
        self.inner.finalize();
    }

    #[napi]
    pub fn contains(&self, key: Buffer) -> Result<bool> {
        if !self.inner.is_finalized() {
            return Err(Error::new(
                Status::InvalidArg,
                "Must call build() before querying",
            ));
        }
        Ok(self.inner.contains(&key))
    }

    #[napi]
    pub fn isFinalized(&self) -> bool {
        self.inner.is_finalized()
    }

    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn falsePositiveRate(&self) -> f64 {
        self.inner.false_positive_rate()
    }

    #[napi]
    pub fn memoryUsage(&self) -> u32 {
        self.inner.memory_usage() as u32
    }

    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        if !self.inner.is_finalized() {
            return Err(Error::new(
                Status::InvalidArg,
                "Must call build() before serialization",
            ));
        }
        Ok(Buffer::from(self.inner.to_bytes()))
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustRibbonFilter::from_bytes(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "RibbonFilter(count={}, finalized={}, fpr={:.4}%)",
            self.inner.len(),
            self.inner.is_finalized(),
            self.inner.false_positive_rate() * 100.0
        )
    }
}

/// StableBloomFilter for membership testing with decay mechanism
///
/// A Bloom filter variant that decays counts over time, useful for tracking
/// recently-seen items while removing old ones without explicit deletion.
///
/// # Example
/// ```javascript
/// const { StableBloomFilter } = require('@sketch-oxide/node');
/// const sbf = new StableBloomFilter(1000, 0.01);
/// sbf.insert(Buffer.from('item1'));
/// console.log(sbf.contains(Buffer.from('item1'))); // true
/// ```
#[napi]
pub struct StableBloomFilter {
    inner: RustStableBloomFilter,
}

#[napi]
impl StableBloomFilter {
    /// Create a new StableBloomFilter
    ///
    /// # Arguments
    /// * `expected_items` - Expected number of items to store
    /// * `fpr` - Target false positive rate (0 < fpr < 1)
    #[napi(constructor)]
    pub fn new(expected_items: u32, fpr: Option<f64>) -> Result<Self> {
        let fpr = fpr.unwrap_or(0.01);
        if expected_items == 0 {
            return Err(Error::new(Status::InvalidArg, "expected_items must be > 0"));
        }
        if fpr <= 0.0 || fpr >= 1.0 {
            return Err(Error::new(Status::InvalidArg, "fpr must be in (0, 1)"));
        }
        RustStableBloomFilter::new(expected_items as usize, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
    }

    /// Insert a key into the filter
    ///
    /// # Arguments
    /// * `key` - Binary data to insert
    #[napi]
    pub fn insert(&mut self, key: Buffer) -> Result<()> {
        self.inner.insert(&key);
        Ok(())
    }

    /// Check if a key might be in the set
    ///
    /// # Returns
    /// True if key might be present, False if definitely not present
    #[napi]
    pub fn contains(&self, key: Buffer) -> bool {
        self.inner.contains(&key)
    }

    /// Get the count estimate for a key
    ///
    /// # Arguments
    /// * `key` - Binary data to check
    ///
    /// # Returns
    /// Counter value (0-255 depending on counter_bits)
    #[napi]
    pub fn getCount(&self, key: Buffer) -> u32 {
        self.inner.get_count(&key) as u32
    }

    /// Get the number of counters in this filter
    #[napi]
    pub fn numCounters(&self) -> u32 {
        self.inner.num_counters() as u32
    }

    /// Get the number of hash functions used
    #[napi]
    pub fn numHashes(&self) -> u32 {
        self.inner.num_hashes() as u32
    }

    /// Get the number of decrements performed
    #[napi]
    pub fn decrementCount(&self) -> u32 {
        self.inner.decrement_count() as u32
    }

    /// Get the fill ratio of the filter (0.0 to 1.0)
    #[napi]
    pub fn fillRatio(&self) -> f64 {
        self.inner.fill_ratio()
    }

    /// Clear all data from the filter
    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get memory usage in bytes
    #[napi]
    pub fn memoryUsage(&self) -> u32 {
        self.inner.memory_usage() as u32
    }

    /// Serialize the filter to binary format
    #[napi]
    pub fn serialize(&self) -> Buffer {
        Buffer::from(self.inner.to_bytes())
    }

    /// Deserialize a filter from binary format
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustStableBloomFilter::from_bytes(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    /// Get a string representation
    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "StableBloomFilter(counters={}, hashes={}, fill={:.2}%)",
            self.inner.num_counters(),
            self.inner.num_hashes(),
            self.inner.fill_ratio() * 100.0
        )
    }
}

// =============================================================================
// QUANTILE ESTIMATION
// =============================================================================

/// DDSketch for quantile estimation with relative error guarantees
#[napi]
pub struct DDSketch {
    inner: RustDDSketch,
}

#[napi]
impl DDSketch {
    #[napi(constructor)]
    pub fn new(relative_accuracy: f64) -> Result<Self> {
        RustDDSketch::new(relative_accuracy)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
    }

    #[napi]
    pub fn update(&mut self, value: f64) {
        Sketch::update(&mut self.inner, &value);
    }

    #[napi]
    pub fn updateBatch(&mut self, values: Vec<f64>) {
        for val in values {
            Sketch::update(&mut self.inner, &val);
        }
    }

    #[napi]
    pub fn quantile(&self, q: f64) -> Option<f64> {
        self.inner.quantile(q)
    }

    #[napi]
    pub fn quantiles(&self, quantiles: Vec<f64>) -> Vec<f64> {
        quantiles
            .iter()
            .filter_map(|&q| self.inner.quantile(q))
            .collect()
    }

    #[napi]
    pub fn mergeWith(&mut self, other: &DDSketch) -> Result<()> {
        Mergeable::merge(&mut self.inner, &other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
    }

    #[napi]
    pub fn count(&self) -> i64 {
        self.inner.count() as i64
    }

    #[napi]
    pub fn min(&self) -> Option<f64> {
        self.inner.min()
    }

    #[napi]
    pub fn max(&self) -> Option<f64> {
        self.inner.max()
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        Sketch::is_empty(&self.inner)
    }

    #[napi]
    pub fn serialize(&self) -> Buffer {
        Buffer::from(Sketch::serialize(&self.inner))
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustDDSketch::deserialize(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "DDSketch(count={}, min={:.2}, max={:.2})",
            self.inner.count(),
            self.inner.min().unwrap_or(0.0),
            self.inner.max().unwrap_or(0.0)
        )
    }
}

/// REQ Sketch mode
#[napi]
pub enum ReqSketchMode {
    HighRankAccuracy,
    LowRankAccuracy,
}

/// REQ Sketch for streaming quantile estimation (PODS 2021)
#[napi]
pub struct ReqSketch {
    inner: RustReqSketch,
}

#[napi]
impl ReqSketch {
    #[napi(constructor)]
    pub fn new(k: u32, mode: ReqSketchMode) -> Result<Self> {
        let rust_mode = match mode {
            ReqSketchMode::HighRankAccuracy => ReqMode::HighRankAccuracy,
            ReqSketchMode::LowRankAccuracy => ReqMode::LowRankAccuracy,
        };
        RustReqSketch::new(k as usize, rust_mode)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::InvalidArg, e))
    }

    #[napi]
    pub fn update(&mut self, value: f64) {
        self.inner.update(value);
    }

    #[napi]
    pub fn updateBatch(&mut self, values: Vec<f64>) {
        for val in values {
            self.inner.update(val);
        }
    }

    #[napi]
    pub fn quantile(&self, q: f64) -> Option<f64> {
        self.inner.quantile(q)
    }

    #[napi]
    pub fn quantiles(&self, quantiles: Vec<f64>) -> Vec<f64> {
        quantiles
            .iter()
            .filter_map(|&q| self.inner.quantile(q))
            .collect()
    }

    #[napi]
    pub fn mergeWith(&self, other: &ReqSketch) -> Result<ReqSketch> {
        self.inner
            .merge(&other.inner)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::InvalidArg, e))
    }

    #[napi]
    pub fn count(&self) -> i64 {
        self.inner.n() as i64
    }

    #[napi]
    pub fn min(&self) -> Option<f64> {
        self.inner.min()
    }

    #[napi]
    pub fn max(&self) -> Option<f64> {
        self.inner.max()
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        self.inner.is_empty()
    }

    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "ReqSketch(count={}, min={:.2}, max={:.2})",
            self.inner.n(),
            self.inner.min().unwrap_or(0.0),
            self.inner.max().unwrap_or(0.0)
        )
    }
}

/// T-Digest for quantile estimation with tail accuracy
#[napi]
pub struct TDigest {
    inner: RustTDigest,
}

#[napi]
impl TDigest {
    #[napi(constructor)]
    pub fn new(compression: Option<f64>) -> Self {
        Self {
            inner: RustTDigest::new(compression.unwrap_or(100.0)),
        }
    }

    #[napi]
    pub fn update(&mut self, value: f64) {
        self.inner.update(value);
    }

    #[napi]
    pub fn updateBatch(&mut self, values: Vec<f64>) {
        self.inner.update_batch(&values);
    }

    #[napi]
    pub fn quantile(&mut self, q: f64) -> f64 {
        self.inner.quantile(q)
    }

    #[napi]
    pub fn quantiles(&mut self, quantiles: Vec<f64>) -> Vec<f64> {
        quantiles.iter().map(|&q| self.inner.quantile(q)).collect()
    }

    #[napi]
    pub fn cdf(&mut self, value: f64) -> f64 {
        self.inner.cdf(value)
    }

    #[napi]
    pub fn trimmedMean(&mut self, low: f64, high: f64) -> f64 {
        self.inner.trimmed_mean(low, high)
    }

    #[napi]
    pub fn mergeWith(&mut self, other: &TDigest) -> Result<()> {
        Mergeable::merge(&mut self.inner, &other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
    }

    #[napi]
    pub fn count(&self) -> f64 {
        self.inner.count()
    }

    #[napi]
    pub fn compression(&self) -> f64 {
        self.inner.compression()
    }

    #[napi]
    pub fn centroidCount(&self) -> u32 {
        self.inner.centroid_count() as u32
    }

    #[napi]
    pub fn min(&self) -> f64 {
        self.inner.min()
    }

    #[napi]
    pub fn max(&self) -> f64 {
        self.inner.max()
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        Sketch::is_empty(&self.inner)
    }

    #[napi]
    pub fn serialize(&mut self) -> Buffer {
        Buffer::from(self.inner.to_bytes())
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustTDigest::from_bytes(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "TDigest(count={:.0}, centroids={}, compression={:.0})",
            self.inner.count(),
            self.inner.centroid_count(),
            self.inner.compression()
        )
    }
}

/// KLL Sketch for quantile estimation (Karnin 2016)
#[napi]
pub struct KllSketch {
    inner: RustKllSketch,
}

#[napi]
impl KllSketch {
    #[napi(constructor)]
    pub fn new(k: Option<u32>) -> Result<Self> {
        RustKllSketch::new(k.unwrap_or(200) as u16)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
    }

    #[napi]
    pub fn update(&mut self, value: f64) {
        self.inner.update(value);
    }

    #[napi]
    pub fn updateBatch(&mut self, values: Vec<f64>) {
        for val in values {
            self.inner.update(val);
        }
    }

    #[napi]
    pub fn quantile(&mut self, rank: f64) -> Option<f64> {
        self.inner.quantile(rank)
    }

    #[napi]
    pub fn quantiles(&mut self, ranks: Vec<f64>) -> Vec<f64> {
        ranks
            .iter()
            .filter_map(|&r| self.inner.quantile(r))
            .collect()
    }

    #[napi]
    pub fn rank(&mut self, value: f64) -> f64 {
        self.inner.rank(value)
    }

    #[napi]
    pub fn mergeWith(&mut self, other: &KllSketch) -> Result<()> {
        Mergeable::merge(&mut self.inner, &other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
    }

    #[napi]
    pub fn k(&self) -> u32 {
        self.inner.k() as u32
    }

    #[napi]
    pub fn count(&self) -> i64 {
        self.inner.count() as i64
    }

    #[napi]
    pub fn min(&self) -> f64 {
        self.inner.min()
    }

    #[napi]
    pub fn max(&self) -> f64 {
        self.inner.max()
    }

    #[napi]
    pub fn normalizedRankError(&self) -> f64 {
        self.inner.normalized_rank_error()
    }

    #[napi]
    pub fn numRetained(&self) -> u32 {
        self.inner.num_retained() as u32
    }

    #[napi]
    pub fn isEmpty(&self) -> bool {
        Sketch::is_empty(&self.inner)
    }

    #[napi]
    pub fn serialize(&mut self) -> Buffer {
        Buffer::from(self.inner.to_bytes())
    }

    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustKllSketch::from_bytes(&data)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::GenericFailure, e.to_string()))
    }

    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "KllSketch(k={}, count={}, min={:.2}, max={:.2})",
            self.inner.k(),
            self.inner.count(),
            self.inner.min(),
            self.inner.max()
        )
    }
}

/// SplineSketch for quantile estimation
///
/// A space-efficient sketch for estimating quantiles of a data stream.
/// Uses spline-based interpolation for smooth quantile estimates.
///
/// # Example
/// ```javascript
/// const { SplineSketch } = require('@sketch-oxide/node');
/// const sketch = new SplineSketch(256);
/// for (let i = 1; i <= 1000; i++) {
///   sketch.insert(i, 1.0);
/// }
/// console.log(sketch.query(0.5)); // ~500
/// ```
#[napi]
pub struct SplineSketch {
    inner: RustSplineSketch,
}

#[napi]
impl SplineSketch {
    /// Create a new SplineSketch
    ///
    /// # Arguments
    /// * `max_samples` - Maximum number of samples to retain
    #[napi(constructor)]
    pub fn new(max_samples: Option<u32>) -> Self {
        let max_samples = max_samples.unwrap_or(256) as usize;
        Self {
            inner: RustSplineSketch::new(max_samples),
        }
    }

    /// Add a value to the sketch
    ///
    /// # Arguments
    /// * `value` - The value to add
    /// * `weight` - The weight of the value (default 1.0)
    #[napi]
    pub fn insert(&mut self, value: i64, weight: Option<f64>) -> Result<()> {
        let weight = weight.unwrap_or(1.0);
        self.inner.update(value as u64, weight);
        Ok(())
    }

    /// Query a quantile value
    ///
    /// # Arguments
    /// * `quantile` - The quantile to query (0.0 to 1.0)
    ///
    /// # Returns
    /// The estimated value at that quantile
    #[napi]
    pub fn query(&self, quantile: f64) -> i64 {
        self.inner.query(quantile) as i64
    }

    /// Get the minimum value seen
    #[napi]
    pub fn min(&self) -> Option<i64> {
        self.inner.min().map(|v| v as i64)
    }

    /// Get the maximum value seen
    #[napi]
    pub fn max(&self) -> Option<i64> {
        self.inner.max().map(|v| v as i64)
    }

    /// Get the number of samples retained
    #[napi]
    pub fn sampleCount(&self) -> u32 {
        self.inner.sample_count() as u32
    }

    /// Get the maximum number of samples
    #[napi]
    pub fn maxSamples(&self) -> u32 {
        self.inner.max_samples() as u32
    }

    /// Get the total weight of all values
    #[napi]
    pub fn totalWeight(&self) -> f64 {
        self.inner.total_weight()
    }

    /// Merge another SplineSketch into this one
    ///
    /// # Arguments
    /// * `other` - Another SplineSketch to merge
    #[napi]
    pub fn mergeWith(&mut self, other: &SplineSketch) -> Result<()> {
        self.inner.merge_into(&other.inner);
        Ok(())
    }

    /// Reset the sketch to empty state
    #[napi]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// Get a string representation
    #[napi]
    pub fn toString(&self) -> String {
        let min_str = self
            .inner
            .min()
            .map_or("N/A".to_string(), |v| format!("{}", v));
        let max_str = self
            .inner
            .max()
            .map_or("N/A".to_string(), |v| format!("{}", v));
        format!(
            "SplineSketch(samples={}/{}, min={}, max={})",
            self.inner.sample_count(),
            self.inner.max_samples(),
            min_str,
            max_str
        )
    }
}

// New bindings for similarity, sampling, streaming, and additional frequency algorithms
// To be merged into lib.rs

// =============================================================================
// SIMILARITY ALGORITHMS
// =============================================================================

use sketch_oxide::similarity::MinHash as RustMinHash;
use sketch_oxide::similarity::SimHash as RustSimHash;

/// MinHash sketch for Jaccard similarity estimation
///
/// Approximates the Jaccard similarity |A ∩ B| / |A ∪ B| using k hash functions.
/// Standard error is approximately 1/sqrt(k).
#[napi]
pub struct MinHash {
    inner: RustMinHash,
}

#[napi]
impl MinHash {
    /// Create a new MinHash sketch with specified number of permutations
    #[napi(constructor)]
    pub fn new(num_perm: u32) -> Result<Self> {
        RustMinHash::new(num_perm as usize)
            .map(|inner| MinHash { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("MinHash creation failed: {}", e),
                )
            })
    }

    /// Add an item to the set
    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        self.inner.update(&item.to_vec());
        Ok(())
    }

    /// Estimate Jaccard similarity with another MinHash
    #[napi]
    pub fn jaccardSimilarity(&self, other: &MinHash) -> Result<f64> {
        self.inner
            .jaccard_similarity(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Similarity failed: {}", e)))
    }

    /// Merge another MinHash (union operation)
    #[napi]
    pub fn merge(&mut self, other: &MinHash) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Get the number of permutations
    #[napi]
    pub fn numPerm(&self) -> Result<u32> {
        Ok(self.inner.num_perm() as u32)
    }

    /// Check if empty
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    /// Serialize to binary format
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.inner.serialize()))
    }

    /// Deserialize from binary format
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustMinHash::deserialize(&data)
            .map(|inner| MinHash { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "MinHash(numPerm={}, isEmpty={})",
            self.inner.num_perm(),
            self.inner.is_empty()
        ))
    }
}

/// SimHash sketch for near-duplicate detection
#[napi]
pub struct SimHash {
    inner: RustSimHash,
}

#[napi]
impl SimHash {
    /// Create a new SimHash sketch
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        Ok(SimHash {
            inner: RustSimHash::new(),
        })
    }

    /// Add a feature to the sketch
    #[napi]
    pub fn update(&mut self, feature: Buffer) -> Result<()> {
        self.inner.update(&feature.to_vec());
        Ok(())
    }

    /// Add a weighted feature
    #[napi]
    pub fn updateWeighted(&mut self, feature: Buffer, weight: i64) -> Result<()> {
        self.inner.update_weighted(&feature.to_vec(), weight);
        Ok(())
    }

    /// Get the 64-bit fingerprint
    #[napi]
    pub fn fingerprint(&mut self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.fingerprint()))
    }

    /// Compute Hamming distance to another SimHash (0-64)
    #[napi]
    pub fn hammingDistance(&mut self, other: &mut SimHash) -> Result<u32> {
        Ok(self.inner.hamming_distance(&mut other.inner))
    }

    /// Compute similarity as (64 - hammingDistance) / 64
    #[napi]
    pub fn similarity(&mut self, other: &mut SimHash) -> Result<f64> {
        Ok(self.inner.similarity(&mut other.inner))
    }

    /// Get number of features added
    #[napi]
    pub fn len(&self) -> Result<u32> {
        Ok(self.inner.len() as u32)
    }

    /// Check if empty
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    /// Merge another SimHash
    #[napi]
    pub fn merge(&mut self, other: &SimHash) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Serialize to binary format
    #[napi]
    pub fn serialize(&mut self) -> Result<Buffer> {
        Ok(Buffer::from(self.inner.to_bytes()))
    }

    /// Deserialize from binary format
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustSimHash::from_bytes(&data)
            .map(|inner| SimHash { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "SimHash(features={}, isEmpty={})",
            self.inner.len(),
            self.inner.is_empty()
        ))
    }
}

// =============================================================================
// SAMPLING ALGORITHMS
// =============================================================================

use sketch_oxide::sampling::ReservoirSampling as RustReservoirSampling;
use sketch_oxide::sampling::VarOptSampling as RustVarOptSampling;

/// Reservoir Sampling for uniform random samples from streams
#[napi]
pub struct ReservoirSampling {
    inner: RustReservoirSampling<Vec<u8>>,
}

#[napi]
impl ReservoirSampling {
    /// Create a new Reservoir Sampling instance
    #[napi(constructor)]
    pub fn new(k: u32) -> Result<Self> {
        RustReservoirSampling::new(k as usize)
            .map(|inner| ReservoirSampling { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ReservoirSampling creation failed: {}", e),
                )
            })
    }

    /// Create with seed for reproducibility
    #[napi(factory)]
    pub fn withSeed(k: u32, seed: BigInt) -> Result<Self> {
        let (_, seed_val, _) = seed.get_u64();
        RustReservoirSampling::with_seed(k as usize, seed_val)
            .map(|inner| ReservoirSampling { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ReservoirSampling creation failed: {}", e),
                )
            })
    }

    /// Add an item to the stream
    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        self.inner.update(item.to_vec());
        Ok(())
    }

    /// Get the current sample
    #[napi]
    pub fn sample(&self) -> Result<Vec<Buffer>> {
        Ok(self
            .inner
            .sample()
            .iter()
            .map(|v| Buffer::from(v.clone()))
            .collect())
    }

    /// Check if empty
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    /// Get current sample size
    #[napi]
    pub fn len(&self) -> Result<u32> {
        Ok(self.inner.len() as u32)
    }

    /// Get maximum capacity
    #[napi]
    pub fn capacity(&self) -> Result<u32> {
        Ok(self.inner.capacity() as u32)
    }

    /// Get total items seen
    #[napi]
    pub fn count(&self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.count()))
    }

    /// Get inclusion probability
    #[napi]
    pub fn inclusionProbability(&self) -> Result<f64> {
        Ok(self.inner.inclusion_probability())
    }

    /// Clear the reservoir
    #[napi]
    pub fn clear(&mut self) -> Result<()> {
        self.inner.clear();
        Ok(())
    }

    /// Merge another reservoir sample
    #[napi]
    pub fn merge(&mut self, other: &ReservoirSampling) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "ReservoirSampling(capacity={}, len={}, count={})",
            self.inner.capacity(),
            self.inner.len(),
            self.inner.count()
        ))
    }
}

/// Weighted item in a VarOpt sample
#[napi(object)]
pub struct WeightedSampleItem {
    pub item: Buffer,
    pub weight: f64,
    pub adjusted_weight: f64,
}

/// VarOpt Sampling for variance-optimal weighted samples
#[napi]
pub struct VarOptSampling {
    inner: RustVarOptSampling<Vec<u8>>,
}

#[napi]
impl VarOptSampling {
    /// Create a new VarOpt Sampling instance
    #[napi(constructor)]
    pub fn new(k: u32) -> Result<Self> {
        RustVarOptSampling::new(k as usize)
            .map(|inner| VarOptSampling { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("VarOptSampling creation failed: {}", e),
                )
            })
    }

    /// Create with seed for reproducibility
    #[napi(factory)]
    pub fn withSeed(k: u32, seed: BigInt) -> Result<Self> {
        let (_, seed_val, _) = seed.get_u64();
        RustVarOptSampling::with_seed(k as usize, seed_val)
            .map(|inner| VarOptSampling { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("VarOptSampling creation failed: {}", e),
                )
            })
    }

    /// Add a weighted item
    #[napi]
    pub fn update(&mut self, item: Buffer, weight: f64) -> Result<()> {
        if weight <= 0.0 || !weight.is_finite() {
            return Err(Error::new(
                Status::InvalidArg,
                "Weight must be positive and finite",
            ));
        }
        self.inner.update(item.to_vec(), weight);
        Ok(())
    }

    /// Get the current sample
    #[napi]
    pub fn sample(&self) -> Result<Vec<WeightedSampleItem>> {
        Ok(self
            .inner
            .sample()
            .iter()
            .map(|wi| WeightedSampleItem {
                item: Buffer::from(wi.item.clone()),
                weight: wi.weight,
                adjusted_weight: wi.adjusted_weight,
            })
            .collect())
    }

    /// Check if empty
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    /// Get current sample size
    #[napi]
    pub fn len(&self) -> Result<u32> {
        Ok(self.inner.len() as u32)
    }

    /// Get maximum capacity
    #[napi]
    pub fn capacity(&self) -> Result<u32> {
        Ok(self.inner.capacity() as u32)
    }

    /// Get total items seen
    #[napi]
    pub fn count(&self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.count()))
    }

    /// Get current threshold
    #[napi]
    pub fn threshold(&self) -> Result<f64> {
        Ok(self.inner.threshold())
    }

    /// Get total weight in sample
    #[napi]
    pub fn totalWeight(&self) -> Result<f64> {
        Ok(self.inner.total_weight())
    }

    /// Estimate total weight of stream
    #[napi]
    pub fn estimateTotalWeight(&self) -> Result<f64> {
        Ok(self.inner.estimate_total_weight())
    }

    /// Clear the sampler
    #[napi]
    pub fn clear(&mut self) -> Result<()> {
        self.inner.clear();
        Ok(())
    }

    /// Merge another VarOpt sample
    #[napi]
    pub fn merge(&mut self, other: &VarOptSampling) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "VarOptSampling(capacity={}, len={}, count={}, threshold={:.2})",
            self.inner.capacity(),
            self.inner.len(),
            self.inner.count(),
            self.inner.threshold()
        ))
    }
}

// =============================================================================
// STREAMING ALGORITHMS
// =============================================================================

use sketch_oxide::streaming::ExponentialHistogram as RustExponentialHistogram;
use sketch_oxide::streaming::SlidingWindowCounter as RustSlidingWindowCounter;

/// Sliding Window Counter using Exponential Histogram
#[napi]
pub struct SlidingWindowCounter {
    inner: RustSlidingWindowCounter,
}

#[napi]
impl SlidingWindowCounter {
    /// Create a new Sliding Window Counter
    #[napi(constructor)]
    pub fn new(window_size: BigInt, epsilon: f64) -> Result<Self> {
        let (_, window_val, _) = window_size.get_u64();
        RustSlidingWindowCounter::new(window_val, epsilon)
            .map(|inner| SlidingWindowCounter { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("SlidingWindowCounter creation failed: {}", e),
                )
            })
    }

    /// Increment at given timestamp
    #[napi]
    pub fn increment(&mut self, timestamp: BigInt) -> Result<()> {
        let (_, ts, _) = timestamp.get_u64();
        self.inner.increment(ts);
        Ok(())
    }

    /// Increment by amount at given timestamp
    #[napi]
    pub fn incrementBy(&mut self, timestamp: BigInt, count: BigInt) -> Result<()> {
        let (_, ts, _) = timestamp.get_u64();
        let (_, cnt, _) = count.get_u64();
        self.inner.increment_by(ts, cnt);
        Ok(())
    }

    /// Get count within window ending at given time
    #[napi]
    pub fn count(&self, current_time: BigInt) -> Result<BigInt> {
        let (_, ts, _) = current_time.get_u64();
        Ok(BigInt::from(self.inner.count(ts)))
    }

    /// Get count for specific range
    #[napi]
    pub fn countRange(&self, start: BigInt, end: BigInt) -> Result<BigInt> {
        let (_, s, _) = start.get_u64();
        let (_, e, _) = end.get_u64();
        Ok(BigInt::from(self.inner.count_range(s, e)))
    }

    /// Expire old buckets
    #[napi]
    pub fn expire(&mut self, current_time: BigInt) -> Result<()> {
        let (_, ts, _) = current_time.get_u64();
        self.inner.expire(ts);
        Ok(())
    }

    /// Clear all buckets
    #[napi]
    pub fn clear(&mut self) -> Result<()> {
        self.inner.clear();
        Ok(())
    }

    /// Get window size
    #[napi]
    pub fn windowSize(&self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.window_size()))
    }

    /// Get error bound
    #[napi]
    pub fn epsilon(&self) -> Result<f64> {
        Ok(self.inner.epsilon())
    }

    /// Get number of buckets
    #[napi]
    pub fn numBuckets(&self) -> Result<u32> {
        Ok(self.inner.num_buckets() as u32)
    }

    /// Get theoretical error bound
    #[napi]
    pub fn errorBound(&self) -> Result<f64> {
        Ok(self.inner.error_bound())
    }

    /// Get memory usage in bytes
    #[napi]
    pub fn memoryUsage(&self) -> Result<u32> {
        Ok(self.inner.memory_usage() as u32)
    }

    /// Serialize to binary format
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.inner.to_bytes()))
    }

    /// Deserialize from binary format
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustSlidingWindowCounter::from_bytes(&data)
            .map(|inner| SlidingWindowCounter { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "SlidingWindowCounter(windowSize={}, epsilon={:.4}, buckets={})",
            self.inner.window_size(),
            self.inner.epsilon(),
            self.inner.num_buckets()
        ))
    }
}

/// Count estimate with bounds
#[napi(object)]
pub struct CountWithBounds {
    pub estimate: BigInt,
    pub lower: BigInt,
    pub upper: BigInt,
}

/// Exponential Histogram with formal error bounds
#[napi]
pub struct ExponentialHistogram {
    inner: RustExponentialHistogram,
}

#[napi]
impl ExponentialHistogram {
    /// Create a new Exponential Histogram
    #[napi(constructor)]
    pub fn new(window_size: BigInt, epsilon: f64) -> Result<Self> {
        let (_, window_val, _) = window_size.get_u64();
        RustExponentialHistogram::new(window_val, epsilon)
            .map(|inner| ExponentialHistogram { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ExponentialHistogram creation failed: {}", e),
                )
            })
    }

    /// Insert an event at timestamp with count
    #[napi]
    pub fn insert(&mut self, timestamp: BigInt, count: BigInt) -> Result<()> {
        let (_, ts, _) = timestamp.get_u64();
        let (_, cnt, _) = count.get_u64();
        self.inner.insert(ts, cnt);
        Ok(())
    }

    /// Get count estimate with bounds
    #[napi]
    pub fn count(&self, current_time: BigInt) -> Result<CountWithBounds> {
        let (_, ts, _) = current_time.get_u64();
        let (estimate, lower, upper) = self.inner.count(ts);
        Ok(CountWithBounds {
            estimate: BigInt::from(estimate),
            lower: BigInt::from(lower),
            upper: BigInt::from(upper),
        })
    }

    /// Expire old buckets
    #[napi]
    pub fn expire(&mut self, current_time: BigInt) -> Result<()> {
        let (_, ts, _) = current_time.get_u64();
        self.inner.expire(ts);
        Ok(())
    }

    /// Clear all buckets
    #[napi]
    pub fn clear(&mut self) -> Result<()> {
        self.inner.clear();
        Ok(())
    }

    /// Get window size
    #[napi]
    pub fn windowSize(&self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.window_size()))
    }

    /// Get error bound
    #[napi]
    pub fn epsilon(&self) -> Result<f64> {
        Ok(self.inner.epsilon())
    }

    /// Get k value
    #[napi]
    pub fn k(&self) -> Result<u32> {
        Ok(self.inner.k() as u32)
    }

    /// Get number of buckets
    #[napi]
    pub fn numBuckets(&self) -> Result<u32> {
        Ok(self.inner.num_buckets() as u32)
    }

    /// Get theoretical error bound
    #[napi]
    pub fn errorBound(&self) -> Result<f64> {
        Ok(self.inner.error_bound())
    }

    /// Get memory usage in bytes
    #[napi]
    pub fn memoryUsage(&self) -> Result<u32> {
        Ok(self.inner.memory_usage() as u32)
    }

    /// Check if empty
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    /// Merge another ExponentialHistogram
    #[napi]
    pub fn merge(&mut self, other: &ExponentialHistogram) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Serialize to binary format
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.inner.serialize()))
    }

    /// Deserialize from binary format
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustExponentialHistogram::deserialize(&data)
            .map(|inner| ExponentialHistogram { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "ExponentialHistogram(windowSize={}, epsilon={:.4}, k={}, buckets={})",
            self.inner.window_size(),
            self.inner.epsilon(),
            self.inner.k(),
            self.inner.num_buckets()
        ))
    }
}

// =============================================================================
// ADDITIONAL FREQUENCY ALGORITHMS
// =============================================================================

use sketch_oxide::frequency::ElasticSketch as RustElasticSketch;
use sketch_oxide::frequency::RemovableUniversalSketch as RustRemovableUniversalSketch;
use sketch_oxide::frequency::SALSA as RustSALSA;

/// Heavy hitter result
#[napi(object)]
pub struct HeavyHitter {
    pub item_hash: BigInt,
    pub frequency: BigInt,
}

/// Elastic Sketch for frequency estimation
#[napi]
pub struct ElasticSketch {
    inner: RustElasticSketch,
}

#[napi]
impl ElasticSketch {
    /// Create a new Elastic Sketch with default elastic ratio (0.2)
    #[napi(constructor)]
    pub fn new(bucket_count: u32, depth: u32) -> Result<Self> {
        RustElasticSketch::new(bucket_count as usize, depth as usize)
            .map(|inner| ElasticSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ElasticSketch creation failed: {}", e),
                )
            })
    }

    /// Create with custom elastic ratio
    #[napi(factory)]
    pub fn withElasticRatio(bucket_count: u32, depth: u32, elastic_ratio: f64) -> Result<Self> {
        RustElasticSketch::with_elastic_ratio(bucket_count as usize, depth as usize, elastic_ratio)
            .map(|inner| ElasticSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("ElasticSketch creation failed: {}", e),
                )
            })
    }

    /// Update with item and count
    #[napi]
    pub fn update(&mut self, item: Buffer, count: BigInt) -> Result<()> {
        let (_, cnt, _) = count.get_u64();
        self.inner.update(&item.to_vec(), cnt);
        Ok(())
    }

    /// Estimate frequency of an item
    #[napi]
    pub fn estimate(&self, item: Buffer) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.estimate(&item.to_vec())))
    }

    /// Find heavy hitters with frequency >= threshold
    #[napi]
    pub fn heavyHitters(&self, threshold: BigInt) -> Result<Vec<HeavyHitter>> {
        let (_, thresh, _) = threshold.get_u64();
        Ok(self
            .inner
            .heavy_hitters(thresh)
            .into_iter()
            .map(|(hash, freq)| HeavyHitter {
                item_hash: BigInt::from(hash),
                frequency: BigInt::from(freq),
            })
            .collect())
    }

    /// Reset the sketch
    #[napi]
    pub fn reset(&mut self) -> Result<()> {
        self.inner.reset();
        Ok(())
    }

    /// Get bucket count
    #[napi]
    pub fn bucketCount(&self) -> Result<u32> {
        Ok(self.inner.bucket_count() as u32)
    }

    /// Get depth
    #[napi]
    pub fn depth(&self) -> Result<u32> {
        Ok(self.inner.depth() as u32)
    }

    /// Get elastic ratio
    #[napi]
    pub fn elasticRatio(&self) -> Result<f64> {
        Ok(self.inner.elastic_ratio())
    }

    /// Get total count
    #[napi]
    pub fn totalCount(&self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.total_count()))
    }

    /// Check if empty
    #[napi]
    pub fn isEmpty(&self) -> Result<bool> {
        Ok(self.inner.is_empty())
    }

    /// Get memory usage
    #[napi]
    pub fn memoryUsage(&self) -> Result<u32> {
        Ok(self.inner.memory_usage() as u32)
    }

    /// Merge another ElasticSketch
    #[napi]
    pub fn merge(&mut self, other: &ElasticSketch) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Serialize to binary format
    #[napi]
    pub fn serialize(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.inner.serialize()))
    }

    /// Deserialize from binary format
    #[napi(factory)]
    pub fn deserialize(data: Buffer) -> Result<Self> {
        RustElasticSketch::deserialize(&data)
            .map(|inner| ElasticSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::GenericFailure,
                    format!("Deserialization failed: {}", e),
                )
            })
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "ElasticSketch(buckets={}, depth={}, elasticRatio={:.2}, totalCount={})",
            self.inner.bucket_count(),
            self.inner.depth(),
            self.inner.elastic_ratio(),
            self.inner.total_count()
        ))
    }
}

/// Estimate result with confidence
#[napi(object)]
pub struct EstimateWithConfidence {
    pub estimate: BigInt,
    pub confidence: BigInt,
}

/// SALSA: Self-Adjusting Counter Sizing for frequency estimation
#[napi]
pub struct SALSA {
    inner: RustSALSA,
}

#[napi]
impl SALSA {
    /// Create a new SALSA sketch
    #[napi(constructor)]
    pub fn new(epsilon: f64, delta: f64) -> Result<Self> {
        RustSALSA::new(epsilon, delta)
            .map(|inner| SALSA { inner })
            .map_err(|e| Error::new(Status::InvalidArg, format!("SALSA creation failed: {}", e)))
    }

    /// Update with item and frequency
    #[napi]
    pub fn update(&mut self, item: Buffer, count: BigInt) -> Result<()> {
        let (_, cnt, _) = count.get_u64();
        self.inner.update(&item.to_vec(), cnt);
        Ok(())
    }

    /// Estimate frequency with confidence
    #[napi]
    pub fn estimate(&self, item: Buffer) -> Result<EstimateWithConfidence> {
        let (estimate, confidence) = self.inner.estimate(&item.to_vec());
        Ok(EstimateWithConfidence {
            estimate: BigInt::from(estimate),
            confidence: BigInt::from(confidence),
        })
    }

    /// Get epsilon parameter
    #[napi]
    pub fn epsilon(&self) -> Result<f64> {
        Ok(self.inner.epsilon())
    }

    /// Get delta parameter
    #[napi]
    pub fn delta(&self) -> Result<f64> {
        Ok(self.inner.delta())
    }

    /// Get maximum frequency observed
    #[napi]
    pub fn maxObserved(&self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.max_observed()))
    }

    /// Get total updates
    #[napi]
    pub fn totalUpdates(&self) -> Result<BigInt> {
        Ok(BigInt::from(self.inner.total_updates()))
    }

    /// Get adaptation level
    #[napi]
    pub fn adaptationLevel(&self) -> Result<u32> {
        Ok(self.inner.adaptation_level())
    }

    /// Get width
    #[napi]
    pub fn width(&self) -> Result<u32> {
        Ok(self.inner.width() as u32)
    }

    /// Get depth
    #[napi]
    pub fn depth(&self) -> Result<u32> {
        Ok(self.inner.depth() as u32)
    }

    /// Merge another SALSA sketch
    #[napi]
    pub fn merge(&mut self, other: &SALSA) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "SALSA(epsilon={:.4}, delta={:.4}, width={}, depth={}, adaptationLevel={})",
            self.inner.epsilon(),
            self.inner.delta(),
            self.inner.width(),
            self.inner.depth(),
            self.inner.adaptation_level()
        ))
    }
}

/// Removable Universal Sketch for frequency estimation with deletions
#[napi]
pub struct RemovableUniversalSketch {
    inner: RustRemovableUniversalSketch,
}

#[napi]
impl RemovableUniversalSketch {
    /// Create a new Removable Universal Sketch
    #[napi(constructor)]
    pub fn new(epsilon: f64, delta: f64) -> Result<Self> {
        RustRemovableUniversalSketch::new(epsilon, delta)
            .map(|inner| RemovableUniversalSketch { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("RemovableUniversalSketch creation failed: {}", e),
                )
            })
    }

    /// Update with signed frequency (positive for insert, negative for delete)
    #[napi]
    pub fn update(&mut self, item: Buffer, delta: i32) -> Result<()> {
        self.inner.update(&item.to_vec(), delta);
        Ok(())
    }

    /// Estimate frequency
    #[napi]
    pub fn estimate(&self, item: Buffer) -> Result<i64> {
        Ok(self.inner.estimate(&item.to_vec()))
    }

    /// Compute L2 norm of frequency vector
    #[napi]
    pub fn l2Norm(&self) -> Result<f64> {
        Ok(self.inner.l2_norm())
    }

    /// Get epsilon parameter
    #[napi]
    pub fn epsilon(&self) -> Result<f64> {
        Ok(self.inner.epsilon())
    }

    /// Get delta parameter
    #[napi]
    pub fn delta(&self) -> Result<f64> {
        Ok(self.inner.delta())
    }

    /// Get width
    #[napi]
    pub fn width(&self) -> Result<u32> {
        Ok(self.inner.width() as u32)
    }

    /// Get depth
    #[napi]
    pub fn depth(&self) -> Result<u32> {
        Ok(self.inner.depth() as u32)
    }

    /// Merge another RemovableUniversalSketch
    #[napi]
    pub fn merge(&mut self, other: &RemovableUniversalSketch) -> Result<()> {
        self.inner
            .merge(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Merge failed: {}", e)))
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> Result<String> {
        Ok(format!(
            "RemovableUniversalSketch(epsilon={:.4}, delta={:.4}, width={}, depth={})",
            self.inner.epsilon(),
            self.inner.delta(),
            self.inner.width(),
            self.inner.depth()
        ))
    }
}

// ============================================================================
// TIER 1 NEW SKETCHES (2025)
// ============================================================================

/// HeavyKeeper - Top-k heavy hitter detection with exponential decay
///
/// Identifies the most frequent items in a data stream with high precision.
/// Uses exponential decay to actively remove small flows while protecting heavy hitters.
///
/// # Example
/// ```javascript
/// const { HeavyKeeper } = require('@sketch-oxide/node');
/// const hk = new HeavyKeeper(100, 0.001, 0.01);
///
/// // Update with items
/// for (let i = 0; i < 1000; i++) {
///     hk.update(Buffer.from('frequent_item'));
/// }
/// for (let i = 0; i < 10; i++) {
///     hk.update(Buffer.from('rare_item'));
/// }
///
/// // Get top-k heavy hitters
/// const topK = hk.topK();
/// console.log(topK); // [{hash: ..., count: ...}, ...]
///
/// // Estimate specific item
/// const count = hk.estimate(Buffer.from('frequent_item'));
/// console.log(count); // ~1000
/// ```
#[napi]
pub struct HeavyKeeper {
    inner: RustHeavyKeeper,
}

#[napi(object)]
pub struct HeavyKeeperResult {
    pub hash: BigInt,
    pub count: u32,
}

#[napi]
impl HeavyKeeper {
    /// Create a new HeavyKeeper sketch
    ///
    /// # Arguments
    /// * `k` - Number of top items to track
    /// * `epsilon` - Error bound (default: 0.001)
    /// * `delta` - Failure probability (default: 0.01)
    ///
    /// # Throws
    /// - If k is 0
    /// - If epsilon or delta are out of range (0, 1)
    ///
    /// # Example
    /// ```javascript
    /// const hk = new HeavyKeeper(100, 0.001, 0.01);
    /// ```
    #[napi(constructor)]
    pub fn new(k: u32, epsilon: Option<f64>, delta: Option<f64>) -> Result<Self> {
        let epsilon = epsilon.unwrap_or(0.001);
        let delta = delta.unwrap_or(0.01);

        RustHeavyKeeper::new(k as usize, epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("HeavyKeeper creation failed: {}", e),
                )
            })
    }

    /// Add an item to the sketch
    ///
    /// # Arguments
    /// * `item` - Binary data to track
    ///
    /// # Example
    /// ```javascript
    /// hk.update(Buffer.from('item1'));
    /// hk.update(Buffer.from('frequent_item'));
    /// ```
    #[napi]
    pub fn update(&mut self, item: Buffer) -> Result<()> {
        self.inner.update(&item);
        Ok(())
    }

    /// Estimate the frequency of an item
    ///
    /// # Arguments
    /// * `item` - Binary data to query
    ///
    /// # Returns
    /// Estimated count (may overestimate, never underestimates)
    ///
    /// # Example
    /// ```javascript
    /// const count = hk.estimate(Buffer.from('item1'));
    /// console.log(count); // e.g., 100
    /// ```
    #[napi]
    pub fn estimate(&self, item: Buffer) -> u32 {
        self.inner.estimate(&item)
    }

    /// Get the top-k heavy hitters
    ///
    /// # Returns
    /// Array of {hash, count} objects sorted by count descending
    ///
    /// # Example
    /// ```javascript
    /// const topK = hk.topK();
    /// for (const item of topK) {
    ///     console.log(`Hash: ${item.hash}, Count: ${item.count}`);
    /// }
    /// ```
    #[napi]
    pub fn topK(&self) -> Vec<HeavyKeeperResult> {
        self.inner
            .top_k()
            .into_iter()
            .map(|(hash, count)| HeavyKeeperResult {
                hash: BigInt::from(hash),
                count,
            })
            .collect()
    }

    /// Apply exponential decay to all counters
    ///
    /// Ages old items to make room for new heavy hitters.
    ///
    /// # Example
    /// ```javascript
    /// hk.decay(); // Apply decay
    /// ```
    #[napi]
    pub fn decay(&mut self) -> Result<()> {
        self.inner.decay();
        Ok(())
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "HeavyKeeper(k={}, estimates={})",
            self.inner.top_k().len(),
            self.inner.top_k().len()
        )
    }
}

/// RatelessIBLT - Efficient set reconciliation for distributed systems
///
/// Computes the symmetric difference between two sets without knowing
/// the difference size a priori. Used in blockchain, P2P networks, and
/// distributed databases.
///
/// # Example
/// ```javascript
/// const { RatelessIBLT } = require('@sketch-oxide/node');
///
/// // Create IBLTs for Alice and Bob
/// const alice = new RatelessIBLT(100, 32);
/// const bob = new RatelessIBLT(100, 32);
///
/// // Both insert shared items
/// alice.insert(Buffer.from('shared1'), Buffer.from('value1'));
/// bob.insert(Buffer.from('shared1'), Buffer.from('value1'));
///
/// // Alice has unique items
/// alice.insert(Buffer.from('alice_only'), Buffer.from('alice_value'));
///
/// // Bob has unique items
/// bob.insert(Buffer.from('bob_only'), Buffer.from('bob_value'));
///
/// // Compute difference
/// const diff = alice.clone();
/// diff.subtract(bob);
///
/// // Decode to recover items
/// const result = diff.decode();
/// console.log(result.toInsert); // Items in Alice but not Bob
/// console.log(result.toRemove); // Items in Bob but not Alice
/// ```
#[napi]
pub struct RatelessIBLT {
    inner: RustRatelessIBLT,
}

#[napi(object)]
pub struct IBLTDecodeResult {
    pub to_insert: Vec<KeyValuePair>,
    pub to_remove: Vec<KeyValuePair>,
    pub success: bool,
}

#[napi(object)]
pub struct KeyValuePair {
    pub key: Buffer,
    pub value: Buffer,
}

#[napi]
impl RatelessIBLT {
    /// Create a new RatelessIBLT
    ///
    /// # Arguments
    /// * `expectedDiff` - Expected size of symmetric difference
    /// * `cellSize` - Maximum size for cell data in bytes (typically 32-128)
    ///
    /// # Throws
    /// - If expectedDiff is 0
    /// - If cellSize is 0
    ///
    /// # Example
    /// ```javascript
    /// const iblt = new RatelessIBLT(100, 32);
    /// ```
    #[napi(constructor)]
    pub fn new(expected_diff: u32, cell_size: u32) -> Result<Self> {
        RustRatelessIBLT::new(expected_diff as usize, cell_size as usize)
            .map(|inner| Self { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("RatelessIBLT creation failed: {}", e),
                )
            })
    }

    /// Insert a key-value pair
    ///
    /// # Arguments
    /// * `key` - Key to insert
    /// * `value` - Value to insert
    ///
    /// # Example
    /// ```javascript
    /// iblt.insert(Buffer.from('key1'), Buffer.from('value1'));
    /// ```
    #[napi]
    pub fn insert(&mut self, key: Buffer, value: Buffer) -> Result<()> {
        self.inner
            .insert(&key, &value)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Insert failed: {}", e)))
    }

    /// Delete a key-value pair
    ///
    /// # Arguments
    /// * `key` - Key to delete
    /// * `value` - Value to delete
    ///
    /// # Example
    /// ```javascript
    /// iblt.delete(Buffer.from('key1'), Buffer.from('value1'));
    /// ```
    #[napi]
    pub fn delete(&mut self, key: Buffer, value: Buffer) -> Result<()> {
        self.inner
            .delete(&key, &value)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Delete failed: {}", e)))
    }

    /// Subtract another IBLT to compute symmetric difference
    ///
    /// # Arguments
    /// * `other` - Another RatelessIBLT to subtract
    ///
    /// # Throws
    /// - If IBLTs have incompatible parameters
    ///
    /// # Example
    /// ```javascript
    /// const diff = alice.clone();
    /// diff.subtract(bob);
    /// ```
    #[napi]
    pub fn subtract(&mut self, other: &RatelessIBLT) -> Result<()> {
        self.inner
            .subtract(&other.inner)
            .map_err(|e| Error::new(Status::InvalidArg, format!("Subtract failed: {}", e)))
    }

    /// Decode the IBLT to recover items
    ///
    /// # Returns
    /// Object with toInsert, toRemove arrays and success flag
    ///
    /// # Example
    /// ```javascript
    /// const result = iblt.decode();
    /// if (result.success) {
    ///     console.log('Recovered difference successfully');
    ///     console.log('To insert:', result.toInsert);
    ///     console.log('To remove:', result.toRemove);
    /// }
    /// ```
    #[napi]
    pub fn decode(&mut self) -> Result<IBLTDecodeResult> {
        match self.inner.decode() {
            Ok(diff) => Ok(IBLTDecodeResult {
                to_insert: diff
                    .to_insert
                    .into_iter()
                    .map(|(k, v)| KeyValuePair {
                        key: Buffer::from(k),
                        value: Buffer::from(v),
                    })
                    .collect(),
                to_remove: diff
                    .to_remove
                    .into_iter()
                    .map(|(k, v)| KeyValuePair {
                        key: Buffer::from(k),
                        value: Buffer::from(v),
                    })
                    .collect(),
                success: true,
            }),
            Err(_e) => Ok(IBLTDecodeResult {
                to_insert: vec![],
                to_remove: vec![],
                success: false,
            }),
        }
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        format!("RatelessIBLT(cells=...)")
    }
}

/// Grafite - Optimal range filter with adversarial-robust guarantees
///
/// The first optimal range filter providing provable FPR bounds of
/// L / 2^(B-2) where L is range width and B is bits per key.
///
/// # Example
/// ```javascript
/// const { Grafite } = require('@sketch-oxide/node');
///
/// // Build from sorted keys
/// const keys = [10n, 20n, 30n, 40n, 50n];
/// const filter = Grafite.build(keys, 6);
///
/// // Query ranges
/// console.log(filter.mayContainRange(15n, 25n)); // true (contains 20)
/// console.log(filter.mayContain(30n)); // true (exact match)
///
/// // Check FPR
/// const fpr = filter.expectedFpr(10n);
/// console.log(`Expected FPR: ${fpr}`); // 10 / 2^4 = 0.625
/// ```
#[napi]
pub struct Grafite {
    inner: RustGrafite,
}

#[napi(object)]
pub struct GrafiteStats {
    pub key_count: u32,
    pub bits_per_key: u32,
    pub total_bits: u32,
}

#[napi]
impl Grafite {
    /// Build a Grafite filter from sorted keys
    ///
    /// # Arguments
    /// * `keys` - Sorted array of 64-bit unsigned integers
    /// * `bitsPerKey` - Number of bits per key (typically 4-8)
    ///
    /// # Throws
    /// - If keys array is empty
    /// - If bitsPerKey is out of range (2-16)
    ///
    /// # Example
    /// ```javascript
    /// const keys = [100n, 200n, 300n, 400n, 500n];
    /// const filter = Grafite.build(keys, 6);
    /// ```
    #[napi(factory)]
    pub fn build(keys: Vec<BigInt>, bits_per_key: u32) -> Result<Self> {
        let keys_u64: Vec<u64> = keys.into_iter().map(|k| k.get_u64().1).collect();

        RustGrafite::build(&keys_u64, bits_per_key as usize)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::InvalidArg, format!("Grafite build failed: {}", e)))
    }

    /// Check if a range may contain keys
    ///
    /// # Arguments
    /// * `low` - Lower bound (inclusive)
    /// * `high` - Upper bound (inclusive)
    ///
    /// # Returns
    /// true if range may contain keys, false if definitely does not
    ///
    /// # Example
    /// ```javascript
    /// const mayContain = filter.mayContainRange(150n, 250n);
    /// ```
    #[napi]
    pub fn mayContainRange(&self, low: BigInt, high: BigInt) -> bool {
        let low_u64 = low.get_u64().1;
        let high_u64 = high.get_u64().1;
        self.inner.may_contain_range(low_u64, high_u64)
    }

    /// Check if a specific key may be present
    ///
    /// # Arguments
    /// * `key` - Key to check
    ///
    /// # Returns
    /// true if key may be present, false if definitely not
    ///
    /// # Example
    /// ```javascript
    /// const mayContain = filter.mayContain(200n);
    /// ```
    #[napi]
    pub fn mayContain(&self, key: BigInt) -> bool {
        let key_u64 = key.get_u64().1;
        self.inner.may_contain(key_u64)
    }

    /// Calculate expected false positive rate for a range width
    ///
    /// # Arguments
    /// * `rangeWidth` - Width of the query range
    ///
    /// # Returns
    /// Expected FPR = rangeWidth / 2^(bitsPerKey - 2)
    ///
    /// # Example
    /// ```javascript
    /// const fpr = filter.expectedFpr(100n);
    /// console.log(`FPR: ${fpr}`);
    /// ```
    #[napi]
    pub fn expectedFpr(&self, range_width: BigInt) -> f64 {
        let width = range_width.get_u64().1;
        self.inner.expected_fpr(width)
    }

    /// Get filter statistics
    ///
    /// # Returns
    /// Object with keyCount, bitsPerKey, totalBits
    ///
    /// # Example
    /// ```javascript
    /// const stats = filter.stats();
    /// console.log(`Keys: ${stats.keyCount}, Bits/key: ${stats.bitsPerKey}`);
    /// ```
    #[napi]
    pub fn stats(&self) -> GrafiteStats {
        let stats = self.inner.stats();
        GrafiteStats {
            key_count: stats.key_count as u32,
            bits_per_key: stats.bits_per_key as u32,
            total_bits: stats.total_bits as u32,
        }
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "Grafite(keys={}, bits_per_key={})",
            stats.key_count, stats.bits_per_key
        )
    }
}

/// MementoFilter - Dynamic range filter with FPR guarantees
///
/// The first dynamic range filter supporting insertions while maintaining
/// false positive rate guarantees. Combines a base range filter with a
/// quotient filter layer for precise element storage.
///
/// # Example
/// ```javascript
/// const { MementoFilter } = require('@sketch-oxide/node');
///
/// const filter = new MementoFilter(1000, 0.01);
///
/// // Insert key-value pairs dynamically
/// filter.insert(42n, Buffer.from('value1'));
/// filter.insert(100n, Buffer.from('value2'));
/// filter.insert(250n, Buffer.from('value3'));
///
/// // Query ranges
/// console.log(filter.mayContainRange(40n, 50n)); // true
/// console.log(filter.mayContainRange(500n, 600n)); // likely false
/// ```
#[napi]
pub struct MementoFilter {
    inner: RustMementoFilter,
}

#[napi]
impl MementoFilter {
    /// Create a new MementoFilter
    ///
    /// # Arguments
    /// * `expectedElements` - Expected number of elements
    /// * `fpr` - Target false positive rate (0 < fpr < 1)
    ///
    /// # Throws
    /// - If expectedElements is 0
    /// - If fpr is out of range (0, 1)
    ///
    /// # Example
    /// ```javascript
    /// const filter = new MementoFilter(1000, 0.01); // 1% FPR
    /// ```
    #[napi(constructor)]
    pub fn new(expected_elements: u32, fpr: f64) -> Result<Self> {
        RustMementoFilter::new(expected_elements as usize, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("MementoFilter creation failed: {}", e),
                )
            })
    }

    /// Insert a key-value pair
    ///
    /// # Arguments
    /// * `key` - 64-bit unsigned integer key
    /// * `value` - Value as binary data
    ///
    /// # Example
    /// ```javascript
    /// filter.insert(42n, Buffer.from('value1'));
    /// filter.insert(100n, Buffer.from('value2'));
    /// ```
    #[napi]
    pub fn insert(&mut self, key: BigInt, value: Buffer) -> Result<()> {
        let key_u64 = key.get_u64().1;
        self.inner
            .insert(key_u64, &value)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Insert failed: {}", e)))
    }

    /// Check if a range may contain keys
    ///
    /// # Arguments
    /// * `low` - Lower bound (inclusive)
    /// * `high` - Upper bound (inclusive)
    ///
    /// # Returns
    /// true if range may contain keys, false if definitely does not
    ///
    /// # Example
    /// ```javascript
    /// const mayContain = filter.mayContainRange(40n, 50n);
    /// ```
    #[napi]
    pub fn mayContainRange(&self, low: BigInt, high: BigInt) -> bool {
        let low_u64 = low.get_u64().1;
        let high_u64 = high.get_u64().1;
        self.inner.may_contain_range(low_u64, high_u64)
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        format!("MementoFilter(...)")
    }
}

/// SlidingHyperLogLog - Time-windowed cardinality estimation
///
/// Extends HyperLogLog with temporal awareness for cardinality estimation
/// over sliding time windows. Essential for real-time analytics, DDoS detection,
/// and streaming applications.
///
/// # Example
/// ```javascript
/// const { SlidingHyperLogLog } = require('@sketch-oxide/node');
///
/// // Create with precision 12, 1-hour max window
/// const hll = new SlidingHyperLogLog(12, 3600);
///
/// // Add items with timestamps
/// hll.update(Buffer.from('user_123'), 1000);
/// hll.update(Buffer.from('user_456'), 1030);
/// hll.update(Buffer.from('user_789'), 1060);
///
/// // Estimate cardinality in last 60 seconds
/// const estimate = hll.estimateWindow(1060, 60);
/// console.log(`Unique items in window: ${Math.round(estimate)}`);
///
/// // Decay old entries
/// hll.decay(2000, 600);
/// ```
#[napi]
pub struct SlidingHyperLogLog {
    inner: RustSlidingHyperLogLog,
}

#[napi]
impl SlidingHyperLogLog {
    /// Create a new SlidingHyperLogLog
    ///
    /// # Arguments
    /// * `precision` - Number of bits (4-16, typical 12-14)
    /// * `maxWindowSeconds` - Maximum window size in seconds
    ///
    /// # Throws
    /// - If precision is out of range (4-16)
    /// - If maxWindowSeconds is 0
    ///
    /// # Example
    /// ```javascript
    /// const hll = new SlidingHyperLogLog(12, 3600); // 1-hour window
    /// ```
    #[napi(constructor)]
    pub fn new(precision: u8, max_window_seconds: BigInt) -> Result<Self> {
        let max_window = max_window_seconds.get_u64().1;

        RustSlidingHyperLogLog::new(precision, max_window)
            .map(|inner| Self { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("SlidingHyperLogLog creation failed: {}", e),
                )
            })
    }

    /// Add an item with timestamp
    ///
    /// # Arguments
    /// * `item` - Binary data to add
    /// * `timestamp` - Unix timestamp in seconds
    ///
    /// # Example
    /// ```javascript
    /// hll.update(Buffer.from('user_id'), 1000);
    /// hll.update(Buffer.from('event'), Date.now() / 1000);
    /// ```
    #[napi]
    pub fn update(&mut self, item: Buffer, timestamp: BigInt) -> Result<()> {
        let ts = timestamp.get_u64().1;
        let data: Vec<u8> = item.to_vec();
        self.inner
            .update(&data, ts)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Update failed: {}", e)))
    }

    /// Estimate cardinality in a sliding window
    ///
    /// # Arguments
    /// * `currentTime` - Current timestamp in seconds
    /// * `windowSeconds` - Window size in seconds
    ///
    /// # Returns
    /// Estimated number of unique items in [currentTime - windowSeconds, currentTime]
    ///
    /// # Example
    /// ```javascript
    /// const now = Date.now() / 1000;
    /// const estimate = hll.estimateWindow(now, 60); // Last 60 seconds
    /// ```
    #[napi]
    pub fn estimateWindow(&self, current_time: BigInt, window_seconds: BigInt) -> f64 {
        let current = current_time.get_u64().1;
        let window = window_seconds.get_u64().1;
        self.inner.estimate_window(current, window)
    }

    /// Estimate total cardinality (all time)
    ///
    /// # Returns
    /// Estimated number of unique items ever seen
    ///
    /// # Example
    /// ```javascript
    /// const total = hll.estimateTotal();
    /// console.log(`Total unique items: ${Math.round(total)}`);
    /// ```
    #[napi]
    pub fn estimateTotal(&self) -> f64 {
        self.inner.estimate_total()
    }

    /// Remove expired entries outside the window
    ///
    /// # Arguments
    /// * `currentTime` - Current timestamp in seconds
    /// * `windowSeconds` - Window size in seconds
    ///
    /// # Example
    /// ```javascript
    /// const now = Date.now() / 1000;
    /// hll.decay(now, 3600); // Keep last hour
    /// ```
    #[napi]
    pub fn decay(&mut self, current_time: BigInt, window_seconds: BigInt) -> Result<()> {
        let current = current_time.get_u64().1;
        let window = window_seconds.get_u64().1;
        self.inner
            .decay(current, window)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Decay failed: {}", e)))
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "SlidingHyperLogLog(precision=..., estimate={:.0})",
            self.inner.estimate_total()
        )
    }
}

// ============================================================================
// TIER 2 SKETCHES (2025)
// ============================================================================

/// VacuumFilter: Best-in-class dynamic membership filter
///
/// Space-efficient filter supporting insertions AND deletions with
/// <15 bits/item at 1% FPR (better than Cuckoo and Counting Bloom).
///
/// # Use Cases
/// - Dynamic set membership with deletions
/// - Cache tracking with eviction
/// - Database deduplication
/// - Security: malicious URL tracking
///
/// # Examples
/// ```javascript
/// const { VacuumFilter } = require('@sketch-oxide/node');
///
/// const filter = new VacuumFilter(1000, 0.01);
/// filter.insert(Buffer.from('key1'));
/// console.log(filter.contains(Buffer.from('key1'))); // true
///
/// filter.delete(Buffer.from('key1'));
/// console.log(filter.contains(Buffer.from('key1'))); // false
///
/// const stats = filter.stats();
/// console.log(`Load factor: ${stats.loadFactor.toFixed(2)}`);
/// ```
#[napi]
pub struct VacuumFilter {
    inner: RustVacuumFilter,
}

#[napi]
impl VacuumFilter {
    /// Create a new VacuumFilter
    ///
    /// # Arguments
    /// * `capacity` - Expected number of elements
    /// * `fpr` - Target false positive rate (0 < fpr < 1)
    ///
    /// # Throws
    /// - If capacity is 0
    /// - If fpr is out of range
    ///
    /// # Example
    /// ```javascript
    /// const filter = new VacuumFilter(1000, 0.01); // 1000 items, 1% FPR
    /// ```
    #[napi(constructor)]
    pub fn new(capacity: u32, fpr: f64) -> Result<Self> {
        RustVacuumFilter::new(capacity as usize, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("VacuumFilter creation failed: {}", e),
                )
            })
    }

    /// Insert an element into the filter
    ///
    /// # Arguments
    /// * `key` - The key to insert
    ///
    /// # Example
    /// ```javascript
    /// filter.insert(Buffer.from('hello'));
    /// filter.insert(Buffer.from('world'));
    /// ```
    #[napi]
    pub fn insert(&mut self, key: Buffer) -> Result<()> {
        let data: Vec<u8> = key.to_vec();
        self.inner
            .insert(&data)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Insert failed: {}", e)))
    }

    /// Check if an element might be in the filter
    ///
    /// # Arguments
    /// * `key` - The key to check
    ///
    /// # Returns
    /// `true` if might be present (with FPR probability of false positive),
    /// `false` if definitely not present
    ///
    /// # Example
    /// ```javascript
    /// if (filter.contains(Buffer.from('hello'))) {
    ///   console.log('Key might be present');
    /// }
    /// ```
    #[napi]
    pub fn contains(&self, key: Buffer) -> bool {
        let data: Vec<u8> = key.to_vec();
        self.inner.contains(&data)
    }

    /// Delete an element from the filter
    ///
    /// # Arguments
    /// * `key` - The key to delete
    ///
    /// # Returns
    /// `true` if element was found and removed, `false` otherwise
    ///
    /// # Example
    /// ```javascript
    /// const wasDeleted = filter.delete(Buffer.from('hello'));
    /// console.log(`Deleted: ${wasDeleted}`);
    /// ```
    #[napi]
    pub fn delete(&mut self, key: Buffer) -> Result<bool> {
        let data: Vec<u8> = key.to_vec();
        self.inner
            .delete(&data)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Delete failed: {}", e)))
    }

    /// Get current load factor (0.0 to 1.0)
    ///
    /// # Example
    /// ```javascript
    /// const loadFactor = filter.loadFactor();
    /// console.log(`Filter is ${(loadFactor * 100).toFixed(1)}% full`);
    /// ```
    #[napi]
    pub fn loadFactor(&self) -> f64 {
        self.inner.load_factor()
    }

    /// Get total capacity
    ///
    /// # Example
    /// ```javascript
    /// console.log(`Capacity: ${filter.capacity()}`);
    /// ```
    #[napi]
    pub fn capacity(&self) -> u32 {
        self.inner.capacity() as u32
    }

    /// Get number of items currently stored
    ///
    /// # Example
    /// ```javascript
    /// console.log(`Items: ${filter.len()}`);
    /// ```
    #[napi]
    pub fn len(&self) -> u32 {
        self.inner.len() as u32
    }

    /// Check if filter is empty
    ///
    /// # Example
    /// ```javascript
    /// if (filter.isEmpty()) {
    ///   console.log('Filter is empty');
    /// }
    /// ```
    #[napi]
    pub fn isEmpty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get memory usage in bytes
    ///
    /// # Example
    /// ```javascript
    /// console.log(`Memory: ${filter.memoryUsage()} bytes`);
    /// ```
    #[napi]
    pub fn memoryUsage(&self) -> u32 {
        self.inner.memory_usage() as u32
    }

    /// Get filter statistics
    ///
    /// # Returns
    /// Object with capacity, numItems, loadFactor, memoryBits, fingerprintBits
    ///
    /// # Example
    /// ```javascript
    /// const stats = filter.stats();
    /// console.log(`Load: ${stats.loadFactor.toFixed(2)}`);
    /// console.log(`Memory: ${stats.memoryBits} bits`);
    /// ```
    #[napi]
    pub fn stats(&self) -> VacuumFilterStats {
        let rust_stats = self.inner.stats();
        VacuumFilterStats {
            capacity: rust_stats.capacity as u32,
            num_items: rust_stats.num_items as u32,
            load_factor: rust_stats.load_factor,
            memory_bits: rust_stats.memory_bits as i64,
            fingerprint_bits: rust_stats.fingerprint_bits,
        }
    }

    /// Clear all items from the filter
    ///
    /// # Example
    /// ```javascript
    /// filter.clear();
    /// console.log(filter.isEmpty()); // true
    /// ```
    #[napi]
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "VacuumFilter(capacity={}, items={}, load={:.2})",
            self.inner.capacity(),
            self.inner.len(),
            self.inner.load_factor()
        )
    }
}

#[napi(object)]
pub struct VacuumFilterStats {
    pub capacity: u32,
    pub num_items: u32,
    pub load_factor: f64,
    pub memory_bits: i64,
    pub fingerprint_bits: u8,
}

/// GRF (Gorilla Range Filter): Shape-based range filter for LSM-trees
///
/// Advanced range filter optimized for skewed distributions.
/// Uses shape encoding for 30-50% better FPR than Grafite on real data.
///
/// # Use Cases
/// - RocksDB/LevelDB SSTable filters
/// - Time-series databases (InfluxDB, TimescaleDB)
/// - Log aggregation systems
/// - Financial time-series data
///
/// # Examples
/// ```javascript
/// const { GRF } = require('@sketch-oxide/node');
///
/// // Build from Zipf-distributed keys
/// const keys = [1n, 2n, 3n, 5n, 8n, 13n, 21n];
/// const grf = GRF.build(keys, 6);
///
/// console.log(grf.mayContainRange(10n, 25n)); // true (contains 13, 21)
/// console.log(grf.mayContain(13n)); // true
///
/// const stats = grf.stats();
/// console.log(`Segments: ${stats.segmentCount}`);
/// ```
#[napi]
pub struct GRF {
    inner: RustGRF,
}

#[napi]
impl GRF {
    /// Build a GRF filter from sorted keys
    ///
    /// # Arguments
    /// * `keys` - Array of sorted 64-bit unsigned integers
    /// * `bitsPerKey` - Number of bits per key (2-16, typical 4-8)
    ///
    /// # Throws
    /// - If keys array is empty
    /// - If bitsPerKey is out of range
    ///
    /// # Example
    /// ```javascript
    /// const keys = [10n, 20n, 30n, 40n, 50n];
    /// const grf = GRF.build(keys, 6);
    /// ```
    #[napi(factory)]
    pub fn build(keys: Vec<BigInt>, bits_per_key: u32) -> Result<Self> {
        let rust_keys: Vec<u64> = keys.iter().map(|b| b.get_u64().1).collect();
        RustGRF::build(&rust_keys, bits_per_key as usize)
            .map(|inner| Self { inner })
            .map_err(|e| Error::new(Status::InvalidArg, format!("GRF build failed: {}", e)))
    }

    /// Check if a range may contain keys
    ///
    /// # Arguments
    /// * `low` - Lower bound (inclusive)
    /// * `high` - Upper bound (inclusive)
    ///
    /// # Returns
    /// `true` if range might contain keys, `false` if definitely does not
    ///
    /// # Example
    /// ```javascript
    /// if (grf.mayContainRange(15n, 25n)) {
    ///   console.log('Range might have keys');
    /// }
    /// ```
    #[napi]
    pub fn mayContainRange(&self, low: BigInt, high: BigInt) -> bool {
        let low_val = low.get_u64().1;
        let high_val = high.get_u64().1;
        self.inner.may_contain_range(low_val, high_val)
    }

    /// Check if a specific key may be present
    ///
    /// # Arguments
    /// * `key` - Key to check
    ///
    /// # Returns
    /// `true` if key might be present, `false` if definitely not
    ///
    /// # Example
    /// ```javascript
    /// if (grf.mayContain(30n)) {
    ///   console.log('Key might be present');
    /// }
    /// ```
    #[napi]
    pub fn mayContain(&self, key: BigInt) -> bool {
        let key_val = key.get_u64().1;
        self.inner.may_contain(key_val)
    }

    /// Calculate expected FPR for a range width
    ///
    /// # Arguments
    /// * `rangeWidth` - Width of the query range
    ///
    /// # Returns
    /// Expected false positive rate (0.0 to 1.0)
    ///
    /// # Example
    /// ```javascript
    /// const fpr = grf.expectedFpr(10n);
    /// console.log(`Expected FPR: ${(fpr * 100).toFixed(2)}%`);
    /// ```
    #[napi]
    pub fn expectedFpr(&self, range_width: BigInt) -> f64 {
        let width = range_width.get_u64().1;
        self.inner.expected_fpr(width)
    }

    /// Get filter statistics
    ///
    /// # Returns
    /// Object with keyCount, segmentCount, avgKeysPerSegment, bitsPerKey, totalBits, memoryBytes
    ///
    /// # Example
    /// ```javascript
    /// const stats = grf.stats();
    /// console.log(`Keys: ${stats.keyCount}, Segments: ${stats.segmentCount}`);
    /// ```
    #[napi]
    pub fn stats(&self) -> GRFStats {
        let rust_stats = self.inner.stats();
        GRFStats {
            key_count: rust_stats.key_count as u32,
            segment_count: rust_stats.segment_count as u32,
            avg_keys_per_segment: rust_stats.avg_keys_per_segment,
            bits_per_key: rust_stats.bits_per_key as u32,
            total_bits: rust_stats.total_bits as i64,
            memory_bytes: rust_stats.memory_bytes as u32,
        }
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "GRF(keys={}, segments={}, bits/key={})",
            stats.key_count, stats.segment_count, stats.bits_per_key
        )
    }
}

#[napi(object)]
pub struct GRFStats {
    pub key_count: u32,
    pub segment_count: u32,
    pub avg_keys_per_segment: f64,
    pub bits_per_key: u32,
    pub total_bits: i64,
    pub memory_bytes: u32,
}

/// NitroSketch: High-speed network telemetry with selective sampling
///
/// Achieves 100Gbps line rate through probabilistic sampling while
/// maintaining accuracy via background synchronization.
///
/// # Use Cases
/// - Network traffic monitoring at 100Gbps+
/// - DDoS detection
/// - Software-Defined Networking (SDN)
/// - Cloud telemetry
/// - Real-time analytics with CPU constraints
///
/// # Examples
/// ```javascript
/// const { NitroSketch, CountMinSketch } = require('@sketch-oxide/node');
///
/// const base = new CountMinSketch(0.01, 0.01);
/// const nitro = new NitroSketch(base, 0.1); // 10% sampling
///
/// // High-speed updates
/// for (let i = 0; i < 100000; i++) {
///   nitro.updateSampled(Buffer.from(`packet_${i % 100}`));
/// }
///
/// // Synchronize for accuracy
/// nitro.sync(1.0);
///
/// const stats = nitro.stats();
/// console.log(`Sampled: ${stats.sampledCount}, Total: ${stats.totalItemsEstimated}`);
/// ```
#[napi]
pub struct NitroSketch {
    inner: RustNitroSketch<RustCountMinSketch>,
}

#[napi]
impl NitroSketch {
    /// Create a new NitroSketch wrapping a CountMinSketch
    ///
    /// # Arguments
    /// * `baseSketch` - CountMinSketch to wrap
    /// * `sampleRate` - Probability of updating (0 < rate <= 1)
    ///   - 1.0 = update every item (no sampling)
    ///   - 0.1 = update 10% of items
    ///   - 0.01 = update 1% of items
    ///
    /// # Throws
    /// - If sampleRate is out of range
    ///
    /// # Example
    /// ```javascript
    /// const base = new CountMinSketch(0.01, 0.01);
    /// const nitro = new NitroSketch(base, 0.1);
    /// ```
    #[napi(constructor)]
    pub fn new(base_sketch: &CountMinSketch, sample_rate: f64) -> Result<Self> {
        // Clone the base sketch's inner representation
        let base_clone = base_sketch.inner.clone();
        RustNitroSketch::new(base_clone, sample_rate)
            .map(|inner| Self { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("NitroSketch creation failed: {}", e),
                )
            })
    }

    /// Update with selective sampling
    ///
    /// Uses hash-based sampling to decide whether to update the base sketch.
    ///
    /// # Arguments
    /// * `key` - The item to possibly add
    ///
    /// # Example
    /// ```javascript
    /// nitro.updateSampled(Buffer.from('flow_key'));
    /// ```
    #[napi]
    pub fn updateSampled(&mut self, key: Buffer) {
        let data: Vec<u8> = key.to_vec();
        self.inner.update_sampled(&data);
    }

    /// Query the frequency of a key
    ///
    /// For accurate results, call sync() periodically.
    ///
    /// # Arguments
    /// * `key` - The item to query
    ///
    /// # Returns
    /// Estimated frequency (may be underestimated if sync() not called)
    ///
    /// # Example
    /// ```javascript
    /// const freq = nitro.query(Buffer.from('key'));
    /// console.log(`Frequency: ${freq}`);
    /// ```
    #[napi]
    pub fn query(&self, key: Buffer) -> i64 {
        let data: Vec<u8> = key.to_vec();
        self.inner.query(&data) as i64
    }

    /// Synchronize to adjust for unsampled items
    ///
    /// Background synchronization adjusts the sketch to account for
    /// items that were not sampled, recovering accuracy.
    ///
    /// # Arguments
    /// * `unsampledWeight` - Weight to apply to unsampled items (typically 1.0)
    ///
    /// # Example
    /// ```javascript
    /// nitro.sync(1.0); // Adjust for unsampled items
    /// ```
    #[napi]
    pub fn sync(&mut self, unsampled_weight: f64) -> Result<()> {
        self.inner
            .sync(unsampled_weight)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Sync failed: {}", e)))
    }

    /// Get statistics about sampling
    ///
    /// # Returns
    /// Object with sampleRate, sampledCount, unsampledCount, totalItemsEstimated
    ///
    /// # Example
    /// ```javascript
    /// const stats = nitro.stats();
    /// console.log(`Sample rate: ${stats.sampleRate}`);
    /// console.log(`Sampled: ${stats.sampledCount}`);
    /// ```
    #[napi]
    pub fn stats(&self) -> NitroSketchStats {
        let rust_stats = self.inner.stats();
        NitroSketchStats {
            sample_rate: rust_stats.sample_rate,
            sampled_count: rust_stats.sampled_count as i64,
            unsampled_count: rust_stats.unsampled_count as i64,
            total_items_estimated: rust_stats.total_items_estimated as i64,
        }
    }

    /// Reset sampling statistics
    ///
    /// # Example
    /// ```javascript
    /// nitro.resetStats();
    /// ```
    #[napi]
    pub fn resetStats(&mut self) {
        self.inner.reset_stats();
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "NitroSketch(sampleRate={:.2}, sampled={}, unsampled={})",
            stats.sample_rate, stats.sampled_count, stats.unsampled_count
        )
    }
}

#[napi(object)]
pub struct NitroSketchStats {
    pub sample_rate: f64,
    pub sampled_count: i64,
    pub unsampled_count: i64,
    pub total_items_estimated: i64,
}

/// UnivMon: Universal sketch supporting multiple simultaneous metrics
///
/// A single UnivMon estimates L1/L2 norms, entropy, heavy hitters,
/// and change detection, eliminating need for multiple specialized sketches.
///
/// # Supported Metrics (from ONE sketch!)
/// 1. L1 Norm (sum of frequencies): Total traffic volume
/// 2. L2 Norm (sum of squared frequencies): Load balance
/// 3. Entropy (Shannon entropy): Distribution diversity
/// 4. Heavy Hitters: Most frequent items
/// 5. Change Detection: Temporal anomalies
/// 6. Flow Size Distribution: Per-flow statistics
///
/// # Use Cases
/// - Network monitoring (simultaneous bandwidth, flows, protocols)
/// - Cloud analytics (unified telemetry)
/// - Real-time anomaly detection
/// - Multi-tenant systems
///
/// # Examples
/// ```javascript
/// const { UnivMon } = require('@sketch-oxide/node');
///
/// const univmon = new UnivMon(1000000, 0.01, 0.01);
///
/// // Update with network packets
/// univmon.update(Buffer.from('192.168.1.1'), 1500);
/// univmon.update(Buffer.from('192.168.1.2'), 800);
///
/// // Query multiple metrics from SAME sketch
/// console.log(`Total traffic: ${univmon.estimateL1()}`);
/// console.log(`Load balance: ${univmon.estimateL2()}`);
/// console.log(`IP diversity: ${univmon.estimateEntropy()}`);
///
/// const topIPs = univmon.heavyHitters(0.1);
/// console.log(`Top IPs: ${topIPs.length}`);
/// ```
#[napi]
pub struct UnivMon {
    inner: RustUnivMon,
}

#[napi]
impl UnivMon {
    /// Create a new UnivMon sketch
    ///
    /// # Arguments
    /// * `maxStreamSize` - Expected maximum number of items (determines layers)
    /// * `epsilon` - Error parameter (0 < epsilon < 1)
    /// * `delta` - Failure probability (0 < delta < 1)
    ///
    /// # Throws
    /// - If maxStreamSize is 0
    /// - If epsilon or delta are out of range
    ///
    /// # Example
    /// ```javascript
    /// const univmon = new UnivMon(1000000, 0.01, 0.01); // 1M items, 1% error
    /// ```
    #[napi(constructor)]
    pub fn new(max_stream_size: i64, epsilon: f64, delta: f64) -> Result<Self> {
        RustUnivMon::new(max_stream_size as u64, epsilon, delta)
            .map(|inner| Self { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("UnivMon creation failed: {}", e),
                )
            })
    }

    /// Update the sketch with an item and value
    ///
    /// # Arguments
    /// * `item` - The item (e.g., IP address, user ID)
    /// * `value` - The value/weight (e.g., packet size, transaction amount)
    ///
    /// # Example
    /// ```javascript
    /// univmon.update(Buffer.from('192.168.1.1'), 1500);
    /// univmon.update(Buffer.from('user_123'), 99.99);
    /// ```
    #[napi]
    pub fn update(&mut self, item: Buffer, value: f64) -> Result<()> {
        let data: Vec<u8> = item.to_vec();
        self.inner
            .update(&data, value)
            .map_err(|e| Error::new(Status::GenericFailure, format!("Update failed: {}", e)))
    }

    /// Estimate L1 norm (sum of frequencies)
    ///
    /// # Returns
    /// Total sum of all values (e.g., total traffic volume)
    ///
    /// # Example
    /// ```javascript
    /// const totalTraffic = univmon.estimateL1();
    /// console.log(`Total: ${totalTraffic} bytes`);
    /// ```
    #[napi]
    pub fn estimateL1(&self) -> f64 {
        self.inner.estimate_l1()
    }

    /// Estimate L2 norm (sum of squared frequencies)
    ///
    /// # Returns
    /// L2 norm indicating distribution spread
    ///
    /// # Example
    /// ```javascript
    /// const l2 = univmon.estimateL2();
    /// console.log(`Load balance metric: ${l2}`);
    /// ```
    #[napi]
    pub fn estimateL2(&self) -> f64 {
        self.inner.estimate_l2()
    }

    /// Estimate Shannon entropy
    ///
    /// # Returns
    /// Entropy value indicating distribution diversity
    ///
    /// # Example
    /// ```javascript
    /// const entropy = univmon.estimateEntropy();
    /// console.log(`Distribution diversity: ${entropy.toFixed(2)}`);
    /// ```
    #[napi]
    pub fn estimateEntropy(&self) -> f64 {
        self.inner.estimate_entropy()
    }

    /// Get heavy hitters (most frequent items)
    ///
    /// # Arguments
    /// * `threshold` - Frequency threshold (0 < threshold <= 1)
    ///   - 0.1 = items with frequency >= 10% of total
    ///
    /// # Returns
    /// Array of heavy hitter hashes
    ///
    /// # Example
    /// ```javascript
    /// const topItems = univmon.heavyHitters(0.1); // Top 10% items
    /// console.log(`Found ${topItems.length} heavy hitters`);
    /// ```
    #[napi]
    pub fn heavyHitters(&self, threshold: f64) -> Vec<i64> {
        // UnivMon returns Vec<(Vec<u8>, f64)>, we hash the keys to i64
        self.inner
            .heavy_hitters(threshold)
            .into_iter()
            .map(|(key, _freq)| {
                let mut hasher = XxHash64::with_seed(0);
                key.hash(&mut hasher);
                hasher.finish() as i64
            })
            .collect()
    }

    /// Detect change between two UnivMon sketches
    ///
    /// # Arguments
    /// * `other` - Another UnivMon sketch to compare
    ///
    /// # Returns
    /// Change magnitude (higher = more change)
    ///
    /// # Example
    /// ```javascript
    /// const change = univmon1.detectChange(univmon2);
    /// if (change > 0.5) {
    ///   console.log('Significant distribution shift detected!');
    /// }
    /// ```
    #[napi]
    pub fn detectChange(&self, other: &UnivMon) -> f64 {
        self.inner.detect_change(&other.inner)
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "UnivMon(L1={:.0}, L2={:.0}, entropy={:.2})",
            self.inner.estimate_l1(),
            self.inner.estimate_l2(),
            self.inner.estimate_entropy()
        )
    }
}

/// LearnedBloomFilter: ML-enhanced membership testing
///
/// **EXPERIMENTAL** - Uses machine learning to achieve 70-80% memory
/// reduction compared to standard Bloom filters.
///
/// # WARNING
/// Do NOT use in security-critical applications. ML models can be
/// adversarially attacked to craft keys that fool the predictor.
///
/// # Use Cases (Non-security)
/// - In-memory caches (memory optimization)
/// - Database query optimization
/// - Data deduplication
/// - Analytics systems
///
/// # Examples
/// ```javascript
/// const { LearnedBloomFilter } = require('@sketch-oxide/node');
///
/// // Train on dataset
/// const keys = [];
/// for (let i = 0; i < 10000; i++) {
///   keys.push(Buffer.from(`key${i}`));
/// }
///
/// const filter = LearnedBloomFilter.new(keys, 0.01);
///
/// console.log(filter.contains(Buffer.from('key500'))); // true
/// console.log(filter.contains(Buffer.from('nonexistent'))); // probably false
///
/// const mem = filter.memoryUsage();
/// console.log(`Memory: ${mem} bytes (70-80% reduction)`);
/// ```
#[napi]
pub struct LearnedBloomFilter {
    inner: RustLearnedBloomFilter,
}

#[napi]
impl LearnedBloomFilter {
    /// Create a new LearnedBloomFilter
    ///
    /// # Arguments
    /// * `trainingKeys` - Keys to train on (must be members, at least 10 keys)
    /// * `fpr` - Target false positive rate (0 < fpr < 1)
    ///
    /// # Throws
    /// - If training data is empty or too small
    /// - If fpr is out of range
    ///
    /// # Example
    /// ```javascript
    /// const keys = [];
    /// for (let i = 0; i < 1000; i++) {
    ///   keys.push(Buffer.from(`key${i}`));
    /// }
    /// const filter = LearnedBloomFilter.new(keys, 0.01);
    /// ```
    #[napi(factory)]
    pub fn new(training_keys: Vec<Buffer>, fpr: f64) -> Result<Self> {
        let rust_keys: Vec<Vec<u8>> = training_keys.iter().map(|b| b.to_vec()).collect();
        RustLearnedBloomFilter::new(&rust_keys, fpr)
            .map(|inner| Self { inner })
            .map_err(|e| {
                Error::new(
                    Status::InvalidArg,
                    format!("LearnedBloomFilter creation failed: {}", e),
                )
            })
    }

    /// Check if a key might be in the set
    ///
    /// # Arguments
    /// * `key` - The key to check
    ///
    /// # Returns
    /// `true` if might be present (or false positive),
    /// `false` if definitely not present
    ///
    /// # Guarantees
    /// Zero false negatives: All training keys will return `true`
    ///
    /// # Example
    /// ```javascript
    /// if (filter.contains(Buffer.from('key1'))) {
    ///   console.log('Key might be present');
    /// }
    /// ```
    #[napi]
    pub fn contains(&self, key: Buffer) -> bool {
        let data: Vec<u8> = key.to_vec();
        self.inner.contains(&data)
    }

    /// Get memory usage in bytes
    ///
    /// # Example
    /// ```javascript
    /// console.log(`Memory: ${filter.memoryUsage()} bytes`);
    /// ```
    #[napi]
    pub fn memoryUsage(&self) -> u32 {
        self.inner.memory_usage() as u32
    }

    /// Get string representation
    #[napi]
    pub fn toString(&self) -> String {
        format!(
            "LearnedBloomFilter(EXPERIMENTAL, memory={}B)",
            self.inner.memory_usage()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hyperloglog_creation() {
        let hll = HyperLogLog::new(14);
        assert!(hll.is_ok());
    }

    #[test]
    fn test_hyperloglog_invalid_precision() {
        let hll = HyperLogLog::new(3);
        assert!(hll.is_err());
    }

    #[test]
    fn test_hyperloglog_update_and_estimate() {
        let mut hll = HyperLogLog::new(14).unwrap();
        let _ = hll.update(Buffer::from(vec![1, 2, 3]));
        let estimate = hll.estimate();
        assert!(estimate.is_ok());
        assert!(estimate.unwrap() > 0.0);
    }

    // UltraLogLog tests
    #[test]
    fn test_ultraloglog_creation() {
        let ull = UltraLogLog::new(12);
        assert!(ull.is_ok());
    }

    #[test]
    fn test_ultraloglog_invalid_precision() {
        let ull = UltraLogLog::new(3);
        assert!(ull.is_err());
        let ull = UltraLogLog::new(19);
        assert!(ull.is_err());
    }

    #[test]
    fn test_ultraloglog_update_and_estimate() {
        let mut ull = UltraLogLog::new(12).unwrap();
        let _ = ull.update(Buffer::from(vec![1, 2, 3]));
        let estimate = ull.estimate();
        assert!(estimate.is_ok());
        assert!(estimate.unwrap() > 0.0);
    }

    #[test]
    fn test_ultraloglog_is_empty() {
        let ull = UltraLogLog::new(12).unwrap();
        assert!(ull.isEmpty().unwrap());
    }

    // CpcSketch tests
    #[test]
    fn test_cpcsketch_creation() {
        let cpc = CpcSketch::new(11);
        assert!(cpc.is_ok());
    }

    #[test]
    fn test_cpcsketch_invalid_lgk() {
        let cpc = CpcSketch::new(3);
        assert!(cpc.is_err());
        let cpc = CpcSketch::new(27);
        assert!(cpc.is_err());
    }

    #[test]
    fn test_cpcsketch_update_and_estimate() {
        let mut cpc = CpcSketch::new(11).unwrap();
        let _ = cpc.update(Buffer::from(vec![1, 2, 3]));
        assert!(!cpc.isEmpty().unwrap());
    }

    #[test]
    fn test_cpcsketch_flavor() {
        let cpc = CpcSketch::new(11).unwrap();
        let flavor = cpc.flavor().unwrap();
        assert_eq!(flavor, "Empty");
    }

    // QSketch tests
    #[test]
    fn test_qsketch_creation() {
        let qs = QSketch::new(256);
        assert!(qs.is_ok());
    }

    #[test]
    fn test_qsketch_invalid_max_samples() {
        let qs = QSketch::new(16);
        assert!(qs.is_err());
    }

    #[test]
    fn test_qsketch_update_and_estimate() {
        let mut qs = QSketch::new(256).unwrap();
        let _ = qs.update(Buffer::from(vec![1, 2, 3]), 10.0);
        assert!(!qs.isEmpty().unwrap());
        assert!(qs.totalWeight().unwrap() > 0.0);
    }

    #[test]
    fn test_qsketch_weighted_cardinality() {
        let mut qs = QSketch::new(256).unwrap();
        let _ = qs.update(Buffer::from(b"item1".to_vec()), 100.0);
        let _ = qs.update(Buffer::from(b"item2".to_vec()), 200.0);
        let result = qs.estimateWeightedCardinality().unwrap();
        assert!(result.estimate > 0.0);
    }

    // ThetaSketch tests
    #[test]
    fn test_thetasketch_creation() {
        let theta = ThetaSketch::new(12);
        assert!(theta.is_ok());
    }

    #[test]
    fn test_thetasketch_invalid_lgk() {
        let theta = ThetaSketch::new(3);
        assert!(theta.is_err());
        let theta = ThetaSketch::new(27);
        assert!(theta.is_err());
    }

    #[test]
    fn test_thetasketch_update_and_estimate() {
        let mut theta = ThetaSketch::new(12).unwrap();
        let _ = theta.update(Buffer::from(vec![1, 2, 3]));
        assert!(!theta.isEmpty().unwrap());
        let estimate = theta.estimate().unwrap();
        assert!(estimate > 0.0);
    }

    #[test]
    fn test_thetasketch_set_operations() {
        let mut theta_a = ThetaSketch::new(12).unwrap();
        let mut theta_b = ThetaSketch::new(12).unwrap();

        // Add some items to A
        for i in 0..50u8 {
            let _ = theta_a.update(Buffer::from(vec![i]));
        }

        // Add overlapping items to B
        for i in 25..75u8 {
            let _ = theta_b.update(Buffer::from(vec![i]));
        }

        // Test union
        let union = theta_a.union(&theta_b).unwrap();
        assert!(union.estimate().unwrap() > 0.0);

        // Test intersection
        let intersection = theta_a.intersect(&theta_b).unwrap();
        assert!(intersection.estimate().unwrap() >= 0.0);

        // Test difference
        let difference = theta_a.difference(&theta_b).unwrap();
        assert!(difference.estimate().unwrap() >= 0.0);
    }

    #[test]
    fn test_thetasketch_jaccard_similarity() {
        let mut theta_a = ThetaSketch::new(12).unwrap();
        let mut theta_b = ThetaSketch::new(12).unwrap();

        // Add identical items to both
        for i in 0..50u8 {
            let _ = theta_a.update(Buffer::from(vec![i]));
            let _ = theta_b.update(Buffer::from(vec![i]));
        }

        let similarity = theta_a.jaccardSimilarity(&theta_b).unwrap();
        // Should be very close to 1.0 since sets are identical
        assert!(similarity > 0.9);
    }

    // ==========================================================================
    // Frequency Estimation Sketch Tests
    // ==========================================================================

    // CountMinSketch tests
    #[test]
    fn test_count_min_sketch_creation() {
        let cms = CountMinSketch::new(0.01, 0.01);
        assert!(cms.is_ok());
    }

    #[test]
    fn test_count_min_sketch_invalid_params() {
        let cms = CountMinSketch::new(0.0, 0.01);
        assert!(cms.is_err());
        let cms = CountMinSketch::new(0.01, 0.0);
        assert!(cms.is_err());
    }

    #[test]
    fn test_count_min_sketch_update_and_estimate() {
        let mut cms = CountMinSketch::new(0.01, 0.01).unwrap();
        let _ = cms.update(Buffer::from(vec![1, 2, 3]));
        let _ = cms.update(Buffer::from(vec![1, 2, 3]));
        let estimate = cms.estimate(Buffer::from(vec![1, 2, 3]));
        assert!(estimate.is_ok());
        assert!(estimate.unwrap() >= 2);
    }

    #[test]
    fn test_count_min_sketch_merge() {
        let mut cms1 = CountMinSketch::new(0.01, 0.01).unwrap();
        let mut cms2 = CountMinSketch::new(0.01, 0.01).unwrap();
        let _ = cms1.update(Buffer::from(b"test".to_vec()));
        let _ = cms2.update(Buffer::from(b"test".to_vec()));
        let result = cms1.merge(&cms2);
        assert!(result.is_ok());
        assert!(cms1.estimate(Buffer::from(b"test".to_vec())).unwrap() >= 2);
    }

    // CountSketch tests
    #[test]
    fn test_count_sketch_creation() {
        let cs = CountSketch::new(0.1, 0.01);
        assert!(cs.is_ok());
    }

    #[test]
    fn test_count_sketch_update_and_estimate() {
        let mut cs = CountSketch::new(0.1, 0.01).unwrap();
        let _ = cs.update(Buffer::from(vec![1, 2, 3]), 10);
        let estimate = cs.estimate(Buffer::from(vec![1, 2, 3]));
        assert!(estimate.is_ok());
        // Count Sketch is unbiased, so estimate should be close to 10
        assert!((estimate.unwrap() - 10).abs() <= 5);
    }

    #[test]
    fn test_count_sketch_negative_delta() {
        let mut cs = CountSketch::new(0.1, 0.01).unwrap();
        let _ = cs.update(Buffer::from(b"item".to_vec()), 20);
        let _ = cs.update(Buffer::from(b"item".to_vec()), -5);
        let estimate = cs.estimate(Buffer::from(b"item".to_vec())).unwrap();
        // Should be around 15
        assert!((estimate - 15).abs() <= 5);
    }

    // ConservativeCountMin tests
    #[test]
    fn test_conservative_count_min_creation() {
        let ccms = ConservativeCountMin::new(0.01, 0.01);
        assert!(ccms.is_ok());
    }

    #[test]
    fn test_conservative_count_min_with_dimensions() {
        let ccms = ConservativeCountMin::withDimensions(1000, 5);
        assert!(ccms.is_ok());
    }

    #[test]
    fn test_conservative_count_min_update_and_estimate() {
        let mut ccms = ConservativeCountMin::new(0.01, 0.01).unwrap();
        let _ = ccms.update(Buffer::from(b"apple".to_vec()));
        let _ = ccms.update(Buffer::from(b"apple".to_vec()));
        let estimate = ccms.estimate(Buffer::from(b"apple".to_vec())).unwrap();
        assert!(estimate >= 2);
    }

    #[test]
    fn test_conservative_count_min_update_count() {
        let mut ccms = ConservativeCountMin::new(0.01, 0.01).unwrap();
        let _ = ccms.updateCount(Buffer::from(b"item".to_vec()), 100);
        let estimate = ccms.estimate(Buffer::from(b"item".to_vec())).unwrap();
        assert!(estimate >= 100);
    }

    // SpaceSaving tests
    #[test]
    fn test_space_saving_creation() {
        let ss = SpaceSaving::new(0.01);
        assert!(ss.is_ok());
    }

    #[test]
    fn test_space_saving_with_capacity() {
        let ss = SpaceSaving::withCapacity(100);
        assert!(ss.is_ok());
    }

    #[test]
    fn test_space_saving_invalid_epsilon() {
        let ss = SpaceSaving::new(0.0);
        assert!(ss.is_err());
        let ss = SpaceSaving::new(1.0);
        assert!(ss.is_err());
    }

    #[test]
    fn test_space_saving_update_and_estimate() {
        let mut ss = SpaceSaving::new(0.01).unwrap();
        for _ in 0..100 {
            let _ = ss.update(Buffer::from(b"frequent".to_vec()));
        }
        let result = ss.estimate(Buffer::from(b"frequent".to_vec())).unwrap();
        assert!(result.is_some());
        let hh = result.unwrap();
        assert!(hh.lower_bound <= 100);
        assert!(hh.upper_bound >= 100);
    }

    #[test]
    fn test_space_saving_heavy_hitters() {
        let mut ss = SpaceSaving::new(0.01).unwrap();
        for _ in 0..1000 {
            let _ = ss.update(Buffer::from(b"common".to_vec()));
        }
        for _ in 0..10 {
            let _ = ss.update(Buffer::from(b"rare".to_vec()));
        }
        let heavy = ss.heavyHitters(0.5).unwrap();
        // "common" should be a heavy hitter with threshold 0.5
        assert!(!heavy.is_empty());
    }

    #[test]
    fn test_space_saving_top_k() {
        let mut ss = SpaceSaving::new(0.01).unwrap();
        for i in 1..=5u64 {
            for _ in 0..(i * 100) {
                let _ = ss.update(Buffer::from(vec![i as u8]));
            }
        }
        let top3 = ss.topK(3).unwrap();
        assert_eq!(top3.len(), 3);
    }

    // FrequentItems tests
    #[test]
    fn test_frequent_items_creation() {
        let fi = FrequentItems::new(100);
        assert!(fi.is_ok());
    }

    #[test]
    fn test_frequent_items_invalid_max_size() {
        let fi = FrequentItems::new(1);
        assert!(fi.is_err());
    }

    #[test]
    fn test_frequent_items_update_and_estimate() {
        let mut fi = FrequentItems::new(100).unwrap();
        let _ = fi.updateBy(Buffer::from(b"item".to_vec()), 50);
        let result = fi.getEstimate(Buffer::from(b"item".to_vec())).unwrap();
        assert!(result.is_some());
        let est = result.unwrap();
        assert!(est.lower_bound <= 50);
        assert!(est.upper_bound >= 50);
    }

    #[test]
    fn test_frequent_items_frequent_items() {
        let mut fi = FrequentItems::new(10).unwrap();
        let _ = fi.updateBy(Buffer::from(b"common".to_vec()), 1000);
        let _ = fi.updateBy(Buffer::from(b"rare".to_vec()), 5);
        let items = fi
            .frequentItems(FrequentItemsErrorType::NoFalsePositives)
            .unwrap();
        assert!(!items.is_empty());
    }

    #[test]
    fn test_frequent_items_merge() {
        let mut fi1 = FrequentItems::new(100).unwrap();
        let mut fi2 = FrequentItems::new(100).unwrap();
        let _ = fi1.updateBy(Buffer::from(b"a".to_vec()), 10);
        let _ = fi2.updateBy(Buffer::from(b"b".to_vec()), 20);
        let result = fi1.merge(&fi2);
        assert!(result.is_ok());
    }

    // ==========================================================================
    // Membership Testing Sketch Tests
    // ==========================================================================

    #[test]
    fn test_bloom_filter_creation() {
        let bf = BloomFilter::new(100, Some(0.01));
        assert!(bf.is_ok());
    }

    #[test]
    fn test_bloom_filter_invalid_params() {
        let bf = BloomFilter::new(0, Some(0.01));
        assert!(bf.is_err());
        let bf = BloomFilter::new(100, Some(0.0));
        assert!(bf.is_err());
    }

    #[test]
    fn test_bloom_filter_insert_contains() {
        let mut bf = BloomFilter::new(100, Some(0.01)).unwrap();
        bf.insert(Buffer::from(b"test".to_vec()));
        assert!(bf.contains(Buffer::from(b"test".to_vec())));
    }

    #[test]
    fn test_blocked_bloom_filter() {
        let mut bbf = BlockedBloomFilter::new(100, Some(0.01)).unwrap();
        bbf.insert(Buffer::from(b"key".to_vec()));
        assert!(bbf.contains(Buffer::from(b"key".to_vec())));
    }

    #[test]
    fn test_counting_bloom_filter() {
        let mut cbf = CountingBloomFilter::new(100, Some(0.01)).unwrap();
        cbf.insert(Buffer::from(b"key".to_vec()));
        assert!(cbf.contains(Buffer::from(b"key".to_vec())));
        cbf.remove(Buffer::from(b"key".to_vec()));
        assert!(!cbf.contains(Buffer::from(b"key".to_vec())));
    }

    #[test]
    fn test_cuckoo_filter() {
        let mut cf = CuckooFilter::new(100).unwrap();
        let _ = cf.insert(Buffer::from(b"item".to_vec()));
        assert!(cf.contains(Buffer::from(b"item".to_vec())));
        cf.remove(Buffer::from(b"item".to_vec()));
        assert!(!cf.contains(Buffer::from(b"item".to_vec())));
    }

    #[test]
    fn test_ribbon_filter() {
        let mut rf = RibbonFilter::new(100, Some(0.01)).unwrap();
        let _ = rf.insert(Buffer::from(b"key1".to_vec()));
        let _ = rf.insert(Buffer::from(b"key2".to_vec()));
        rf.build();
        assert!(rf.contains(Buffer::from(b"key1".to_vec())).unwrap());
        assert!(rf.contains(Buffer::from(b"key2".to_vec())).unwrap());
    }

    // ==========================================================================
    // Quantile Estimation Sketch Tests
    // ==========================================================================

    #[test]
    fn test_ddsketch_creation() {
        let dd = DDSketch::new(0.01);
        assert!(dd.is_ok());
    }

    #[test]
    fn test_ddsketch_invalid_accuracy() {
        let dd = DDSketch::new(0.0);
        assert!(dd.is_err());
        let dd = DDSketch::new(1.0);
        assert!(dd.is_err());
    }

    #[test]
    fn test_ddsketch_update_and_quantile() {
        let mut dd = DDSketch::new(0.01).unwrap();
        for i in 1..=100 {
            dd.update(i as f64);
        }
        assert_eq!(dd.count(), 100);
        let median = dd.quantile(0.5);
        assert!(median.is_some());
        // Median should be around 50
        assert!((median.unwrap() - 50.0).abs() < 10.0);
    }

    #[test]
    fn test_kll_sketch_creation() {
        let kll = KllSketch::new(Some(200));
        assert!(kll.is_ok());
    }

    #[test]
    fn test_kll_sketch_invalid_k() {
        let kll = KllSketch::new(Some(5));
        assert!(kll.is_err());
    }

    #[test]
    fn test_kll_sketch_update_and_quantile() {
        let mut kll = KllSketch::new(Some(200)).unwrap();
        for i in 0..1000 {
            kll.update(i as f64);
        }
        assert_eq!(kll.count(), 1000);
        let median = kll.quantile(0.5);
        assert!(median.is_some());
        // Median should be around 500
        assert!((median.unwrap() - 500.0).abs() < 100.0);
    }

    #[test]
    fn test_tdigest_creation() {
        let td = TDigest::new(Some(100.0));
        assert!(td.isEmpty());
    }

    #[test]
    fn test_tdigest_update_and_quantile() {
        let mut td = TDigest::new(Some(100.0));
        for i in 0..1000 {
            td.update(i as f64);
        }
        let median = td.quantile(0.5);
        // Median should be around 500
        assert!((median - 500.0).abs() < 50.0);
    }

    #[test]
    fn test_req_sketch_creation_hra() {
        let req = ReqSketch::new(32, ReqSketchMode::HighRankAccuracy);
        assert!(req.is_ok());
    }

    #[test]
    fn test_req_sketch_creation_lra() {
        let req = ReqSketch::new(32, ReqSketchMode::LowRankAccuracy);
        assert!(req.is_ok());
    }

    #[test]
    fn test_req_sketch_invalid_k() {
        let req = ReqSketch::new(3, ReqSketchMode::HighRankAccuracy);
        assert!(req.is_err());
    }

    #[test]
    fn test_req_sketch_exact_max() {
        let mut req = ReqSketch::new(32, ReqSketchMode::HighRankAccuracy).unwrap();
        for i in 1..=1000 {
            req.update(i as f64);
        }
        // In HRA mode, p100 should be exact
        let max = req.quantile(1.0);
        assert!(max.is_some());
        assert_eq!(max.unwrap(), 1000.0);
    }
}
