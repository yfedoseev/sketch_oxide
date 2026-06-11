//! Matrix and linear-algebra sketches.
//!
//! Dimensionality reduction and low-rank / linear-algebra summaries: random projections
//! today, with Frequent Directions, TensorSketch and sketch-and-solve in later waves.

mod count_sketch_embedding;
mod dump_snapshots_fd;
mod frequent_directions;
mod jl;
mod robust_frequent_directions;
mod sliding_frequent_directions;

pub use count_sketch_embedding::CountSketchEmbedding;
pub use dump_snapshots_fd::DumpSnapshotsFd;
pub use frequent_directions::FrequentDirections;
pub use jl::JohnsonLindenstrauss;
pub use robust_frequent_directions::RobustFrequentDirections;
pub use sliding_frequent_directions::SlidingFrequentDirections;
