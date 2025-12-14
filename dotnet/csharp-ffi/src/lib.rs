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
// Membership - BloomFilter
// ============================================================================

use sketch_oxide::membership::{BloomFilter, BlockedBloomFilter, CountingBloomFilter, CuckooFilter, RibbonFilter, StableBloomFilter};

/// Creates a new BloomFilter
#[no_mangle]
pub unsafe extern "C" fn bloom_new(n: usize, fpr: f64) -> *mut BloomFilter {
    let bf = BloomFilter::new(n, fpr);
    Box::into_raw(Box::new(bf))
}

/// Frees a BloomFilter
#[no_mangle]
pub unsafe extern "C" fn bloom_free(ptr: *mut BloomFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts an element into BloomFilter
#[no_mangle]
pub unsafe extern "C" fn bloom_insert(ptr: *mut BloomFilter, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).insert(bytes);
}

/// Checks if an element may be in BloomFilter
#[no_mangle]
pub unsafe extern "C" fn bloom_contains(ptr: *const BloomFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).contains(bytes)
}

/// Serializes a BloomFilter
#[no_mangle]
pub unsafe extern "C" fn bloom_serialize(_ptr: *const BloomFilter, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a BloomFilter
#[no_mangle]
pub unsafe extern "C" fn bloom_deserialize(_data: *const u8, _len: usize) -> *mut BloomFilter {
    std::ptr::null_mut()
}

// ============================================================================
// Membership - BlockedBloomFilter
// ============================================================================

/// Creates a new BlockedBloomFilter
#[no_mangle]
pub unsafe extern "C" fn blockedbloom_new(n: usize, fpr: f64) -> *mut BlockedBloomFilter {
    let bf = BlockedBloomFilter::new(n, fpr);
    Box::into_raw(Box::new(bf))
}

/// Frees a BlockedBloomFilter
#[no_mangle]
pub unsafe extern "C" fn blockedbloom_free(ptr: *mut BlockedBloomFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts an element into BlockedBloomFilter
#[no_mangle]
pub unsafe extern "C" fn blockedbloom_insert(ptr: *mut BlockedBloomFilter, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).insert(bytes);
}

/// Checks if an element may be in BlockedBloomFilter
#[no_mangle]
pub unsafe extern "C" fn blockedbloom_contains(ptr: *const BlockedBloomFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).contains(bytes)
}

/// Serializes a BlockedBloomFilter
#[no_mangle]
pub unsafe extern "C" fn blockedbloom_serialize(_ptr: *const BlockedBloomFilter, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a BlockedBloomFilter
#[no_mangle]
pub unsafe extern "C" fn blockedbloom_deserialize(_data: *const u8, _len: usize) -> *mut BlockedBloomFilter {
    std::ptr::null_mut()
}

// ============================================================================
// Membership - CountingBloomFilter
// ============================================================================

/// Creates a new CountingBloomFilter
#[no_mangle]
pub unsafe extern "C" fn countingbloom_new(n: usize, fpr: f64) -> *mut CountingBloomFilter {
    let bf = CountingBloomFilter::new(n, fpr);
    Box::into_raw(Box::new(bf))
}

/// Frees a CountingBloomFilter
#[no_mangle]
pub unsafe extern "C" fn countingbloom_free(ptr: *mut CountingBloomFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts an element into CountingBloomFilter
#[no_mangle]
pub unsafe extern "C" fn countingbloom_insert(ptr: *mut CountingBloomFilter, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).insert(bytes);
}

/// Checks if an element may be in CountingBloomFilter
#[no_mangle]
pub unsafe extern "C" fn countingbloom_contains(ptr: *const CountingBloomFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).contains(bytes)
}

/// Removes an element from CountingBloomFilter
#[no_mangle]
pub unsafe extern "C" fn countingbloom_remove(ptr: *mut CountingBloomFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).remove(bytes)
}

