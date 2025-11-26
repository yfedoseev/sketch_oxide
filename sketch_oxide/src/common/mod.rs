//! Common utilities, traits, and errors

mod error;
pub mod hash;
mod traits;
mod types;
pub mod validation;

pub use error::{Result, SketchError};
pub use traits::{Mergeable, RangeFilter, Reconcilable, Sketch, WindowedSketch};
pub use types::SetDifference;
