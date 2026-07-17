//! Common utilities, traits, and errors

pub mod batch;
pub mod canonical;
pub mod capabilities;
pub mod cursor;
mod error;
pub mod hash;
pub mod simd;
pub mod time;
mod traits;
mod types;
pub mod validation;

pub use canonical::{CanonicalDecode, CanonicalEncode, STABLE_HASH_SEED, stable_hash};
pub use capabilities::{
    CardinalityEstimate, Filter, PointQuery, QuantileQuery, Serializable, Update,
};
pub use cursor::{Framing, MAGIC, ReadCursor, SketchId, WriteBuf};
pub use error::{Result, SketchError};
pub use hash::{Salt, keyed_hash};
pub use time::{Admission, LateDataPolicy, Temporal, TimeDomain, Timestamp, Watermark};
pub use traits::{Mergeable, RangeFilter, Reconcilable, Sketch, WindowedSketch};
pub use types::SetDifference;