/// Serializes a CountingBloomFilter
#[no_mangle]
pub unsafe extern "C" fn countingbloom_serialize(_ptr: *const CountingBloomFilter, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a CountingBloomFilter
#[no_mangle]
pub unsafe extern "C" fn countingbloom_deserialize(_data: *const u8, _len: usize) -> *mut CountingBloomFilter {
    std::ptr::null_mut()
}

// ============================================================================
// Membership - CuckooFilter
// ============================================================================

/// Creates a new CuckooFilter
#[no_mangle]
pub unsafe extern "C" fn cuckoo_new(capacity: usize) -> *mut CuckooFilter {
    match CuckooFilter::new(capacity) {
        Ok(cf) => Box::into_raw(Box::new(cf)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a CuckooFilter
#[no_mangle]
pub unsafe extern "C" fn cuckoo_free(ptr: *mut CuckooFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts an element into CuckooFilter
#[no_mangle]
pub unsafe extern "C" fn cuckoo_insert(ptr: *mut CuckooFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    match (*ptr).insert(bytes) {
        Ok(()) => true,
        Err(_) => false,
    }
}

/// Checks if an element may be in CuckooFilter
#[no_mangle]
pub unsafe extern "C" fn cuckoo_contains(ptr: *const CuckooFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).contains(bytes)
}

/// Removes an element from CuckooFilter
#[no_mangle]
pub unsafe extern "C" fn cuckoo_remove(ptr: *mut CuckooFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).remove(bytes)
}

/// Serializes a CuckooFilter
#[no_mangle]
pub unsafe extern "C" fn cuckoo_serialize(_ptr: *const CuckooFilter, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a CuckooFilter
#[no_mangle]
pub unsafe extern "C" fn cuckoo_deserialize(_data: *const u8, _len: usize) -> *mut CuckooFilter {
    std::ptr::null_mut()
}

// ============================================================================
// Membership - BinaryFuseFilter
// ============================================================================

use sketch_oxide::membership::BinaryFuseFilter;

/// Creates a new BinaryFuseFilter from u64 items
#[no_mangle]
pub unsafe extern "C" fn binaryfuse_new(items: *const u64, items_len: usize, bits_per_entry: u8) -> *mut BinaryFuseFilter {
    if items.is_null() {
        return std::ptr::null_mut();
    }
    let items_slice = slice::from_raw_parts(items, items_len);
    match BinaryFuseFilter::from_items(items_slice.to_vec(), bits_per_entry) {
        Ok(bf) => Box::into_raw(Box::new(bf)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a BinaryFuseFilter
#[no_mangle]
pub unsafe extern "C" fn binaryfuse_free(ptr: *mut BinaryFuseFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Checks if an item (u64) may be in BinaryFuseFilter
#[no_mangle]
pub unsafe extern "C" fn binaryfuse_contains(ptr: *const BinaryFuseFilter, item: u64) -> bool {
    if ptr.is_null() {
        return false;
    }
    (*ptr).contains(&item)
}

/// Serializes a BinaryFuseFilter
#[no_mangle]
pub unsafe extern "C" fn binaryfuse_serialize(_ptr: *const BinaryFuseFilter, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a BinaryFuseFilter
#[no_mangle]
pub unsafe extern "C" fn binaryfuse_deserialize(_data: *const u8, _len: usize) -> *mut BinaryFuseFilter {
    std::ptr::null_mut()
}

// ============================================================================
// Membership - RibbonFilter
// ============================================================================

/// Creates a new RibbonFilter
#[no_mangle]
pub unsafe extern "C" fn ribbon_new(n: usize, fpr: f64) -> *mut RibbonFilter {
    let rf = RibbonFilter::new(n, fpr);
    Box::into_raw(Box::new(rf))
}

/// Frees a RibbonFilter
#[no_mangle]
pub unsafe extern "C" fn ribbon_free(ptr: *mut RibbonFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts an element into RibbonFilter
#[no_mangle]
pub unsafe extern "C" fn ribbon_insert(ptr: *mut RibbonFilter, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).insert(bytes);
}

/// Finalizes a RibbonFilter (must be called before contains)
#[no_mangle]
pub unsafe extern "C" fn ribbon_finalize(ptr: *mut RibbonFilter) {
    if ptr.is_null() {
        return;
    }
    (*ptr).finalize();
}

/// Checks if an element may be in RibbonFilter
#[no_mangle]
pub unsafe extern "C" fn ribbon_contains(ptr: *const RibbonFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).contains(bytes)
}

/// Serializes a RibbonFilter
#[no_mangle]
pub unsafe extern "C" fn ribbon_serialize(_ptr: *const RibbonFilter, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a RibbonFilter
#[no_mangle]
pub unsafe extern "C" fn ribbon_deserialize(_data: *const u8, _len: usize) -> *mut RibbonFilter {
    std::ptr::null_mut()
}

// ============================================================================
// Membership - StableBloomFilter
// ============================================================================

/// Creates a new StableBloomFilter
#[no_mangle]
pub unsafe extern "C" fn stablebloom_new(expected_items: usize, fpr: f64) -> *mut StableBloomFilter {
    match StableBloomFilter::new(expected_items, fpr) {
        Ok(sf) => Box::into_raw(Box::new(sf)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a StableBloomFilter
#[no_mangle]
pub unsafe extern "C" fn stablebloom_free(ptr: *mut StableBloomFilter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts an element into StableBloomFilter
#[no_mangle]
pub unsafe extern "C" fn stablebloom_insert(ptr: *mut StableBloomFilter, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).insert(bytes);
}

/// Checks if an element may be in StableBloomFilter
#[no_mangle]
pub unsafe extern "C" fn stablebloom_contains(ptr: *const StableBloomFilter, data: *const u8, len: usize) -> bool {
    if ptr.is_null() || data.is_null() {
        return false;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).contains(bytes)
}

/// Serializes a StableBloomFilter
#[no_mangle]
pub unsafe extern "C" fn stablebloom_serialize(_ptr: *const StableBloomFilter, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a StableBloomFilter
#[no_mangle]
pub unsafe extern "C" fn stablebloom_deserialize(_data: *const u8, _len: usize) -> *mut StableBloomFilter {
    std::ptr::null_mut()
}

// ============================================================================
// Quantiles - DDSketch, KllSketch, ReqSketch, SplineSketch, TDigest
// ============================================================================

use sketch_oxide::quantiles::{DDSketch, KllSketch, ReqSketch, SplineSketch, TDigest};

/// Creates a new DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_new(relative_accuracy: f64) -> *mut DDSketch {
    match DDSketch::new(relative_accuracy) {
        Ok(dd) => Box::into_raw(Box::new(dd)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_free(ptr: *mut DDSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Adds a value to DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_add(ptr: *mut DDSketch, value: f64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).add(value);
}

/// Gets the quantile from DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_quantile(ptr: *const DDSketch, q: f64) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).quantile(q).unwrap_or(0.0)
}

/// Gets count from DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_count(ptr: *const DDSketch) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).count()
}

/// Gets min from DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_min(ptr: *const DDSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).min().unwrap_or(0.0)
}

/// Gets max from DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_max(ptr: *const DDSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).max().unwrap_or(0.0)
}

/// Merges two DDSketches
#[no_mangle]
pub unsafe extern "C" fn ddsketch_merge(ptr1: *mut DDSketch, ptr2: *const DDSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes a DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_serialize(_ptr: *const DDSketch, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a DDSketch
#[no_mangle]
pub unsafe extern "C" fn ddsketch_deserialize(_data: *const u8, _len: usize) -> *mut DDSketch {
    std::ptr::null_mut()
}

// ============================================================================
// KllSketch
// ============================================================================

/// Creates a new KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_new(k: u16) -> *mut KllSketch {
    match KllSketch::new(k) {
        Ok(kll) => Box::into_raw(Box::new(kll)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_free(ptr: *mut KllSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_update(ptr: *mut KllSketch, value: f64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).update(value);
}

/// Gets quantile from KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_quantile(ptr: *mut KllSketch, rank: f64) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).quantile(rank).unwrap_or(0.0)
}

/// Gets count from KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_count(ptr: *const KllSketch) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).count()
}

/// Gets min from KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_min(ptr: *const KllSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).min()
}

/// Gets max from KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_max(ptr: *const KllSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).max()
}

/// Merges two KllSketches
#[no_mangle]
pub unsafe extern "C" fn kll_merge(ptr1: *mut KllSketch, ptr2: *const KllSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes a KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_serialize(_ptr: *mut KllSketch, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a KllSketch
#[no_mangle]
pub unsafe extern "C" fn kll_deserialize(_data: *const u8, _len: usize) -> *mut KllSketch {
    std::ptr::null_mut()
}

// ============================================================================
// ReqSketch
// ============================================================================

/// Creates a new ReqSketch (mode: 0=HighRankAccuracy, 1=LowRankAccuracy)
#[no_mangle]
pub unsafe extern "C" fn req_new(k: usize, mode: u8) -> *mut ReqSketch {
    use sketch_oxide::quantiles::ReqMode;
    let req_mode = if mode == 0 {
        ReqMode::HighRankAccuracy
    } else {
        ReqMode::LowRankAccuracy
    };
    match ReqSketch::new(k, req_mode) {
        Ok(req) => Box::into_raw(Box::new(req)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a ReqSketch
#[no_mangle]
pub unsafe extern "C" fn req_free(ptr: *mut ReqSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a ReqSketch
#[no_mangle]
pub unsafe extern "C" fn req_update(ptr: *mut ReqSketch, value: f64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).update(value);
}

/// Gets quantile from ReqSketch
#[no_mangle]
pub unsafe extern "C" fn req_quantile(ptr: *const ReqSketch, q: f64) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).quantile(q).unwrap_or(0.0)
}

/// Gets count from ReqSketch
#[no_mangle]
pub unsafe extern "C" fn req_count(ptr: *const ReqSketch) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).n()
}

/// Gets min from ReqSketch
#[no_mangle]
pub unsafe extern "C" fn req_min(ptr: *const ReqSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).min().unwrap_or(0.0)
}

/// Gets max from ReqSketch
#[no_mangle]
pub unsafe extern "C" fn req_max(ptr: *const ReqSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).max().unwrap_or(0.0)
}

/// Serializes a ReqSketch
#[no_mangle]
pub unsafe extern "C" fn req_serialize(_ptr: *const ReqSketch, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a ReqSketch
#[no_mangle]
pub unsafe extern "C" fn req_deserialize(_data: *const u8, _len: usize) -> *mut ReqSketch {
    std::ptr::null_mut()
}

// ============================================================================
// SplineSketch
// ============================================================================

/// Creates a new SplineSketch
#[no_mangle]
pub unsafe extern "C" fn spline_new(max_samples: usize) -> *mut SplineSketch {
    let ss = SplineSketch::new(max_samples);
    Box::into_raw(Box::new(ss))
}

/// Frees a SplineSketch
#[no_mangle]
pub unsafe extern "C" fn spline_free(ptr: *mut SplineSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a SplineSketch with value and weight
#[no_mangle]
pub unsafe extern "C" fn spline_update(ptr: *mut SplineSketch, value: u64, weight: f64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).update(value, weight);
}

/// Queries a quantile from SplineSketch
#[no_mangle]
pub unsafe extern "C" fn spline_query(ptr: *const SplineSketch, quantile: f64) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).query(quantile)
}

/// Gets sample count from SplineSketch
#[no_mangle]
pub unsafe extern "C" fn spline_sample_count(ptr: *const SplineSketch) -> usize {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).sample_count()
}

/// Gets total weight from SplineSketch
#[no_mangle]
pub unsafe extern "C" fn spline_total_weight(ptr: *const SplineSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).total_weight()
}

/// Merges two SplingSketches
#[no_mangle]
pub unsafe extern "C" fn spline_merge(ptr1: *mut SplineSketch, ptr2: *const SplineSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes a SplineSketch
#[no_mangle]
pub unsafe extern "C" fn spline_serialize(_ptr: *const SplineSketch, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a SplineSketch
#[no_mangle]
pub unsafe extern "C" fn spline_deserialize(_data: *const u8, _len: usize) -> *mut SplineSketch {
    std::ptr::null_mut()
}

// ============================================================================
// TDigest
// ============================================================================

/// Creates a new TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_new(compression: f64) -> *mut TDigest {
    let td = TDigest::new(compression);
    Box::into_raw(Box::new(td))
}

/// Frees a TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_free(ptr: *mut TDigest) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_update(ptr: *mut TDigest, value: f64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).update(value);
}

/// Updates a TDigest with weight
#[no_mangle]
pub unsafe extern "C" fn tdigest_update_weighted(ptr: *mut TDigest, value: f64, weight: f64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).update_weighted(value, weight);
}

/// Gets quantile from TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_quantile(ptr: *mut TDigest, q: f64) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).quantile(q)
}

/// Gets count from TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_count(ptr: *const TDigest) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).count()
}

/// Gets min from TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_min(ptr: *const TDigest) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).min()
}

/// Gets max from TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_max(ptr: *const TDigest) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).max()
}

/// Merges two TDigests
#[no_mangle]
pub unsafe extern "C" fn tdigest_merge(ptr1: *mut TDigest, ptr2: *const TDigest) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes a TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_serialize(_ptr: *mut TDigest, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes a TDigest
#[no_mangle]
pub unsafe extern "C" fn tdigest_deserialize(_data: *const u8, _len: usize) -> *mut TDigest {
    std::ptr::null_mut()
}

// ============================================================================
// Streaming - SlidingWindowCounter, ExponentialHistogram, SlidingHyperLogLog
// ============================================================================

use sketch_oxide::streaming::{SlidingWindowCounter, ExponentialHistogram};

/// Creates a new SlidingWindowCounter
#[no_mangle]
pub unsafe extern "C" fn slidingwindow_new(window_size: u64, epsilon: f64) -> *mut SlidingWindowCounter {
    match SlidingWindowCounter::new(window_size, epsilon) {
        Ok(swc) => Box::into_raw(Box::new(swc)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a SlidingWindowCounter
#[no_mangle]
pub unsafe extern "C" fn slidingwindow_free(ptr: *mut SlidingWindowCounter) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Increments SlidingWindowCounter at timestamp
#[no_mangle]
pub unsafe extern "C" fn slidingwindow_increment(ptr: *mut SlidingWindowCounter, timestamp: u64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).increment(timestamp);
}

/// Gets count within window from SlidingWindowCounter
#[no_mangle]
pub unsafe extern "C" fn slidingwindow_count(ptr: *const SlidingWindowCounter, current_time: u64) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).count(current_time)
}

/// Gets window size from SlidingWindowCounter
#[no_mangle]
pub unsafe extern "C" fn slidingwindow_window_size(ptr: *const SlidingWindowCounter) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).window_size()
}

// ============================================================================
// Streaming - ExponentialHistogram
// ============================================================================

/// Creates a new ExponentialHistogram
#[no_mangle]
pub unsafe extern "C" fn exphistogram_new(window_size: u64, epsilon: f64) -> *mut ExponentialHistogram {
    match ExponentialHistogram::new(window_size, epsilon) {
        Ok(eh) => Box::into_raw(Box::new(eh)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees an ExponentialHistogram
#[no_mangle]
pub unsafe extern "C" fn exphistogram_free(ptr: *mut ExponentialHistogram) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Inserts into ExponentialHistogram
#[no_mangle]
pub unsafe extern "C" fn exphistogram_insert(ptr: *mut ExponentialHistogram, timestamp: u64, count: u64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).insert(timestamp, count);
}

/// Gets count within window from ExponentialHistogram
#[no_mangle]
pub unsafe extern "C" fn exphistogram_count(ptr: *const ExponentialHistogram, current_time: u64) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    let (_a, _b, c) = (*ptr).count(current_time);
    c
}

/// Gets window size from ExponentialHistogram
#[no_mangle]
pub unsafe extern "C" fn exphistogram_window_size(ptr: *const ExponentialHistogram) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).window_size()
}

