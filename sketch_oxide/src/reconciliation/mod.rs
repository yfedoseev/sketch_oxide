//! Set reconciliation algorithms
//!
//! Data structures for efficiently synchronizing sets between nodes
//! by transmitting only their differences.
//!
//! # Use Cases
//! - Distributed database synchronization
//! - P2P network state reconciliation
//! - Blockchain synchronization
//! - CDN cache invalidation
//! - Distributed file systems
//!
//! # Available Algorithms
//!
//! - [`RatelessIBLT`] - Invertible Bloom Lookup Table for set reconciliation
//!
//! # Theory
//!
//! Set reconciliation protocols allow two parties with sets A and B to:
//! 1. Exchange compact summaries of their sets
//! 2. Compute the symmetric difference (A △ B)
//! 3. Synchronize by transmitting only the differences
//!
//! This is more efficient than naive approaches when the set difference
//! is small relative to the set size.
//!
//! # Example
//! ```
//! use sketch_oxide::reconciliation::RatelessIBLT;
//! use sketch_oxide::common::Reconcilable;
//!
//! let mut alice = RatelessIBLT::new(100, 32).unwrap();
//! let mut bob = RatelessIBLT::new(100, 32).unwrap();
//!
//! alice.insert(b"shared1", b"value1").unwrap();
//! alice.insert(b"shared2", b"value2").unwrap();
//! alice.insert(b"alice_only", b"alice_value").unwrap();
//!
//! bob.insert(b"shared1", b"value1").unwrap();
//! bob.insert(b"shared2", b"value2").unwrap();
//! bob.insert(b"bob_only", b"bob_value").unwrap();
//!
//! // Compute difference
//! let mut diff = alice.clone();
//! diff.subtract(&bob).unwrap();
//!
//! let set_diff = diff.decode().unwrap();
//! // set_diff.to_insert contains items Bob needs
//! // set_diff.to_remove contains items Bob should remove
//! ```

mod rateless_iblt;

pub use rateless_iblt::{RatelessIBLT, RatelessIBLTStats};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify module exports work
        let iblt = RatelessIBLT::new(10, 32);
        assert!(iblt.is_ok());
    }
}
