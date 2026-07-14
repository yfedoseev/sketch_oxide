#![no_main]
//! Fuzz SplineSketch deserialization: must reject malformed/oversized headers
//! without overflow, OOM, or panic.
use libfuzzer_sys::fuzz_target;
use sketch_oxide::common::Sketch;
use sketch_oxide::quantiles::SplineSketch;

fuzz_target!(|data: &[u8]| {
    let _ = SplineSketch::deserialize(data);
});