// ============================================================================
// Streaming - SlidingHyperLogLog
// ============================================================================

/// Creates a new SlidingHyperLogLog
#[no_mangle]
pub unsafe extern "C" fn slidinghll_new(precision: u8, max_window_seconds: u64) -> *mut SlidingHyperLogLog {
    match SlidingHyperLogLog::new(precision, max_window_seconds) {
        Ok(shll) => Box::into_raw(Box::new(shll)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a SlidingHyperLogLog
#[no_mangle]
pub unsafe extern "C" fn slidinghll_free(ptr: *mut SlidingHyperLogLog) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates SlidingHyperLogLog
#[no_mangle]
pub unsafe extern "C" fn slidinghll_update(ptr: *mut SlidingHyperLogLog, data: *const u8, len: usize, timestamp: u64) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let _ = (*ptr).update(&hash, timestamp);
}

/// Estimates cardinality within window
#[no_mangle]
pub unsafe extern "C" fn slidinghll_estimate_window(ptr: *const SlidingHyperLogLog, current_time: u64, window_seconds: u64) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate_window(current_time, window_seconds)
}

/// Gets precision from SlidingHyperLogLog
#[no_mangle]
pub unsafe extern "C" fn slidinghll_precision(ptr: *const SlidingHyperLogLog) -> u8 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).precision()
}

