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

pub mod minhash;
pub mod simhash;

pub use minhash::MinHash;
pub use simhash::SimHash;
