//! Network-measurement sketches.
//!
//! Sketches whose native problem is network telemetry over `(key, value)` traffic — per-key
//! distinct counting (super-spreaders), per-flow size, and related measurements under tight
//! per-packet memory budgets.

mod beaucoup;

pub use beaucoup::BeauCoup;