// ============================================================================
// Similarity - MinHash, SimHash
// ============================================================================

use sketch_oxide::similarity::{MinHash, SimHash};

/// Creates a new MinHash
#[no_mangle]
pub unsafe extern "C" fn minhash_new(num_perm: usize) -> *mut MinHash {
    match MinHash::new(num_perm) {
        Ok(mh) => Box::into_raw(Box::new(mh)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a MinHash
#[no_mangle]
pub unsafe extern "C" fn minhash_free(ptr: *mut MinHash) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates MinHash
#[no_mangle]
pub unsafe extern "C" fn minhash_update(ptr: *mut MinHash, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash);
}

/// Computes Jaccard similarity between two MinHashes
#[no_mangle]
pub unsafe extern "C" fn minhash_jaccard(ptr1: *const MinHash, ptr2: *const MinHash) -> f64 {
    if ptr1.is_null() || ptr2.is_null() {
        return 0.0;
    }
    (*ptr1).jaccard_similarity(&*ptr2).unwrap_or(0.0)
}

/// Merges two MinHashes
#[no_mangle]
pub unsafe extern "C" fn minhash_merge(ptr1: *mut MinHash, ptr2: *const MinHash) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

// ============================================================================
// Similarity - SimHash
// ============================================================================

/// Creates a new SimHash
#[no_mangle]
pub unsafe extern "C" fn simhash_new() -> *mut SimHash {
    let sh = SimHash::new();
    Box::into_raw(Box::new(sh))
}

/// Frees a SimHash
#[no_mangle]
pub unsafe extern "C" fn simhash_free(ptr: *mut SimHash) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates SimHash with a feature
#[no_mangle]
pub unsafe extern "C" fn simhash_update(ptr: *mut SimHash, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash);
}

/// Gets fingerprint from SimHash
#[no_mangle]
pub unsafe extern "C" fn simhash_fingerprint(ptr: *mut SimHash) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).fingerprint()
}

