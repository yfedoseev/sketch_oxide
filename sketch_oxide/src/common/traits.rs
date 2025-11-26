//! Core traits for all sketch implementations

use super::error::SketchError;
use super::types::SetDifference;
use std::hash::Hash;

/// Core trait that all sketches must implement
///
/// This trait defines the fundamental operations that any data sketch must support:
/// updating with new data, estimating results, checking emptiness, and serialization.
///
/// # SOLID Principles
/// - **Single Responsibility**: This trait focuses solely on core sketch operations
/// - **Interface Segregation**: Minimal interface - only essential operations
/// - **Liskov Substitution**: All implementations must maintain the contract
pub trait Sketch {
    /// The type of items this sketch can process
    type Item;

    /// Update the sketch with a new item
    ///
    /// # Arguments
    /// * `item` - The item to add to the sketch
    fn update(&mut self, item: &Self::Item);

    /// Get the estimated result from the sketch
    ///
    /// The meaning of this value depends on the sketch type:
    /// - Cardinality sketches: estimated number of unique items
    /// - Quantile sketches: varies by query
    /// - Frequency sketches: varies by query
    ///
    /// # Returns
    /// The estimated value as a float
    fn estimate(&self) -> f64;

    /// Check if the sketch is empty (no items added)
    ///
    /// # Returns
    /// `true` if the sketch has not processed any items
    fn is_empty(&self) -> bool;

    /// Serialize the sketch to bytes
    ///
    /// # Returns
    /// A byte vector containing the serialized sketch
    fn serialize(&self) -> Vec<u8>;

    /// Deserialize a sketch from bytes
    ///
    /// # Arguments
    /// * `bytes` - The byte slice containing serialized sketch data
    ///
    /// # Returns
    /// Result containing the deserialized sketch or an error
    ///
    /// # Errors
    /// Returns `SketchError::DeserializationError` if bytes are invalid
    fn deserialize(bytes: &[u8]) -> Result<Self, SketchError>
    where
        Self: Sized;
}

/// Trait for sketches that support merging
///
/// This trait extends `Sketch` with the ability to merge two sketches together.
/// Merging is essential for distributed computing scenarios where sketches
/// are computed independently and then combined.
///
/// # SOLID Principles
/// - **Interface Segregation**: Separate trait for mergeable sketches
///   (not all sketches can be merged, e.g., immutable Binary Fuse Filters)
pub trait Mergeable: Sketch {
    /// Merge another sketch into this one
    ///
    /// After merging, this sketch should represent the union of both sketches.
    ///
    /// # Arguments
    /// * `other` - The sketch to merge into this one
    ///
    /// # Returns
    /// `Ok(())` if merge was successful, or an error if sketches are incompatible
    ///
    /// # Errors
    /// Returns `SketchError::IncompatibleSketches` if:
    /// - Sketches have different configurations (e.g., different precision)
    /// - Sketches are of incompatible types
    fn merge(&mut self, other: &Self) -> Result<(), SketchError>;
}

/// Trait for filters that support range-based queries
///
/// This trait is designed for data structures that can answer queries about
/// whether a range of values might be present in the set. Useful for range
/// filters, spatial indexes, and ordered set approximations.
///
/// # Use Cases
/// - Range Bloom Filters for database range queries
/// - Spatial data structures for geometric queries
/// - Time-series filters for temporal range checks
///
/// # Example
/// ```
/// use sketch_oxide::common::RangeFilter;
///
/// struct SimpleRangeFilter {
///     min: u64,
///     max: u64,
/// }
///
/// impl RangeFilter for SimpleRangeFilter {
///     fn may_contain_range(&self, low: u64, high: u64) -> bool {
///         // Check if query range overlaps with filter range
///         !(high < self.min || low > self.max)
///     }
/// }
///
/// let filter = SimpleRangeFilter { min: 10, max: 100 };
/// assert!(filter.may_contain_range(50, 60));  // Overlaps
/// assert!(!filter.may_contain_range(200, 300)); // No overlap
/// ```
pub trait RangeFilter {
    /// Check if a range of values might be in the set
    ///
    /// Returns `true` if the range [low, high] might contain elements in the set.
    /// Returns `false` if the range definitely does not contain elements.
    ///
    /// # Arguments
    /// * `low` - Lower bound of the range (inclusive)
    /// * `high` - Upper bound of the range (inclusive)
    ///
    /// # Returns
    /// - `true` if the range might contain elements (may have false positives)
    /// - `false` if the range definitely does not contain elements (no false negatives)
    ///
    /// # Guarantees
    /// - **No false negatives**: If the method returns `false`, the range
    ///   definitely does not contain any elements
    /// - **May have false positives**: If it returns `true`, the range might
    ///   not actually contain elements
    fn may_contain_range(&self, low: u64, high: u64) -> bool;
}

