#![no_main]
//! Fuzz BinaryFuseFilter deserialization: must reject wrapping segment
//! dimensions and truncated fingerprint arrays without panic.
use libfuzzer_sys::fuzz_target;
use sketch_oxide::common::Sketch;
use sketch_oxide::membership::BinaryFuseFilter;

fuzz_target!(|data: &[u8]| {
    let _ = BinaryFuseFilter::deserialize(data);
});