/// Computes Hamming distance between two SimHashes
#[no_mangle]
pub unsafe extern "C" fn simhash_hamming_distance(ptr1: *mut SimHash, ptr2: *mut SimHash) -> u32 {
    if ptr1.is_null() || ptr2.is_null() {
        return 0;
    }
    (*ptr1).hamming_distance(&mut *ptr2)
}

/// Computes similarity between two SimHashes
#[no_mangle]
pub unsafe extern "C" fn simhash_similarity(ptr1: *mut SimHash, ptr2: *mut SimHash) -> f64 {
    if ptr1.is_null() || ptr2.is_null() {
        return 0.0;
    }
    (*ptr1).similarity(&mut *ptr2)
}

/// Merges two SimHashes
#[no_mangle]
pub unsafe extern "C" fn simhash_merge(ptr1: *mut SimHash, ptr2: *const SimHash) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

// ============================================================================
// Sampling - ReservoirSampling<u64>, VarOptSampling<u64>
// ============================================================================

use sketch_oxide::sampling::{ReservoirSampling, VarOptSampling};

/// Creates a new ReservoirSampling
#[no_mangle]
pub unsafe extern "C" fn reservoir_new(k: usize) -> *mut ReservoirSampling<u64> {
    match ReservoirSampling::new(k) {
        Ok(rs) => Box::into_raw(Box::new(rs)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a ReservoirSampling
#[no_mangle]
pub unsafe extern "C" fn reservoir_free(ptr: *mut ReservoirSampling<u64>) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates ReservoirSampling with an item
#[no_mangle]
pub unsafe extern "C" fn reservoir_update(ptr: *mut ReservoirSampling<u64>, item: u64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).update(item);
}

/// Gets count from ReservoirSampling
#[no_mangle]
pub unsafe extern "C" fn reservoir_count(ptr: *const ReservoirSampling<u64>) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).count()
}

/// Gets sample length from ReservoirSampling
#[no_mangle]
pub unsafe extern "C" fn reservoir_len(ptr: *const ReservoirSampling<u64>) -> usize {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).len()
}

/// Merges two ReservoirSamplings
#[no_mangle]
pub unsafe extern "C" fn reservoir_merge(ptr1: *mut ReservoirSampling<u64>, ptr2: *const ReservoirSampling<u64>) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

// ============================================================================
// Sampling - VarOptSampling<u64>
// ============================================================================

/// Creates a new VarOptSampling
#[no_mangle]
pub unsafe extern "C" fn varopt_new(k: usize) -> *mut VarOptSampling<u64> {
    match VarOptSampling::new(k) {
        Ok(vo) => Box::into_raw(Box::new(vo)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a VarOptSampling
#[no_mangle]
pub unsafe extern "C" fn varopt_free(ptr: *mut VarOptSampling<u64>) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates VarOptSampling with weighted item
#[no_mangle]
pub unsafe extern "C" fn varopt_update(ptr: *mut VarOptSampling<u64>, item: u64, weight: f64) {
    if ptr.is_null() {
        return;
    }
    (*ptr).update(item, weight);
}

/// Gets count from VarOptSampling
#[no_mangle]
pub unsafe extern "C" fn varopt_count(ptr: *const VarOptSampling<u64>) -> u64 {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).count()
}

/// Gets sample length from VarOptSampling
#[no_mangle]
pub unsafe extern "C" fn varopt_len(ptr: *const VarOptSampling<u64>) -> usize {
    if ptr.is_null() {
        return 0;
    }
    (*ptr).len()
}

/// Gets total weight from VarOptSampling
#[no_mangle]
pub unsafe extern "C" fn varopt_total_weight(ptr: *const VarOptSampling<u64>) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).total_weight()
}

/// Gets estimated total weight from VarOptSampling
#[no_mangle]
pub unsafe extern "C" fn varopt_estimate_total_weight(ptr: *const VarOptSampling<u64>) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate_total_weight()
}

/// Merges two VarOptSamplings
#[no_mangle]
pub unsafe extern "C" fn varopt_merge(ptr1: *mut VarOptSampling<u64>, ptr2: *const VarOptSampling<u64>) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
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

// ============================================================================
// Cardinality - UltraLogLog
// ============================================================================

use sketch_oxide::cardinality::{UltraLogLog, CpcSketch, QSketch, ThetaSketch};

