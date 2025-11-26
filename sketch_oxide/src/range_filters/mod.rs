//! Range-based filter algorithms
//!
//! Probabilistic data structures for range-based membership queries.
//! These filters can answer queries like "does the set contain any values
//! in the range [low, high]?" with space-efficient approximations.
//!
//! # Use Cases
//! - Database query optimization (range predicate pushdown)
//! - Spatial data indexing
//! - Time-series data filtering
//! - Multi-dimensional range queries
//!
//! # Available Filters
//!
//! ## Grafite (2024)
//! Optimal range filter with robust FPR bounds: FPR = L / 2^(B-2)
//! - LSM-tree range queries (RocksDB, LevelDB)
//! - Database index optimization
//! - Time-series databases
//! - Financial market data (range lookups on timestamps)
//!
//! ## GRF (Gorilla Range Filter - SIGMOD 2024)
//! Shape-based range filter optimized for LSM-tree workloads with skewed distributions.
//! - RocksDB/LevelDB SSTable filters
//! - Time-series databases (InfluxDB, TimescaleDB)
//! - Log aggregation systems (Elasticsearch, Loki)
//! - Columnar databases (Parquet, ORC)
//! - 30-50% better FPR than Grafite for skewed data
//!
//! ## Memento Filter (2025)
//! First dynamic range filter with insertion support while maintaining FPR guarantees.
//! - MongoDB WiredTiger integration
//! - RocksDB block filters
//! - Dynamic database indexes
//! - Log systems with streaming data
//!
//! # Example
//! ```
//! use sketch_oxide::range_filters::{Grafite, GRF, MementoFilter};
//! use sketch_oxide::common::RangeFilter;
//!
//! // Static Grafite filter
//! let keys = vec![10, 20, 30, 40, 50];
//! let grafite = Grafite::build(&keys, 6).unwrap();
//! assert!(grafite.may_contain_range(15, 25));
//!
//! // GRF filter (better for skewed distributions)
//! let grf = GRF::build(&keys, 6).unwrap();
//! assert!(grf.may_contain_range(15, 25));
//!
//! // Dynamic Memento filter
//! let mut memento = MementoFilter::new(1000, 0.01).unwrap();
//! memento.insert(42, b"value1").unwrap();
//! memento.insert(100, b"value2").unwrap();
//! assert!(memento.may_contain_range(40, 50));
//! ```

mod grafite;
mod grf;
mod memento_filter;

pub use grafite::{Grafite, GrafiteStats};
pub use grf::{GRFStats, GRF};
pub use memento_filter::{MementoFilter, MementoStats};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::RangeFilter;

    #[test]
    fn test_module_exports_grafite() {
        // Verify Grafite is properly exported
        let keys = vec![10, 20, 30];
        let filter = Grafite::build(&keys, 6).unwrap();
        assert!(filter.may_contain_range(15, 25));
    }

    #[test]
    fn test_module_exports_grf() {
        // Verify GRF is properly exported
        let keys = vec![10, 20, 30];
        let filter = GRF::build(&keys, 6).unwrap();
        assert!(filter.may_contain_range(15, 25));
    }

    #[test]
    fn test_module_exports_memento() {
        // Verify Memento Filter is properly exported
        let mut filter = MementoFilter::new(1000, 0.01).unwrap();
        filter.insert(42, b"value").unwrap();
        assert!(filter.may_contain_range(40, 50));
    }
}
