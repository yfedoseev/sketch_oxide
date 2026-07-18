//! Vector-quantization sketches (fable5 doc 03 B.1 / C.7).
//!
//! Compress high-dimensional vectors into compact codes while preserving inner
//! products and distances with a theoretical error bound — the family that won
//! 2024–2026 in vector databases (Milvus IVF_RABITQ, Elastic/Lucene BBQ).

mod rabitq;

pub use rabitq::{PreparedQuery, RaBitQ, RaBitQCode};
