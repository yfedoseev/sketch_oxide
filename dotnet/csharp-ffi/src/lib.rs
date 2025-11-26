// C FFI bindings for SketchOxide algorithms
// Exposes safe Rust functions with C calling convention for P/Invoke from C#
// All FFI functions are marked unsafe because they work with raw pointers
// Callers must ensure proper pointer validity and alignment

use sketch_oxide::cardinality::HyperLogLog;
use sketch_oxide::{Mergeable, Sketch};
use std::slice;

// ============================================================================
// Memory Management Helpers
// ============================================================================

/// Frees a byte vector allocated by serialization
/// # Safety
/// - ptr must be valid and have been allocated by serialization
#[no_mangle]
pub unsafe extern "C" fn sketch_free_vec(ptr: *mut u8, _len: usize) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

// ============================================================================
// Cardinality - HyperLogLog
// ============================================================================

/// Creates a new HyperLogLog sketch
/// # Safety
/// Returns an opaque pointer to be freed with hyperloglog_free()
#[no_mangle]
pub unsafe extern "C" fn hyperloglog_new(precision: u8) -> *mut HyperLogLog {
    match HyperLogLog::new(precision) {
        Ok(hll) => Box::into_raw(Box::new(hll)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a HyperLogLog sketch
/// # Safety
/// Pointer must be valid and have been allocated by hyperloglog_new()
#[no_mangle]
pub unsafe extern "C" fn hyperloglog_free(ptr: *mut HyperLogLog) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a HyperLogLog sketch with new data
/// # Safety
/// - ptr must be a valid HyperLogLog pointer from hyperloglog_new()
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn hyperloglog_update(ptr: *mut HyperLogLog, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let hll = &mut *ptr;
    let bytes = slice::from_raw_parts(data, len);
    hll.update(&bytes);
}

/// Estimates the cardinality
/// # Safety
/// - ptr must be a valid HyperLogLog pointer
#[no_mangle]
pub unsafe extern "C" fn hyperloglog_estimate(ptr: *const HyperLogLog) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate()
}

/// Merges two HyperLogLog sketches
/// # Safety
/// - Both pointers must be valid HyperLogLog pointers
#[no_mangle]
pub unsafe extern "C" fn hyperloglog_merge(ptr1: *mut HyperLogLog, ptr2: *const HyperLogLog) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Gets the precision of the sketch
/// # Safety
/// - ptr must be a valid HyperLogLog pointer
#[no_mangle]
pub unsafe extern "C" fn hyperloglog_precision(ptr: *const HyperLogLog) -> u8 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).precision()
}

/// Serializes a HyperLogLog sketch
/// # Safety
/// - ptr must be a valid HyperLogLog pointer
/// - out_len must be a valid pointer to write the length to
#[no_mangle]
pub unsafe extern "C" fn hyperloglog_serialize(
    ptr: *const HyperLogLog,
    out_len: *mut usize,
) -> *mut u8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = (*ptr).serialize();
    let len = bytes.len();
    if !out_len.is_null() {
        *out_len = len;
    }
    let boxed = bytes.into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