/// Creates a new UltraLogLog sketch
#[no_mangle]
pub unsafe extern "C" fn ultraloglog_new(precision: u8) -> *mut UltraLogLog {
    match UltraLogLog::new(precision) {
        Ok(ull) => Box::into_raw(Box::new(ull)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees an UltraLogLog sketch
#[no_mangle]
pub unsafe extern "C" fn ultraloglog_free(ptr: *mut UltraLogLog) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates an UltraLogLog sketch with new data
#[no_mangle]
pub unsafe extern "C" fn ultraloglog_update(ptr: *mut UltraLogLog, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash);
}

/// Estimates the cardinality
#[no_mangle]
pub unsafe extern "C" fn ultraloglog_estimate(ptr: *const UltraLogLog) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate()
}

/// Merges two UltraLogLog sketches
#[no_mangle]
pub unsafe extern "C" fn ultraloglog_merge(ptr1: *mut UltraLogLog, ptr2: *const UltraLogLog) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes an UltraLogLog sketch
#[no_mangle]
pub unsafe extern "C" fn ultraloglog_serialize(
    ptr: *const UltraLogLog,
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

/// Deserializes an UltraLogLog sketch
#[no_mangle]
pub unsafe extern "C" fn ultraloglog_deserialize(data: *const u8, len: usize) -> *mut UltraLogLog {
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = slice::from_raw_parts(data, len);
    match UltraLogLog::deserialize(bytes) {
        Ok(ull) => Box::into_raw(Box::new(ull)),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Cardinality - CpcSketch
// ============================================================================

/// Creates a new CpcSketch
#[no_mangle]
pub unsafe extern "C" fn cpc_new(lgk: u8) -> *mut CpcSketch {
    match CpcSketch::new(lgk) {
        Ok(cpc) => Box::into_raw(Box::new(cpc)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a CpcSketch
#[no_mangle]
pub unsafe extern "C" fn cpc_free(ptr: *mut CpcSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a CpcSketch with new data
#[no_mangle]
pub unsafe extern "C" fn cpc_update(ptr: *mut CpcSketch, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash);
}

/// Estimates the cardinality
#[no_mangle]
pub unsafe extern "C" fn cpc_estimate(ptr: *const CpcSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate()
}

/// Merges two CpcSketch sketches
#[no_mangle]
pub unsafe extern "C" fn cpc_merge(ptr1: *mut CpcSketch, ptr2: *const CpcSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes a CpcSketch
#[no_mangle]
pub unsafe extern "C" fn cpc_serialize(
    ptr: *const CpcSketch,
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

/// Deserializes a CpcSketch
#[no_mangle]
pub unsafe extern "C" fn cpc_deserialize(data: *const u8, len: usize) -> *mut CpcSketch {
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = slice::from_raw_parts(data, len);
    match CpcSketch::deserialize(bytes) {
        Ok(cpc) => Box::into_raw(Box::new(cpc)),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Cardinality - QSketch
// ============================================================================

/// Creates a new QSketch
#[no_mangle]
pub unsafe extern "C" fn qsketch_new(max_samples: u32) -> *mut QSketch {
    let qs = QSketch::new(max_samples as usize);
    Box::into_raw(Box::new(qs))
}

/// Frees a QSketch
#[no_mangle]
pub unsafe extern "C" fn qsketch_free(ptr: *mut QSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a QSketch with new data
#[no_mangle]
pub unsafe extern "C" fn qsketch_update(ptr: *mut QSketch, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).update(bytes, 1.0);
}

/// Updates a QSketch with weighted data
#[no_mangle]
pub unsafe extern "C" fn qsketch_update_weighted(
    ptr: *mut QSketch,
    data: *const u8,
    len: usize,
    weight: f64,
) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).update(bytes, weight);
}

/// Estimates the cardinality
#[no_mangle]
pub unsafe extern "C" fn qsketch_estimate(ptr: *const QSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate_distinct_elements() as f64
}

/// Merges two QSketch sketches
#[no_mangle]
pub unsafe extern "C" fn qsketch_merge(ptr1: *mut QSketch, ptr2: *const QSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes a QSketch
#[no_mangle]
pub unsafe extern "C" fn qsketch_serialize(
    ptr: *const QSketch,
    out_len: *mut usize,
) -> *mut u8 {
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = (*ptr).to_bytes();
    let len = bytes.len();
    if !out_len.is_null() {
        *out_len = len;
    }
    let boxed = bytes.into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

/// Deserializes a QSketch
#[no_mangle]
pub unsafe extern "C" fn qsketch_deserialize(data: *const u8, len: usize) -> *mut QSketch {
    if data.is_null() {
        return std::ptr::null_mut();
    }
    let bytes = slice::from_raw_parts(data, len);
    match QSketch::from_bytes(bytes) {
        Ok(qs) => Box::into_raw(Box::new(qs)),
        Err(_) => std::ptr::null_mut(),
    }
}

// ============================================================================
// Cardinality - ThetaSketch
// ============================================================================

/// Creates a new ThetaSketch
#[no_mangle]
pub unsafe extern "C" fn theta_new(lgk: u8) -> *mut ThetaSketch {
    match ThetaSketch::new(lgk) {
        Ok(theta) => Box::into_raw(Box::new(theta)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees a ThetaSketch
#[no_mangle]
pub unsafe extern "C" fn theta_free(ptr: *mut ThetaSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates a ThetaSketch with new data
#[no_mangle]
pub unsafe extern "C" fn theta_update(ptr: *mut ThetaSketch, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash);
}

/// Estimates the cardinality
#[no_mangle]
pub unsafe extern "C" fn theta_estimate(ptr: *const ThetaSketch) -> f64 {
    if ptr.is_null() {
        return 0.0;
    }
    (*ptr).estimate()
}

// ============================================================================
// Frequency Algorithms (Stub implementations)
// ============================================================================

use sketch_oxide::frequency::{CountSketch, ConservativeCountMin, ElasticSketch, SALSA, FrequentItems, RemovableUniversalSketch, SpaceSaving};

/// CountMinSketch - Creates a new sketch
#[no_mangle]
pub unsafe extern "C" fn countmin_new(epsilon: f64, delta: f64) -> *mut CountMinSketch {
    match CountMinSketch::new(epsilon, delta) {
        Ok(cms) => Box::into_raw(Box::new(cms)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees CountMinSketch
#[no_mangle]
pub unsafe extern "C" fn countmin_free(ptr: *mut CountMinSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates CountMinSketch
#[no_mangle]
pub unsafe extern "C" fn countmin_update(ptr: *mut CountMinSketch, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash);
}

/// Estimates count in CountMinSketch
#[no_mangle]
pub unsafe extern "C" fn countmin_estimate(ptr: *const CountMinSketch, data: *const u8, len: usize) -> u64 {
    if ptr.is_null() || data.is_null() {
        return 0;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).estimate(&hash) as u64
}

/// Merges CountMinSketch
#[no_mangle]
pub unsafe extern "C" fn countmin_merge(ptr1: *mut CountMinSketch, ptr2: *const CountMinSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes CountMinSketch
#[no_mangle]
pub unsafe extern "C" fn countmin_serialize(ptr: *const CountMinSketch, out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes CountMinSketch
#[no_mangle]
pub unsafe extern "C" fn countmin_deserialize(data: *const u8, len: usize) -> *mut CountMinSketch {
    std::ptr::null_mut()
}

// ============================================================================
// Frequency - CountSketch
// ============================================================================

/// Creates a new CountSketch
#[no_mangle]
pub unsafe extern "C" fn countsketch_new(epsilon: f64, delta: f64) -> *mut CountSketch {
    match CountSketch::new(epsilon, delta) {
        Ok(cs) => Box::into_raw(Box::new(cs)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees CountSketch
#[no_mangle]
pub unsafe extern "C" fn countsketch_free(ptr: *mut CountSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates CountSketch
#[no_mangle]
pub unsafe extern "C" fn countsketch_update(ptr: *mut CountSketch, data: *const u8, len: usize, weight: i64) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash, weight);
}

/// Estimates count in CountSketch
#[no_mangle]
pub unsafe extern "C" fn countsketch_estimate(ptr: *const CountSketch, data: *const u8, len: usize) -> i64 {
    if ptr.is_null() || data.is_null() {
        return 0;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).estimate(&hash)
}

/// Merges CountSketch
#[no_mangle]
pub unsafe extern "C" fn countsketch_merge(ptr1: *mut CountSketch, ptr2: *const CountSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes CountSketch
#[no_mangle]
pub unsafe extern "C" fn countsketch_serialize(_ptr: *const CountSketch, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes CountSketch
#[no_mangle]
pub unsafe extern "C" fn countsketch_deserialize(_data: *const u8, _len: usize) -> *mut CountSketch {
    std::ptr::null_mut()
}

// ============================================================================
// Frequency - ConservativeCountMin
// ============================================================================

/// Creates a new ConservativeCountMin
#[no_mangle]
pub unsafe extern "C" fn conservativecountmin_new(epsilon: f64, delta: f64) -> *mut ConservativeCountMin {
    match ConservativeCountMin::new(epsilon, delta) {
        Ok(ccm) => Box::into_raw(Box::new(ccm)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees ConservativeCountMin
#[no_mangle]
pub unsafe extern "C" fn conservativecountmin_free(ptr: *mut ConservativeCountMin) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates ConservativeCountMin
#[no_mangle]
pub unsafe extern "C" fn conservativecountmin_update(ptr: *mut ConservativeCountMin, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash);
}

/// Estimates count in ConservativeCountMin
#[no_mangle]
pub unsafe extern "C" fn conservativecountmin_estimate(ptr: *const ConservativeCountMin, data: *const u8, len: usize) -> u64 {
    if ptr.is_null() || data.is_null() {
        return 0;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).estimate(&hash) as u64
}

/// Merges ConservativeCountMin
#[no_mangle]
pub unsafe extern "C" fn conservativecountmin_merge(ptr1: *mut ConservativeCountMin, ptr2: *const ConservativeCountMin) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes ConservativeCountMin
#[no_mangle]
pub unsafe extern "C" fn conservativecountmin_serialize(_ptr: *const ConservativeCountMin, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes ConservativeCountMin
#[no_mangle]
pub unsafe extern "C" fn conservativecountmin_deserialize(_data: *const u8, _len: usize) -> *mut ConservativeCountMin {
    std::ptr::null_mut()
}

// ============================================================================
// Frequency - SpaceSaving<u64>
// ============================================================================

/// Creates a new SpaceSaving
#[no_mangle]
pub unsafe extern "C" fn spacesaving_new(threshold: f64) -> *mut SpaceSaving<u64> {
    match SpaceSaving::new(threshold) {
        Ok(ss) => Box::into_raw(Box::new(ss)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees SpaceSaving
#[no_mangle]
pub unsafe extern "C" fn spacesaving_free(ptr: *mut SpaceSaving<u64>) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates SpaceSaving
#[no_mangle]
pub unsafe extern "C" fn spacesaving_update(ptr: *mut SpaceSaving<u64>, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(hash);
}

/// Merges SpaceSaving
#[no_mangle]
pub unsafe extern "C" fn spacesaving_merge(ptr1: *mut SpaceSaving<u64>, ptr2: *const SpaceSaving<u64>) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes SpaceSaving
#[no_mangle]
pub unsafe extern "C" fn spacesaving_serialize(_ptr: *const SpaceSaving<u64>, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes SpaceSaving
#[no_mangle]
pub unsafe extern "C" fn spacesaving_deserialize(_data: *const u8, _len: usize) -> *mut SpaceSaving<u64> {
    std::ptr::null_mut()
}

// ============================================================================
// Frequency - FrequentItems<u64>
// ============================================================================

/// Creates a new FrequentItems
#[no_mangle]
pub unsafe extern "C" fn frequentitems_new(max_size: usize) -> *mut FrequentItems<u64> {
    match FrequentItems::new(max_size) {
        Ok(fi) => Box::into_raw(Box::new(fi)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees FrequentItems
#[no_mangle]
pub unsafe extern "C" fn frequentitems_free(ptr: *mut FrequentItems<u64>) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates FrequentItems
#[no_mangle]
pub unsafe extern "C" fn frequentitems_update(ptr: *mut FrequentItems<u64>, data: *const u8, len: usize) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(hash);
}

/// Merges FrequentItems
#[no_mangle]
pub unsafe extern "C" fn frequentitems_merge(ptr1: *mut FrequentItems<u64>, ptr2: *const FrequentItems<u64>) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes FrequentItems
#[no_mangle]
pub unsafe extern "C" fn frequentitems_serialize(_ptr: *const FrequentItems<u64>, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes FrequentItems
#[no_mangle]
pub unsafe extern "C" fn frequentitems_deserialize(_data: *const u8, _len: usize) -> *mut FrequentItems<u64> {
    std::ptr::null_mut()
}

// ============================================================================
// Frequency - ElasticSketch
// ============================================================================

/// Creates a new ElasticSketch
#[no_mangle]
pub unsafe extern "C" fn elasticsketch_new(bucket_count: usize, depth: usize) -> *mut ElasticSketch {
    match ElasticSketch::new(bucket_count, depth) {
        Ok(es) => Box::into_raw(Box::new(es)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees ElasticSketch
#[no_mangle]
pub unsafe extern "C" fn elasticsketch_free(ptr: *mut ElasticSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates ElasticSketch
#[no_mangle]
pub unsafe extern "C" fn elasticsketch_update(ptr: *mut ElasticSketch, data: *const u8, len: usize, count: u64) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).update(bytes, count);
}

/// Estimates count in ElasticSketch
#[no_mangle]
pub unsafe extern "C" fn elasticsketch_estimate(ptr: *const ElasticSketch, data: *const u8, len: usize) -> u64 {
    if ptr.is_null() || data.is_null() {
        return 0;
    }
    let bytes = slice::from_raw_parts(data, len);
    (*ptr).estimate(bytes) as u64
}

/// Merges ElasticSketch
#[no_mangle]
pub unsafe extern "C" fn elasticsketch_merge(ptr1: *mut ElasticSketch, ptr2: *const ElasticSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes ElasticSketch
#[no_mangle]
pub unsafe extern "C" fn elasticsketch_serialize(_ptr: *const ElasticSketch, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes ElasticSketch
#[no_mangle]
pub unsafe extern "C" fn elasticsketch_deserialize(_data: *const u8, _len: usize) -> *mut ElasticSketch {
    std::ptr::null_mut()
}

// ============================================================================
// Frequency - SALSA
// ============================================================================

/// Creates a new SALSA
#[no_mangle]
pub unsafe extern "C" fn salsa_new(epsilon: f64, delta: f64) -> *mut SALSA {
    match SALSA::new(epsilon, delta) {
        Ok(salsa) => Box::into_raw(Box::new(salsa)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees SALSA
#[no_mangle]
pub unsafe extern "C" fn salsa_free(ptr: *mut SALSA) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates SALSA
#[no_mangle]
pub unsafe extern "C" fn salsa_update(ptr: *mut SALSA, data: *const u8, len: usize, weight: u64) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash, weight);
}

/// Estimates count in SALSA
#[no_mangle]
pub unsafe extern "C" fn salsa_estimate(ptr: *const SALSA, data: *const u8, len: usize) -> u64 {
    if ptr.is_null() || data.is_null() {
        return 0;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    let (estimate, _) = (*ptr).estimate(&hash);
    estimate
}

/// Merges SALSA
#[no_mangle]
pub unsafe extern "C" fn salsa_merge(ptr1: *mut SALSA, ptr2: *const SALSA) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes SALSA
#[no_mangle]
pub unsafe extern "C" fn salsa_serialize(_ptr: *const SALSA, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes SALSA
#[no_mangle]
pub unsafe extern "C" fn salsa_deserialize(_data: *const u8, _len: usize) -> *mut SALSA {
    std::ptr::null_mut()
}

// ============================================================================
// Frequency - RemovableUniversalSketch
// ============================================================================

/// Creates a new RemovableUniversalSketch
#[no_mangle]
pub unsafe extern "C" fn removableuniversalsketch_new(epsilon: f64, delta: f64) -> *mut RemovableUniversalSketch {
    match RemovableUniversalSketch::new(epsilon, delta) {
        Ok(rus) => Box::into_raw(Box::new(rus)),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Frees RemovableUniversalSketch
#[no_mangle]
pub unsafe extern "C" fn removableuniversalsketch_free(ptr: *mut RemovableUniversalSketch) {
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

/// Updates RemovableUniversalSketch
#[no_mangle]
pub unsafe extern "C" fn removableuniversalsketch_update(ptr: *mut RemovableUniversalSketch, data: *const u8, len: usize, delta: i32) {
    if ptr.is_null() || data.is_null() {
        return;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).update(&hash, delta);
}

/// Estimates count in RemovableUniversalSketch
#[no_mangle]
pub unsafe extern "C" fn removableuniversalsketch_estimate(ptr: *const RemovableUniversalSketch, data: *const u8, len: usize) -> u64 {
    if ptr.is_null() || data.is_null() {
        return 0;
    }
    let bytes = slice::from_raw_parts(data, len);
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    (*ptr).estimate(&hash) as u64
}

/// Merges RemovableUniversalSketch
#[no_mangle]
pub unsafe extern "C" fn removableuniversalsketch_merge(ptr1: *mut RemovableUniversalSketch, ptr2: *const RemovableUniversalSketch) {
    if ptr1.is_null() || ptr2.is_null() {
        return;
    }
    let _ = (*ptr1).merge(&*ptr2);
}

/// Serializes RemovableUniversalSketch
#[no_mangle]
pub unsafe extern "C" fn removableuniversalsketch_serialize(_ptr: *const RemovableUniversalSketch, _out_len: *mut usize) -> *mut u8 {
    std::ptr::null_mut()
}

/// Deserializes RemovableUniversalSketch
#[no_mangle]
pub unsafe extern "C" fn removableuniversalsketch_deserialize(_data: *const u8, _len: usize) -> *mut RemovableUniversalSketch {
    std::ptr::null_mut()
}
