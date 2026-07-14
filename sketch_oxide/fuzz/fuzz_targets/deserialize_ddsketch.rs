#![no_main]
//! Fuzz DDSketch deserialization against arbitrary bytes: it must return
//! `Err` on malformed input, never panic or over-allocate.
use libfuzzer_sys::fuzz_target;
use sketch_oxide::common::Sketch;
use sketch_oxide::quantiles::DDSketch;

fuzz_target!(|data: &[u8]| {
    let _ = DDSketch::deserialize(data);
});