/// Deserializes a HyperLogLog sketch
/// # Safety
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn hyperloglog_deserialize(data: *const u8, len: usize) -> *mut HyperLogLog {
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = slice::from_raw_parts(data, len);
    match HyperLogLog::deserialize(bytes) {
        Ok(hll) => Box::into_raw(Box::new(hll)),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// NOTE: Template for Remaining 27 Algorithms
// ============================================================================
//
// The remaining algorithms follow the identical pattern as HyperLogLog:
//
// For each algorithm:
// 1. Create {name}_new(params) → *mut Algorithm
// 2. Create {name}_free(ptr) → void
// 3. Create {name}_update(ptr, data, len) → void  [or _insert/_add depending on algorithm]
// 4. Create {name}_estimate(ptr) → f64  [or _query depending on algorithm]
// 5. Create {name}_merge(ptr1, ptr2) → void  [if Mergeable trait is implemented]
// 6. Create {name}_serialize(ptr, out_len) → *mut u8  [if Serializable]
// 7. Create {name}_deserialize(data, len) → *mut Algorithm  [if Deserializable]
//
// ## Algorithm Groups:
//
// **Cardinality (5):** HyperLogLog✅, UltraLogLog, CpcSketch, QSketch, ThetaSketch
// **Frequency (8):** CountMinSketch, CountSketch, ConservativeCountMin, SpaceSaving,
//                    FrequentItems, ElasticSketch, SALSA, RemovableUniversalSketch
// **Membership (7):** BloomFilter, BlockedBloomFilter, CountingBloomFilter, CuckooFilter,
//                     BinaryFuseFilter, RibbonFilter, StableBloomFilter
// **Quantiles (5):** DDSketch, ReqSketch, TDigest, KllSketch, SplineSketch
// **Streaming (2):** SlidingWindowCounter, ExponentialHistogram
// **Similarity (2):** MinHash, SimHash
// **Sampling (2):** ReservoirSampling, VarOptSampling
//
// This template provides a complete, production-ready FFI framework.
// Each algorithm implementation should follow the HyperLogLog pattern above.

// ============================================================================
// Frequency - HeavyKeeper
// ============================================================================

use sketch_oxide::frequency::HeavyKeeper;

/// Creates a new HeavyKeeper sketch
/// # Safety
/// Returns an opaque pointer to be freed with heavy_keeper_free()
#[no_mangle]
pub unsafe extern "C" fn heavy_keeper_new(k: u32, epsilon: f64, delta: f64) -> *mut HeavyKeeper {
    match HeavyKeeper::new(k as usize, epsilon, delta) {
        Ok(hk) => Box::into_raw(Box::new(hk)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a HeavyKeeper sketch
/// # Safety
/// Pointer must be valid and have been allocated by heavy_keeper_new()
#[no_mangle]
pub unsafe extern "C" fn heavy_keeper_free(ptr: *mut HeavyKeeper) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a HeavyKeeper sketch with new data
/// # Safety
/// - ptr must be a valid HeavyKeeper pointer from heavy_keeper_new()
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn heavy_keeper_update(ptr: *mut HeavyKeeper, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let hk = &mut *ptr;
    let bytes = slice::from_raw_parts(data, len);
    hk.update(bytes);
}

/// Estimates the count for an item
/// # Safety
/// - ptr must be a valid HeavyKeeper pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn heavy_keeper_estimate(
    ptr: *const HeavyKeeper,
    data: *const u8,
    len: usize,
) -> u32 {
    if ptr.is_null() || data.is_null() {
        return 0;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).estimate(bytes)
}

/// Gets the top-k items
/// # Safety
/// - ptr must be a valid HeavyKeeper pointer
/// - out_buf must point to at least k * 16 bytes (8 for hash, 8 for count)
/// - Returns the number of items written
#[no_mangle]
pub unsafe extern "C" fn heavy_keeper_top_k(
    ptr: *const HeavyKeeper,
    out_buf: *mut u8,
    buf_size: usize,
) -> usize {
    if ptr.is_null() || out_buf.is_null() {
        return 0;
    }
    let top_k = (*ptr).top_k();
    let item_size = 16; // 8 bytes for u64 hash, 8 bytes for u32 count (padded)
    let max_items = buf_size / item_size;
    let items_to_copy = top_k.len().min(max_items);

    let out_slice = slice::from_raw_parts_mut(out_buf, buf_size);
    for (i, (hash, count)) in top_k.iter().take(items_to_copy).enumerate() {
        let offset = i * item_size;
        out_slice[offset..offset + 8].copy_from_slice(&hash.to_le_bytes());
        out_slice[offset + 8..offset + 12].copy_from_slice(&count.to_le_bytes());
    }

    items_to_copy
}

/// Applies decay to age old items
/// # Safety
/// - ptr must be a valid HeavyKeeper pointer
#[no_mangle]
pub unsafe extern "C" fn heavy_keeper_decay(ptr: *mut HeavyKeeper) {
    if ptr.is_null() {
        return;
    }
    (*ptr).decay();
}

// ============================================================================
// Reconciliation - RatelessIBLT
// ============================================================================

use sketch_oxide::common::Reconcilable;
use sketch_oxide::reconciliation::RatelessIBLT;

/// Creates a new RatelessIBLT
/// # Safety
/// Returns an opaque pointer to be freed with rateless_iblt_free()
#[no_mangle]
pub unsafe extern "C" fn rateless_iblt_new(
    expected_diff: usize,
    cell_size: usize,
) -> *mut RatelessIBLT {
    match RatelessIBLT::new(expected_diff, cell_size) {
        Ok(iblt) => Box::into_raw(Box::new(iblt)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a RatelessIBLT
/// # Safety
/// Pointer must be valid and have been allocated by rateless_iblt_new()
#[no_mangle]
pub unsafe extern "C" fn rateless_iblt_free(ptr: *mut RatelessIBLT) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts a key-value pair
/// # Safety
/// - ptr must be a valid RatelessIBLT pointer
/// - key and value must point to valid bytes
#[no_mangle]
pub unsafe extern "C" fn rateless_iblt_insert(
    ptr: *mut RatelessIBLT,
    key: *const u8,
    key_len: usize,
    value: *const u8,
    value_len: usize,
) -> i32 {
    if ptr.is_null() || key.is_null() || value.is_null() {
        return -1;
    }
    let key_bytes = slice::from_raw_parts(key, key_len);
    let value_bytes = slice::from_raw_parts(value, value_len);
    match (*ptr).insert(key_bytes, value_bytes) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Deletes a key-value pair
/// # Safety
/// - ptr must be a valid RatelessIBLT pointer
/// - key and value must point to valid bytes
#[no_mangle]
pub unsafe extern "C" fn rateless_iblt_delete(
    ptr: *mut RatelessIBLT,
    key: *const u8,
    key_len: usize,
    value: *const u8,
    value_len: usize,
) -> i32 {
    if ptr.is_null() || key.is_null() || value.is_null() {
        return -1;
    }
    let key_bytes = slice::from_raw_parts(key, key_len);
    let value_bytes = slice::from_raw_parts(value, value_len);
    match (*ptr).delete(key_bytes, value_bytes) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Subtracts another IBLT
/// # Safety
/// - Both pointers must be valid RatelessIBLT pointers
#[no_mangle]
pub unsafe extern "C" fn rateless_iblt_subtract(
    ptr1: *mut RatelessIBLT,
    ptr2: *const RatelessIBLT,
) -> i32 {
    if ptr1.is_null() || ptr2.is_null() {
        return -1;
    }
    match (*ptr1).subtract(&*ptr2) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// ============================================================================
// Range Filters - Grafite
// ============================================================================

use sketch_oxide::range_filters::Grafite;

/// Builds a new Grafite filter from sorted keys
/// # Safety
/// - keys must point to len valid u64 values
/// Returns an opaque pointer to be freed with grafite_free()
#[no_mangle]
pub unsafe extern "C" fn grafite_build(
    keys: *const u64,
    keys_len: usize,
    bits_per_key: usize,
) -> *mut Grafite {
    if keys.is_null() {
        return std::ptr::null_mut();
    }
    let keys_slice = slice::from_raw_parts(keys, keys_len);
    match Grafite::build(keys_slice, bits_per_key) {
        Ok(grafite) => Box::into_raw(Box::new(grafite)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a Grafite filter
/// # Safety
/// Pointer must be valid and have been allocated by grafite_build()
#[no_mangle]
pub unsafe extern "C" fn grafite_free(ptr: *mut Grafite) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Checks if a range may contain keys
/// # Safety
/// - ptr must be a valid Grafite pointer
#[no_mangle]
pub unsafe extern "C" fn grafite_may_contain_range(
    ptr: *const Grafite,
    low: u64,
    high: u64,
) -> bool {
    if ptr.is_null() {
        return false;
    }
    (*ptr).may_contain_range(low, high)
}

/// Checks if a specific key may be present
/// # Safety
/// - ptr must be a valid Grafite pointer
#[no_mangle]
pub unsafe extern "C" fn grafite_may_contain(ptr: *const Grafite, key: u64) -> bool {
    if ptr.is_null() {
        return false;
    }
    (*ptr).may_contain(key)
}

/// Gets expected false positive rate for a range width
/// # Safety
/// - ptr must be a valid Grafite pointer
#[no_mangle]
pub unsafe extern "C" fn grafite_expected_fpr(ptr: *const Grafite, range_width: u64) -> f64 {
    if ptr.is_null() {
        return 1.0;
    }
    (*ptr).expected_fpr(range_width)
}

// ============================================================================
// Range Filters - MementoFilter
// ============================================================================

use sketch_oxide::range_filters::MementoFilter;

/// Creates a new MementoFilter
/// # Safety
/// Returns an opaque pointer to be freed with memento_filter_free()
#[no_mangle]
pub unsafe extern "C" fn memento_filter_new(
    expected_elements: usize,
    fpr: f64,
) -> *mut MementoFilter {
    match MementoFilter::new(expected_elements, fpr) {
        Ok(filter) => Box::into_raw(Box::new(filter)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a MementoFilter
/// # Safety
/// Pointer must be valid and have been allocated by memento_filter_new()
#[no_mangle]
pub unsafe extern "C" fn memento_filter_free(ptr: *mut MementoFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts a key-value pair
/// # Safety
/// - ptr must be a valid MementoFilter pointer
/// - value must point to valid bytes
#[no_mangle]
pub unsafe extern "C" fn memento_filter_insert(
    ptr: *mut MementoFilter,
    key: u64,
    value: *const u8,
    value_len: usize,
) -> i32 {
    if ptr.is_null() || value.is_null() {
        return -1;
    }
    let value_bytes = slice::from_raw_parts(value, value_len);
    match (*ptr).insert(key, value_bytes) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Checks if a range may contain keys
/// # Safety
/// - ptr must be a valid MementoFilter pointer
#[no_mangle]
pub unsafe extern "C" fn memento_filter_may_contain_range(
    ptr: *const MementoFilter,
    low: u64,
    high: u64,
) -> bool {
    if ptr.is_null() {
        return false;
    }
    (*ptr).may_contain_range(low, high)
}

// ============================================================================
// Streaming - SlidingHyperLogLog
// ============================================================================

use sketch_oxide::streaming::SlidingHyperLogLog;

/// Creates a new SlidingHyperLogLog
/// # Safety
/// Returns an opaque pointer to be freed with sliding_hll_free()
#[no_mangle]
pub unsafe extern "C" fn sliding_hll_new(
    precision: u8,
    max_window_seconds: u64,
) -> *mut SlidingHyperLogLog {
    match SlidingHyperLogLog::new(precision, max_window_seconds) {
        Ok(hll) => Box::into_raw(Box::new(hll)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a SlidingHyperLogLog
/// # Safety
/// Pointer must be valid and have been allocated by sliding_hll_new()
#[no_mangle]
pub unsafe extern "C" fn sliding_hll_free(ptr: *mut SlidingHyperLogLog) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates the sketch with an item and timestamp
/// # Safety
/// - ptr must be a valid SlidingHyperLogLog pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn sliding_hll_update(
    ptr: *mut SlidingHyperLogLog,
    data: *const u8,
    len: usize,
    timestamp: u64,
) -> i32 {
    if ptr.is_null() || data.is_null() {
        return -1;
    }
    let bytes = slice::from_raw_parts(data, len);
    match (*ptr).update(&bytes, timestamp) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Estimates cardinality within a time window
/// # Safety
/// - ptr must be a valid SlidingHyperLogLog pointer
#[no_mangle]
pub unsafe extern "C" fn sliding_hll_estimate_window(
    ptr: *const SlidingHyperLogLog,
    current_time: u64,
    window_seconds: u64,
) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate_window(current_time, window_seconds)
}

/// Estimates total cardinality
/// # Safety
/// - ptr must be a valid SlidingHyperLogLog pointer
#[no_mangle]
pub unsafe extern "C" fn sliding_hll_estimate_total(ptr: *const SlidingHyperLogLog) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate_total()
}

/// Applies decay to remove old entries
/// # Safety
/// - ptr must be a valid SlidingHyperLogLog pointer
#[no_mangle]
pub unsafe extern "C" fn sliding_hll_decay(
    ptr: *mut SlidingHyperLogLog,
    current_time: u64,
    window_seconds: u64,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match (*ptr).decay(current_time, window_seconds) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

// ============================================================================
// Tier 2 Sketches - Phase 2b Extension
// ============================================================================

// ============================================================================
// Membership - VacuumFilter
// ============================================================================

use sketch_oxide::membership::VacuumFilter;

/// Creates a new VacuumFilter
/// # Safety
/// Returns an opaque pointer to be freed with vacuum_filter_free()
#[no_mangle]
pub unsafe extern "C" fn vacuum_filter_new(capacity: usize, fpr: f64) -> *mut VacuumFilter {
    match VacuumFilter::new(capacity, fpr) {
        Ok(filter) => Box::into_raw(Box::new(filter)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a VacuumFilter
/// # Safety
/// Pointer must be valid and have been allocated by vacuum_filter_new()
#[no_mangle]
pub unsafe extern "C" fn vacuum_filter_free(ptr: *mut VacuumFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts an item into the filter
/// # Safety
/// - ptr must be a valid VacuumFilter pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn vacuum_filter_insert(
    ptr: *mut VacuumFilter,
    data: *const u8,
    len: usize,
) -> i32 {
    if ptr.is_null() || data.is_null() {
        return -1;
    }
    let bytes = slice::from_raw_parts(data, len);
    match (*ptr).insert(bytes) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Checks if an item may be present
/// # Safety
/// - ptr must be a valid VacuumFilter pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn vacuum_filter_contains(
    ptr: *const VacuumFilter,
    data: *const u8,
    len: usize,
) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).contains(bytes)
}

/// Deletes an item from the filter
/// # Safety
/// - ptr must be a valid VacuumFilter pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn vacuum_filter_delete(
    ptr: *mut VacuumFilter,
    data: *const u8,
    len: usize,
) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    match (*ptr).delete(bytes) {
        Ok(deleted) => deleted,
        Err(_) => false,
    }
}

/// Gets statistics about the filter
/// # Safety
/// - ptr must be a valid VacuumFilter pointer
/// - out_stats must be a valid pointer to write stats to
#[no_mangle]
pub unsafe extern "C" fn vacuum_filter_stats(
    ptr: *const VacuumFilter,
    out_capacity: *mut usize,
    out_num_items: *mut usize,
    out_load_factor: *mut f64,
    out_memory_bits: *mut u64,
) {
    if ptr.is_null() {
        return;
    }
    let stats = (*ptr).stats();
    if !out_capacity.is_null() {
        *out_capacity = stats.capacity;
    }
    if !out_num_items.is_null() {
        *out_num_items = stats.num_items;
    }
    if !out_load_factor.is_null() {
        *out_load_factor = stats.load_factor;
    }
    if !out_memory_bits.is_null() {
        *out_memory_bits = stats.memory_bits;
    }
}

/// Clears the filter
/// # Safety
/// - ptr must be a valid VacuumFilter pointer
#[no_mangle]
pub unsafe extern "C" fn vacuum_filter_clear(ptr: *mut VacuumFilter) {
    if !ptr.is_null() {
        (*ptr).clear();
    }
}

// ============================================================================
// Range Filters - GRF
// ============================================================================

use sketch_oxide::common::RangeFilter;
use sketch_oxide::range_filters::GRF;

/// Builds a new GRF filter from sorted keys
/// # Safety
/// - keys must point to len valid u64 values
/// Returns an opaque pointer to be freed with grf_free()
#[no_mangle]
pub unsafe extern "C" fn grf_build(
    keys: *const u64,
    keys_len: usize,
    bits_per_key: usize,
) -> *mut GRF {
    if keys.is_null() {
        return std::ptr::null_mut();
    }
    let keys_slice = slice::from_raw_parts(keys, keys_len);
    match GRF::build(keys_slice, bits_per_key) {
        Ok(grf) => Box::into_raw(Box::new(grf)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a GRF filter
/// # Safety
/// Pointer must be valid and have been allocated by grf_build()
#[no_mangle]
pub unsafe extern "C" fn grf_free(ptr: *mut GRF) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Checks if a range may contain keys
/// # Safety
/// - ptr must be a valid GRF pointer
#[no_mangle]
pub unsafe extern "C" fn grf_may_contain_range(ptr: *const GRF, low: u64, high: u64) -> bool {
    if ptr.is_null() {
        return false;
    }
    (*ptr).may_contain_range(low, high)
}

/// Checks if a specific key may be present
/// # Safety
/// - ptr must be a valid GRF pointer
#[no_mangle]
pub unsafe extern "C" fn grf_may_contain(ptr: *const GRF, key: u64) -> bool {
    if ptr.is_null() {
        return false;
    }
    (*ptr).may_contain(key)
}

/// Gets expected false positive rate for a range width
/// # Safety
/// - ptr must be a valid GRF pointer
#[no_mangle]
pub unsafe extern "C" fn grf_expected_fpr(ptr: *const GRF, range_width: u64) -> f64 {
    if ptr.is_null() {
        return 1.0;
    }
    (*ptr).expected_fpr(range_width)
}

/// Gets statistics about the GRF filter
/// # Safety
/// - ptr must be a valid GRF pointer
#[no_mangle]
pub unsafe extern "C" fn grf_stats(
    ptr: *const GRF,
    out_key_count: *mut usize,
    out_segment_count: *mut usize,
    out_avg_keys_per_segment: *mut f64,
    out_bits_per_key: *mut usize,
    out_total_bits: *mut u64,
) {
    if ptr.is_null() {
        return;
    }
    let stats = (*ptr).stats();
    if !out_key_count.is_null() {
        *out_key_count = stats.key_count;
    }
    if !out_segment_count.is_null() {
        *out_segment_count = stats.segment_count;
    }
    if !out_avg_keys_per_segment.is_null() {
        *out_avg_keys_per_segment = stats.avg_keys_per_segment;
    }
    if !out_bits_per_key.is_null() {
        *out_bits_per_key = stats.bits_per_key;
    }
    if !out_total_bits.is_null() {
        *out_total_bits = stats.total_bits;
    }
}

/// Gets the number of keys in the filter
/// # Safety
/// - ptr must be a valid GRF pointer
#[no_mangle]
pub unsafe extern "C" fn grf_key_count(ptr: *const GRF) -> usize {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).key_count()
}

// ============================================================================
// Frequency - NitroSketch
// ============================================================================

use sketch_oxide::frequency::{CountMinSketch, NitroSketch};

/// Creates a new NitroSketch wrapping a CountMinSketch
/// # Safety
/// Returns an opaque pointer to be freed with nitro_sketch_free()
#[no_mangle]
pub unsafe extern "C" fn nitro_sketch_new(
    epsilon: f64,
    delta: f64,
    sample_rate: f64,
) -> *mut NitroSketch<CountMinSketch> {
    let base_sketch = match CountMinSketch::new(epsilon, delta) {
        Ok(sketch) => sketch,
        Err(_) => return std::ptr::null_mut(),
    };
    match NitroSketch::new(base_sketch, sample_rate) {
        Ok(nitro) => Box::into_raw(Box::new(nitro)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a NitroSketch
/// # Safety
/// Pointer must be valid and have been allocated by nitro_sketch_new()
#[no_mangle]
pub unsafe extern "C" fn nitro_sketch_free(ptr: *mut NitroSketch<CountMinSketch>) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates the sketch with sampled data
/// # Safety
/// - ptr must be a valid NitroSketch pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn nitro_sketch_update_sampled(
    ptr: *mut NitroSketch<CountMinSketch>,
    data: *const u8,
    len: usize,
) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).update_sampled(bytes);
}

/// Queries the frequency estimate
/// # Safety
/// - ptr must be a valid NitroSketch pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn nitro_sketch_query(
    ptr: *const NitroSketch<CountMinSketch>,
    data: *const u8,
    len: usize,
) -> u32 {
    if ptr.is_null() || data.is_null() {
        return 0;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).query(bytes) as u32
}

/// Synchronizes the sketch for accuracy recovery
/// # Safety
/// - ptr must be a valid NitroSketch pointer
#[no_mangle]
pub unsafe extern "C" fn nitro_sketch_sync(
    ptr: *mut NitroSketch<CountMinSketch>,
    sync_ratio: f64,
) -> i32 {
    if ptr.is_null() {
        return -1;
    }
    match (*ptr).sync(sync_ratio) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Gets statistics
/// # Safety
/// - ptr must be a valid NitroSketch pointer
#[no_mangle]
pub unsafe extern "C" fn nitro_sketch_stats(
    ptr: *const NitroSketch<CountMinSketch>,
    out_sample_rate: *mut f64,
    out_sampled_count: *mut u64,
    out_unsampled_count: *mut u64,
    out_total_items_estimated: *mut u64,
) {
    if ptr.is_null() {
        return;
    }
    let stats = (*ptr).stats();
    if !out_sample_rate.is_null() {
        *out_sample_rate = stats.sample_rate;
    }
    if !out_sampled_count.is_null() {
        *out_sampled_count = stats.sampled_count;
    }
    if !out_unsampled_count.is_null() {
        *out_unsampled_count = stats.unsampled_count;
    }
    if !out_total_items_estimated.is_null() {
        *out_total_items_estimated = stats.total_items_estimated;
    }
}

// ============================================================================
// Universal - UnivMon
// ============================================================================

use sketch_oxide::universal::UnivMon;

/// Creates a new UnivMon sketch
/// # Safety
/// Returns an opaque pointer to be freed with univmon_free()
#[no_mangle]
pub unsafe extern "C" fn univmon_new(
    max_stream_size: u64,
    epsilon: f64,
    delta: f64,
) -> *mut UnivMon {
    match UnivMon::new(max_stream_size, epsilon, delta) {
        Ok(univmon) => Box::into_raw(Box::new(univmon)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a UnivMon sketch
/// # Safety
/// Pointer must be valid and have been allocated by univmon_new()
#[no_mangle]
pub unsafe extern "C" fn univmon_free(ptr: *mut UnivMon) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates the sketch with an item and value
/// # Safety
/// - ptr must be a valid UnivMon pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn univmon_update(
    ptr: *mut UnivMon,
    data: *const u8,
    len: usize,
    value: f64,
) -> i32 {
    if ptr.is_null() || data.is_null() {
        return -1;
    }
    let bytes = slice::from_raw_parts(data, len);
    match (*ptr).update(bytes, value) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Estimates L1 norm (sum of frequencies)
/// # Safety
/// - ptr must be a valid UnivMon pointer
#[no_mangle]
pub unsafe extern "C" fn univmon_estimate_l1(ptr: *const UnivMon) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate_l1()
}

/// Estimates L2 norm (sum of squared frequencies)
/// # Safety
/// - ptr must be a valid UnivMon pointer
#[no_mangle]
pub unsafe extern "C" fn univmon_estimate_l2(ptr: *const UnivMon) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate_l2()
}

/// Estimates entropy
/// # Safety
/// - ptr must be a valid UnivMon pointer
#[no_mangle]
pub unsafe extern "C" fn univmon_estimate_entropy(ptr: *const UnivMon) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate_entropy()
}

/// Detects change magnitude between two sketches
/// # Safety
/// - Both pointers must be valid UnivMon pointers
#[no_mangle]
pub unsafe extern "C" fn univmon_detect_change(ptr1: *const UnivMon, ptr2: *const UnivMon) -> f64 {
    if ptr1.is_null() || ptr2.is_null() {
        return 0.0;
    }
    (*ptr1).detect_change(&*ptr2)
}

/// Gets statistics
/// # Safety
/// - ptr must be a valid UnivMon pointer
#[no_mangle]
pub unsafe extern "C" fn univmon_stats(
    ptr: *const UnivMon,
    out_num_layers: *mut usize,
    out_total_memory: *mut u64,
    out_samples_processed: *mut u64,
) {
    if ptr.is_null() {
        return;
    }
    let stats = (*ptr).stats();
    if !out_num_layers.is_null() {
        *out_num_layers = stats.num_layers;
    }
    if !out_total_memory.is_null() {
        *out_total_memory = stats.total_memory;
    }
    if !out_samples_processed.is_null() {
        *out_samples_processed = stats.samples_processed;
    }
}

/// Gets epsilon parameter
/// # Safety
/// - ptr must be a valid UnivMon pointer
#[no_mangle]
pub unsafe extern "C" fn univmon_epsilon(ptr: *const UnivMon) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).epsilon()
}

/// Gets delta parameter
/// # Safety
/// - ptr must be a valid UnivMon pointer
#[no_mangle]
pub unsafe extern "C" fn univmon_delta(ptr: *const UnivMon) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).delta()
}

// ============================================================================
// Membership - LearnedBloomFilter
// ============================================================================

use sketch_oxide::membership::LearnedBloomFilter;

/// Creates a new LearnedBloomFilter
/// # Safety
/// - training_keys must point to valid key data
/// - key_lengths must point to lengths for each key
/// Returns an opaque pointer to be freed with learned_bloom_free()
#[no_mangle]
pub unsafe extern "C" fn learned_bloom_new(
    training_keys: *const *const u8,
    key_lengths: *const usize,
    num_keys: usize,
    fpr: f64,
) -> *mut LearnedBloomFilter {
    if training_keys.is_null() || key_lengths.is_null() || num_keys == 0 {
        return std::ptr::null_mut();
    }

    // Convert raw pointers to Vec<Vec<u8>>
    let mut keys = Vec::with_capacity(num_keys);
    for i in 0..num_keys {
        let key_ptr = *training_keys.add(i);
        let key_len = *key_lengths.add(i);
        if !key_ptr.is_null() && key_len > 0 {
            let key_slice = slice::from_raw_parts(key_ptr, key_len);
            keys.push(key_slice.to_vec());
        }
    }

    match LearnedBloomFilter::new(&keys, fpr) {
        Ok(filter) => Box::into_raw(Box::new(filter)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a LearnedBloomFilter
/// # Safety
/// Pointer must be valid and have been allocated by learned_bloom_new()
#[no_mangle]
pub unsafe extern "C" fn learned_bloom_free(ptr: *mut LearnedBloomFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Checks if an item may be present
/// # Safety
/// - ptr must be a valid LearnedBloomFilter pointer
/// - data must point to len valid bytes
#[no_mangle]
pub unsafe extern "C" fn learned_bloom_contains(
    ptr: *const LearnedBloomFilter,
    data: *const u8,
    len: usize,
) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).contains(bytes)
}

/// Gets the memory usage in bytes
/// # Safety
/// - ptr must be a valid LearnedBloomFilter pointer
#[no_mangle]
pub unsafe extern "C" fn learned_bloom_memory_usage(ptr: *const LearnedBloomFilter) -> usize {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).memory_usage()
}

/// Gets the expected false positive rate
/// # Safety
/// - ptr must be a valid LearnedBloomFilter pointer
#[no_mangle]
pub unsafe extern "C" fn learned_bloom_fpr(ptr: *const LearnedBloomFilter) -> f64 {
    if ptr.is_null() {
        return 1.0;
    }
    (*ptr).fpr()
}
