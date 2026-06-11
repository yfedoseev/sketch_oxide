//! Graph-stream sketches.
//!
//! Summaries whose native object is a graph given as a stream of edges — answering edge,
//! node-degree, and (in later waves) reachability and subgraph queries in sublinear space.

mod gss;
mod tcm;
mod triest;

pub use gss::GssSketch;
pub use tcm::TcmSketch;
pub use triest::Triest;
