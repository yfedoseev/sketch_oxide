#![no_main]
//! Fuzz KLL deserialization: hand-rolled parser with unchecked slices; must
//! return `Err` on malformed input rather than panicking.
use libfuzzer_sys::fuzz_target;
use sketch_oxide::common::Sketch;
use sketch_oxide::quantiles::KllSketch;

fuzz_target!(|data: &[u8]| {
    let _ = KllSketch::deserialize(data);
});
