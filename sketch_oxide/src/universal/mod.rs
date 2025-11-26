//! Universal Monitoring Data Structures
//!
//! This module provides universal sketches that support multiple simultaneous metrics
//! from a single data structure, significantly reducing memory overhead compared to
//! maintaining separate specialized sketches.

mod univmon;

pub use univmon::{UnivMon, UnivMonStats};
