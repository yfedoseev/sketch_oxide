//! Quantile estimation algorithms
//!
//! This module provides state-of-the-art algorithms for estimating quantiles
//! (percentiles) from streaming data.
//!
//! # Algorithms
//!
//! - [`DDSketch`] - Relative error guarantees (VLDB 2019)
//! - [`ReqSketch`] - Zero error at p100 (HRA) or p0 (LRA) (PODS 2021)
//!
//! # Choosing an Algorithm
//!
//! ## DDSketch
//!
//! **Use when:**
//! - You need relative error guarantees (error proportional to value)
//! - Your data spans multiple orders of magnitude
//! - You need to merge sketches from distributed systems
//! - You're tracking latencies, request sizes, or financial metrics
//!
//! **Characteristics:**
//! - Relative accuracy (e.g., 1% error)
//! - Fast merge operations
//! - Space: O(log(max/min))
//! - Production-proven (Datadog, ClickHouse, TimescaleDB)
//!
//! ## REQ Sketch
//!
//! **Use when:**
//! - You need **EXACT** tail quantiles (p100 or p0)
//! - Monitoring SLAs or detecting outliers
//! - Tracking maximum/minimum values with quantile distributions
//! - You need mergeable sketches with controlled space usage
//!
//! **Characteristics:**
//! - HRA mode: Zero error at p100 (maximum), optimized for p90+
//! - LRA mode: Zero error at p0 (minimum), optimized for p10-
//! - Relative error for other quantiles
//! - Space: O(k log(n/k)) where k is configurable
//! - Production-proven (Google BigQuery, Apache DataSketches)
//!
//! # Examples
//!
//! ## DDSketch Example
//!
//! ```
//! use sketch_oxide::quantiles::DDSketch;
//! use sketch_oxide::common::Sketch;
//!
//! let mut dd = DDSketch::new(0.01).unwrap(); // 1% relative error
//!
//! // Add measurements
//! for i in 1..=1000 {
//!     dd.update(&(i as f64));
//! }
//!
//! // Query quantiles
//! println!("Median: {}", dd.quantile(0.5).unwrap());
//! println!("p99: {}", dd.quantile(0.99).unwrap());
//! ```
//!
//! ## REQ Sketch Example
//!
//! ```
//! use sketch_oxide::quantiles::req::{ReqSketch, ReqMode};
//!
//! // HRA mode for exact maximum tracking
//! let mut req = ReqSketch::new(128, ReqMode::HighRankAccuracy).unwrap();
//!
//! // Add measurements
//! for i in 1..=10000 {
//!     req.update(i as f64);
//! }
//!
//! // p100 is EXACT (zero error)
//! assert_eq!(req.quantile(1.0), Some(10000.0));
//!
//! // Other high quantiles have relative error guarantees
//! let p99 = req.quantile(0.99).unwrap();
//! assert!(p99 >= 9800.0); // Close to true p99 = 9900
//! ```

mod ddsketch;
mod kll;
pub mod req;
mod spline_sketch;
mod tdigest;

pub use ddsketch::DDSketch;
pub use kll::{KllFloatSketch, KllSketch};
pub use req::{ReqMode, ReqSketch};
pub use spline_sketch::SplineSketch;
pub use tdigest::TDigest;
