//! Graph-stream sketches.
//!
//! Summaries whose native object is a graph given as a stream of edges — answering edge,
//! node-degree, and (in later waves) reachability and subgraph queries in sublinear space.

mod agm_connectivity;
mod doulion;
mod fleet;
mod gss;
mod hyperanf;
mod mascot;
mod tcm;
mod thinkd;
mod triest;

pub use agm_connectivity::AgmConnectivity;
pub use doulion::Doulion;
pub use fleet::Fleet;
pub use gss::GssSketch;
pub use hyperanf::HyperAnf;
pub use mascot::Mascot;
pub use tcm::TcmSketch;
pub use thinkd::ThinkD;
pub use triest::Triest;
