//! Similarity estimation algorithms
//!
//! This module provides algorithms for estimating similarity between sets/documents:
//!
//! - [`MinHash`]: Jaccard similarity estimation for sets (Broder 1997)
//! - [`SimHash`]: Near-duplicate detection via cosine similarity (Charikar 2002)
//!
//! # Choosing Between MinHash and SimHash
//!
//! | Feature | MinHash | SimHash |
//! |---------|---------|---------|
//! | **Similarity type** | Jaccard (set intersection) | Cosine (weighted features) |
//! | **Best for** | Sets, documents as bags of words | Text near-duplicates |
//! | **Space** | O(k) for k hash functions | O(1) - single 64-bit hash |
//! | **Detection range** | Can find 5%+ similarity | Best for Hamming distance 3-7 |
//! | **Speed** | O(k) per comparison | O(1) per comparison |
//! | **Weights** | No | Yes (weighted features) |

pub mod bbit_minhash;
pub mod c_minhash;
pub mod c_oph;
pub mod lsh_ensemble;
pub mod minhash;
pub mod minhash_lsh;
pub mod odd_sketch;
pub mod oph;
pub mod prob_min_hash;
pub mod simhash;
pub mod simhash_lsh;
pub mod super_min_hash;
pub mod weighted_minhash;

pub use bbit_minhash::BBitMinHash;
pub use c_minhash::CMinHash;
pub use c_oph::COph;
pub use lsh_ensemble::LshEnsemble;
pub use minhash::MinHash;
pub use minhash_lsh::MinHashLsh;
pub use odd_sketch::OddSketch;
pub use oph::OnePermutationHash;
pub use prob_min_hash::ProbMinHash;
pub use simhash::SimHash;
pub use simhash_lsh::{hamming_distance, SimHashLsh};
pub use super_min_hash::SuperMinHash;
pub use weighted_minhash::WeightedMinHash;
