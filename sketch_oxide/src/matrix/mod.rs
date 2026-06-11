//! Matrix and linear-algebra sketches.
//!
//! Dimensionality reduction and low-rank / linear-algebra summaries: random projections
//! today, with Frequent Directions, TensorSketch and sketch-and-solve in later waves.

mod count_sketch_embedding;
mod frequent_directions;
mod jl;
mod sliding_frequent_directions;

pub use count_sketch_embedding::CountSketchEmbedding;
pub use frequent_directions::FrequentDirections;
pub use jl::JohnsonLindenstrauss;
pub use sliding_frequent_directions::SlidingFrequentDirections;