/// Trait for data structures that support set reconciliation
///
/// Set reconciliation is the process of synchronizing two sets by computing
/// and transmitting only their differences. This is essential for distributed
/// systems, P2P networks, and database synchronization.
///
/// # Use Cases
/// - IBLT (Invertible Bloom Lookup Tables) for network synchronization
/// - Distributed database reconciliation
/// - P2P blockchain synchronization
/// - CDN cache invalidation
///
/// # Mathematical Background
/// For sets A and B, reconciliation involves finding:
/// - A \ B (elements in A but not in B) - to_insert for B
/// - B \ A (elements in B but not in A) - to_remove from B
///
/// # Example
/// ```ignore
/// use sketch_oxide::common::{Reconcilable, SetDifference, Result};
///
/// #[derive(Clone)]
/// struct SimpleSet {
///     items: Vec<Vec<u8>>,
/// }
///
/// impl Reconcilable for SimpleSet {
///     fn subtract(&mut self, other: &Self) -> Result<()> {
///         self.items.retain(|item| !other.items.contains(item));
///         Ok(())
///     }
///
///     fn decode(&self) -> Result<SetDifference> {
///         Ok(SetDifference {
///             to_insert: self.items.clone(),
///             to_remove: Vec::new(),
///         })
///     }
/// }
/// ```
pub trait Reconcilable: Sized {
    /// Subtract another set from this one
    ///
    /// Modifies this set to remove elements present in `other`.
    /// This implements the set difference operation: self = self \ other
    ///
    /// # Arguments
    /// * `other` - The set to subtract from this one
    ///
    /// # Returns
    /// `Ok(())` on success, or an error if subtraction fails
    ///
    /// # Errors
    /// Returns `SketchError::ReconciliationError` if:
    /// - Sets are incompatible for subtraction
    /// - Operation would result in invalid state
    fn subtract(&mut self, other: &Self) -> Result<(), SketchError>;

    /// Decode the set difference
    ///
    /// Extracts the set difference information from this reconcilable structure,
    /// returning which elements should be inserted and which should be removed.
    ///
    /// # Returns
    /// A `SetDifference` containing elements to insert and remove
    ///
    /// # Errors
    /// Returns `SketchError::DeserializationError` if:
    /// - The structure cannot be decoded
    /// - The structure is in an invalid state
    fn decode(&self) -> Result<SetDifference, SketchError>;
}

/// Trait for sketches that operate on time-windowed data
///
/// Time-windowed sketches maintain statistics over sliding time windows,
/// essential for real-time streaming analytics and monitoring.
///
/// # Use Cases
/// - Real-time dashboards showing last N minutes of data
/// - Alert systems based on recent events
/// - Time-decaying bloom filters
/// - Streaming anomaly detection
///
/// # Design Considerations
/// - Implementations should efficiently handle window expiration
/// - Timestamps should be monotonically increasing for best results
/// - Window estimates may be approximate depending on implementation
///
/// # Example
/// ```
/// use sketch_oxide::common::WindowedSketch;
/// use std::hash::Hash;
///
/// #[derive(Hash, Clone)]
/// struct Event {
///     id: u64,
/// }
///
/// struct EventCounter {
///     events: Vec<(u64, u64)>, // (item_hash, timestamp)
/// }
///
/// impl WindowedSketch for EventCounter {
///     type Item = Event;
///
///     fn update_with_timestamp(&mut self, item: Self::Item, timestamp: u64) {
///         use std::collections::hash_map::DefaultHasher;
///         use std::hash::Hasher;
///         let mut hasher = DefaultHasher::new();
///         item.hash(&mut hasher);
///         self.events.push((hasher.finish(), timestamp));
///     }
///
///     fn estimate_window(&self, current_time: u64, window_seconds: u64) -> f64 {
///         let cutoff = current_time.saturating_sub(window_seconds);
///         self.events.iter().filter(|(_, ts)| *ts >= cutoff).count() as f64
///     }
/// }
/// ```
pub trait WindowedSketch {
    /// The type of items this windowed sketch processes
    ///
    /// Must implement `Hash` for efficient item processing
    type Item: Hash;

    /// Update the sketch with an item and its timestamp
    ///
    /// # Arguments
    /// * `item` - The item to add to the sketch
    /// * `timestamp` - Unix timestamp (seconds since epoch) when item was observed
    ///
    /// # Performance
    /// Implementations should handle timestamp-based expiration efficiently,
    /// potentially using techniques like exponential bucketing or circular buffers.
    fn update_with_timestamp(&mut self, item: Self::Item, timestamp: u64);

    /// Estimate a value over a time window
    ///
    /// Computes an estimate (e.g., count, cardinality) for items observed
    /// within the specified time window ending at `current_time`.
    ///
    /// # Arguments
    /// * `current_time` - The end of the time window (Unix timestamp)
    /// * `window_seconds` - The size of the window in seconds
    ///
    /// # Returns
    /// An estimated value for the window. The meaning depends on the sketch type:
    /// - Count sketches: number of items in window
    /// - Cardinality sketches: unique items in window
    /// - Frequency sketches: varies by implementation
    ///
    /// # Example
    /// ```ignore
    /// // Count items from last 5 minutes (300 seconds)
    /// let current_time = 1000;
    /// let count = sketch.estimate_window(current_time, 300);
    /// ```
    fn estimate_window(&self, current_time: u64, window_seconds: u64) -> f64;
}
