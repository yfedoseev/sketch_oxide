//! Universal Monitoring Data Structures
//!
//! This module provides universal sketches that support multiple simultaneous metrics
//! from a single data structure, significantly reducing memory overhead compared to
//! maintaining separate specialized sketches.

mod coco_sketch;
mod omni_sketch;
mod univmon;

pub use coco_sketch::CocoSketch;
pub use omni_sketch::OmniSketch;
pub use univmon::{UnivMon, UnivMonStats};
