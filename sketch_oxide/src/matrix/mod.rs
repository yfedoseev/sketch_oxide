//! Matrix and linear-algebra sketches.
//!
//! Dimensionality reduction and low-rank / linear-algebra summaries: random projections
//! today, with Frequent Directions, TensorSketch and sketch-and-solve in later waves.

mod count_sketch_embedding;
mod jl;

pub use count_sketch_embedding::CountSketchEmbedding;
pub use jl::JohnsonLindenstrauss;
