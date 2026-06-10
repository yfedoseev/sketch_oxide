//! Common utilities, traits, and errors

mod error;
pub mod hash;
pub mod time;
mod traits;
mod types;
pub mod validation;

pub use error::{Result, SketchError};
pub use time::{Admission, LateDataPolicy, Temporal, TimeDomain, Timestamp, Watermark};
pub use traits::{Mergeable, RangeFilter, Reconcilable, Sketch, WindowedSketch};
pub use types::SetDifference;
