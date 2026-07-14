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
mod dyadic_count_sketch;
mod gk;
mod kll;
mod moments_sketch;
mod otel_histogram;
pub mod otlp;
mod per_flow_quantiles;
mod per_key;
mod q_digest;
pub mod req;
mod sketch_polymer;
mod spline_sketch;
mod tdigest;
mod udd_sketch;

pub use ddsketch::DDSketch;
pub use dyadic_count_sketch::DyadicCountSketch;
pub use gk::GreenwaldKhanna;
pub use kll::{KllFloatSketch, KllSketch};
pub use moments_sketch::MomentsSketch;
pub use otel_histogram::OtelExponentialHistogram;
pub use otlp::{
    OtlpBuckets, OtlpExponentialHistogramDataPoint, PrometheusNativeHistogram, PrometheusSpan,
};
pub use per_flow_quantiles::PerFlowQuantiles;
pub use per_key::PerKeyQuantiles;
pub use q_digest::QDigest;
pub use req::{ReqMode, ReqSketch};
pub use sketch_polymer::SketchPolymer;
pub use spline_sketch::SplineSketch;
pub use tdigest::TDigest;
pub use udd_sketch::UddSketch;

#[cfg(test)]
mod capability_smoke_tests {
    use crate::common::capabilities::{QuantileQuery, Update};
    use crate::quantiles::{GreenwaldKhanna, ReqMode, ReqSketch, UddSketch};

    // A pipeline generic over ANY quantile sketch that ingests `f64` and answers
    // immutable rank queries purely through the capability traits — the
    // composition the `Sketch`-trait split is meant to enable.
    fn median_via_capabilities<S: Update<f64> + QuantileQuery>(sketch: &mut S) -> f64 {
        for i in 1..=1000 {
            sketch.update(&(i as f64));
        }
        QuantileQuery::quantile(sketch, 0.5).expect("non-empty sketch has a median")
    }

    #[test]
    fn quantile_capability_traits_compose_across_types() {
        let mut gk = GreenwaldKhanna::new(0.01).unwrap();
        let gk_median = median_via_capabilities(&mut gk);
        assert!(
            (gk_median - 500.0).abs() <= 50.0,
            "GK median off: {gk_median}"
        );

        let mut req = ReqSketch::new(128, ReqMode::HighRankAccuracy).unwrap();
        let req_median = median_via_capabilities(&mut req);
        assert!(
            (req_median - 500.0).abs() <= 50.0,
            "REQ median off: {req_median}"
        );

        let mut udd = UddSketch::new(0.01, 256).unwrap();
        let udd_median = median_via_capabilities(&mut udd);
        assert!(
            (udd_median - 500.0).abs() / 500.0 <= 0.05,
            "UDD median off: {udd_median}"
        );
    }
}
