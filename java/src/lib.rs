// Java JNI Bindings for sketch_oxide
// Provides access to probabilistic data structure algorithms from Java
// v0.1.6 Expansion: Complete multi-language support

use jni::objects::{JByteArray, JClass, JObject};
use jni::sys::{jboolean, jbyteArray, jdouble, jint, jlong, JNI_FALSE};
use jni::JNIEnv;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use sketch_oxide::{
    // Cardinality Estimation
    cardinality::{CpcSketch, HyperLogLog, QSketch, ThetaSketch, UltraLogLog},
    // Frequency Estimation
    frequency::{
        ConservativeCountMin, CountMinSketch, CountSketch, ElasticSketch, FrequentItems,
        HeavyKeeper, NitroSketch, RemovableUniversalSketch, SpaceSaving, SALSA,
    },
    // Membership Testing
    membership::{
        BinaryFuseFilter, BlockedBloomFilter, BloomFilter, CountingBloomFilter, CuckooFilter,
        LearnedBloomFilter, RibbonFilter, StableBloomFilter, VacuumFilter,
    },
    // Quantiles
    quantiles::{DDSketch, KllSketch, ReqMode, ReqSketch, SplineSketch, TDigest},
    // Range Filters
    range_filters::{Grafite, MementoFilter, GRF},
    // Reconciliation
    reconciliation::RatelessIBLT,
    // Sampling
    sampling::{ReservoirSampling, VarOptSampling},
    // Similarity
    similarity::{MinHash, SimHash},
    // Streaming
    streaming::{ExponentialHistogram, SlidingHyperLogLog, SlidingWindowCounter},
    // Universal
    universal::UnivMon,
    Mergeable,
    RangeFilter,
    Sketch,
};

// ============================================================================
// CARDINALITY ESTIMATION - HyperLogLog
// ============================================================================

/// Create a new HyperLogLog sketch
/// Args: precision (4-16)
/// Returns: pointer to native HyperLogLog instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_new(
    _env: JNIEnv,
    _: JClass,
    precision: jint,
) -> jlong {
    match HyperLogLog::new(precision as u8) {
        Ok(hll) => Box::into_raw(Box::new(hll)) as jlong,
        Err(_) => 0,
    }
}

/// Add an item to the HyperLogLog sketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let hll = unsafe { &mut *(ptr as *mut HyperLogLog) };

    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        hll.update(&bytes);
    }
}

/// Get cardinality estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_estimate(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let hll = unsafe { &*(ptr as *const HyperLogLog) };
    hll.estimate()
}

/// Merge another HyperLogLog into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let hll1 = unsafe { &mut *(ptr1 as *mut HyperLogLog) };
    let hll2 = unsafe { &*(ptr2 as *const HyperLogLog) };
    let _ = hll1.merge(hll2);
}

/// Get precision parameter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_precision(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let hll = unsafe { &*(ptr as *const HyperLogLog) };
    hll.precision() as jint
}

/// Serialize to binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_serialize(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jbyteArray {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    let hll = unsafe { &*(ptr as *const HyperLogLog) };
    let data = hll.to_bytes();
    match env.byte_array_from_slice(&data) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize from binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_deserialize(
    env: JNIEnv,
    _: JClass,
    data: jbyteArray,
) -> jlong {
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(data_vec) = env.convert_byte_array(arr) {
        if let Ok(hll) = HyperLogLog::from_bytes(&data_vec) {
            return Box::into_raw(Box::new(hll)) as jlong;
        }
    }
    0
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut HyperLogLog) };
    }
}

// ============================================================================
// CARDINALITY ESTIMATION - UltraLogLog (v0.1.6 Addition)
// ============================================================================

/// Create a new UltraLogLog sketch
/// Args: precision (4-18)
/// Returns: pointer to native UltraLogLog instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UltraLogLog_new(
    _env: JNIEnv,
    _: JClass,
    precision: jint,
) -> jlong {
    match UltraLogLog::new(precision as u8) {
        Ok(ull) => Box::into_raw(Box::new(ull)) as jlong,
        Err(_) => 0,
    }
}

/// Add an item to the UltraLogLog sketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UltraLogLog_add(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let ull = unsafe { &mut *(ptr as *mut UltraLogLog) };

    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        ull.add(&bytes);
    }
}

/// Get cardinality estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UltraLogLog_cardinality(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let ull = unsafe { &*(ptr as *const UltraLogLog) };
    ull.cardinality()
}

/// Merge another UltraLogLog into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UltraLogLog_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let ull1 = unsafe { &mut *(ptr1 as *mut UltraLogLog) };
    let ull2 = unsafe { &*(ptr2 as *const UltraLogLog) };
    let _ = ull1.merge(ull2);
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UltraLogLog_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut UltraLogLog) };
    }
}

// ============================================================================
// CARDINALITY ESTIMATION - CpcSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new CpcSketch
/// Args: lg_k (4-20)
/// Returns: pointer to native CpcSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CpcSketch_new(
    _env: JNIEnv,
    _: JClass,
    lg_k: jint,
) -> jlong {
    match CpcSketch::new(lg_k as u8) {
        Ok(cpc) => Box::into_raw(Box::new(cpc)) as jlong,
        Err(_) => 0,
    }
}

/// Add an item to the CpcSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CpcSketch_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let cpc = unsafe { &mut *(ptr as *mut CpcSketch) };

    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        cpc.update(&hash);
    }
}

/// Get cardinality estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CpcSketch_estimate(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let cpc = unsafe { &*(ptr as *const CpcSketch) };
    cpc.estimate()
}

/// Get lg_k parameter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CpcSketch_lg_k(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let cpc = unsafe { &*(ptr as *const CpcSketch) };
    cpc.lg_k() as jint
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CpcSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut CpcSketch) };
    }
}

// ============================================================================
// CARDINALITY ESTIMATION - QSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new QSketch
/// Args: max_samples
/// Returns: pointer to native QSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_QSketch_new(
    _env: JNIEnv,
    _: JClass,
    max_samples: jlong,
) -> jlong {
    Box::into_raw(Box::new(QSketch::new(max_samples as usize))) as jlong
}

/// Add an item with weight to the QSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_QSketch_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    weight: jdouble,
) {
    if ptr == 0 {
        return;
    }

    let qsketch = unsafe { &mut *(ptr as *mut QSketch) };

    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        qsketch.update(&bytes, weight);
    }
}

/// Get cardinality estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_QSketch_estimate_distinct_elements(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let qsketch = unsafe { &*(ptr as *const QSketch) };
    qsketch.estimate_distinct_elements() as jlong
}

/// Merge another QSketch into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_QSketch_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let qsketch1 = unsafe { &mut *(ptr1 as *mut QSketch) };
    let qsketch2 = unsafe { &*(ptr2 as *const QSketch) };
    let _ = qsketch1.merge(qsketch2);
}

/// Get max samples parameter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_QSketch_max_samples(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let qsketch = unsafe { &*(ptr as *const QSketch) };
    qsketch.max_samples() as jlong
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_QSketch_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut QSketch) };
    }
}

// ============================================================================
// CARDINALITY ESTIMATION - ThetaSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new ThetaSketch
/// Args: lg_k (4-26)
/// Returns: pointer to native ThetaSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ThetaSketch_new(
    _env: JNIEnv,
    _: JClass,
    lg_k: jint,
) -> jlong {
    match ThetaSketch::new(lg_k as u8) {
        Ok(theta) => Box::into_raw(Box::new(theta)) as jlong,
        Err(_) => 0,
    }
}

/// Add an item to the ThetaSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ThetaSketch_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let theta = unsafe { &mut *(ptr as *mut ThetaSketch) };

    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        theta.update(&bytes);
    }
}

/// Get cardinality estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ThetaSketch_estimate(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let theta = unsafe { &*(ptr as *const ThetaSketch) };
    theta.estimate()
}

/// Get number of retained values
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ThetaSketch_num_retained(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let theta = unsafe { &*(ptr as *const ThetaSketch) };
    theta.num_retained() as jlong
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ThetaSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut ThetaSketch) };
    }
}

// ============================================================================
// MEMBERSHIP TESTING - BloomFilter
// ============================================================================

/// Create a new BloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_new(
    _env: JNIEnv,
    _: JClass,
    n: jlong,
    fpr: jdouble,
) -> jlong {
    let bf = BloomFilter::new(n as usize, fpr);
    Box::into_raw(Box::new(bf)) as jlong
}

/// Insert an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let bf = unsafe { &mut *(ptr as *mut BloomFilter) };

    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        bf.insert(&bytes);
    }
}

/// Check if an item might be in the set
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_contains(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }

    let bf = unsafe { &*(ptr as *const BloomFilter) };

    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => bf.contains(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Merge another BloomFilter into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let bf1 = unsafe { &mut *(ptr1 as *mut BloomFilter) };
    let bf2 = unsafe { &*(ptr2 as *const BloomFilter) };
    bf1.merge(bf2);
}

/// Serialize to binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_serialize(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jbyteArray {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    let bf = unsafe { &*(ptr as *const BloomFilter) };
    let data = bf.to_bytes();
    match env.byte_array_from_slice(&data) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize from binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_deserialize(
    env: JNIEnv,
    _: JClass,
    data: jbyteArray,
) -> jlong {
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(data_vec) = env.convert_byte_array(arr) {
        if let Ok(bf) = BloomFilter::from_bytes(&data_vec) {
            return Box::into_raw(Box::new(bf)) as jlong;
        }
    }
    0
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut BloomFilter) };
    }
}

// ============================================================================
// FREQUENCY ESTIMATION - CountMinSketch
// ============================================================================

/// Create a new CountMinSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_new(
    _env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match CountMinSketch::new(epsilon, delta) {
        Ok(cms) => Box::into_raw(Box::new(cms)) as jlong,
        Err(_) => 0,
    }
}

/// Update with an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let cms = unsafe { &mut *(ptr as *mut CountMinSketch) };

    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        cms.update(&bytes);
    }
}

/// Get frequency estimate for an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }

    let cms = unsafe { &*(ptr as *const CountMinSketch) };

    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => cms.estimate(&bytes) as jlong,
        Err(_) => 0,
    }
}

/// Merge another CountMinSketch into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let cms1 = unsafe { &mut *(ptr1 as *mut CountMinSketch) };
    let cms2 = unsafe { &*(ptr2 as *const CountMinSketch) };
    let _ = cms1.merge(cms2);
}

/// Get width parameter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_width(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let cms = unsafe { &*(ptr as *const CountMinSketch) };
    cms.width() as jint
}

/// Get depth parameter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_depth(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let cms = unsafe { &*(ptr as *const CountMinSketch) };
    cms.depth() as jint
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut CountMinSketch) };
    }
}

// ============================================================================
// MEMBERSHIP TESTING - BlockedBloomFilter (v0.1.6 Addition)
// ============================================================================

/// Create a new BlockedBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BlockedBloomFilter_new(
    _env: JNIEnv,
    _: JClass,
    n: jlong,
    fpr: jdouble,
) -> jlong {
    let bf = BlockedBloomFilter::new(n as usize, fpr);
    Box::into_raw(Box::new(bf)) as jlong
}

/// Insert an item into BlockedBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BlockedBloomFilter_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let bf = unsafe { &mut *(ptr as *mut BlockedBloomFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        bf.insert(&bytes);
    }
}

/// Check if item exists in BlockedBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BlockedBloomFilter_contains(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let bf = unsafe { &*(ptr as *const BlockedBloomFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => bf.contains(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Merge another BlockedBloomFilter into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BlockedBloomFilter_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let bf1 = unsafe { &mut *(ptr1 as *mut BlockedBloomFilter) };
    let bf2 = unsafe { &*(ptr2 as *const BlockedBloomFilter) };
    bf1.merge(bf2);
}

/// Free BlockedBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BlockedBloomFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut BlockedBloomFilter) };
    }
}

// ============================================================================
// MEMBERSHIP TESTING - CountingBloomFilter (v0.1.6 Addition)
// ============================================================================

/// Create a new CountingBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountingBloomFilter_new(
    _env: JNIEnv,
    _: JClass,
    n: jlong,
    fpr: jdouble,
) -> jlong {
    let cbf = CountingBloomFilter::new(n as usize, fpr);
    Box::into_raw(Box::new(cbf)) as jlong
}

/// Insert an item into CountingBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountingBloomFilter_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let cbf = unsafe { &mut *(ptr as *mut CountingBloomFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        cbf.insert(&bytes);
    }
}

/// Remove an item from CountingBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountingBloomFilter_remove(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let cbf = unsafe { &mut *(ptr as *mut CountingBloomFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => cbf.remove(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Check if item exists in CountingBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountingBloomFilter_contains(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let cbf = unsafe { &*(ptr as *const CountingBloomFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => cbf.contains(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Free CountingBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountingBloomFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut CountingBloomFilter) };
    }
}

// ============================================================================
// MEMBERSHIP TESTING - CuckooFilter (v0.1.6 Addition)
// ============================================================================

/// Create a new CuckooFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CuckooFilter_new(
    _env: JNIEnv,
    _: JClass,
    capacity: jlong,
) -> jlong {
    match CuckooFilter::new(capacity as usize) {
        Ok(cf) => Box::into_raw(Box::new(cf)) as jlong,
        Err(_) => 0,
    }
}

/// Insert an item into CuckooFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CuckooFilter_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let cf = unsafe { &mut *(ptr as *mut CuckooFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => match cf.insert(&bytes) {
            Ok(_) => 1,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// Check if item exists in CuckooFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CuckooFilter_contains(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let cf = unsafe { &*(ptr as *const CuckooFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => cf.contains(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Remove an item from CuckooFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CuckooFilter_remove(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let cf = unsafe { &mut *(ptr as *mut CuckooFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => cf.remove(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Free CuckooFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CuckooFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut CuckooFilter) };
    }
}

// ============================================================================
// MEMBERSHIP TESTING - RibbonFilter (v0.1.6 Addition)
// ============================================================================

/// Create a new RibbonFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RibbonFilter_new(
    _env: JNIEnv,
    _: JClass,
    n: jlong,
    fpr: jdouble,
) -> jlong {
    let rf = RibbonFilter::new(n as usize, fpr);
    Box::into_raw(Box::new(rf)) as jlong
}

/// Insert an item into RibbonFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RibbonFilter_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let rf = unsafe { &mut *(ptr as *mut RibbonFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        rf.insert(&bytes);
    }
}

/// Finalize RibbonFilter after insertions
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RibbonFilter_finalize(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr == 0 {
        return;
    }
    let rf = unsafe { &mut *(ptr as *mut RibbonFilter) };
    rf.finalize();
}

/// Check if item exists in RibbonFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RibbonFilter_contains(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let rf = unsafe { &*(ptr as *const RibbonFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => rf.contains(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Check if RibbonFilter is finalized
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RibbonFilter_is_finalized(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let rf = unsafe { &*(ptr as *const RibbonFilter) };
    rf.is_finalized() as jboolean
}

/// Free RibbonFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RibbonFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut RibbonFilter) };
    }
}

// ============================================================================
// MEMBERSHIP TESTING - StableBloomFilter (v0.1.6 Addition)
// ============================================================================

/// Create a new StableBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_StableBloomFilter_new(
    _env: JNIEnv,
    _: JClass,
    expected_items: jlong,
    fpr: jdouble,
) -> jlong {
    match StableBloomFilter::new(expected_items as usize, fpr) {
        Ok(sbf) => Box::into_raw(Box::new(sbf)) as jlong,
        Err(_) => 0,
    }
}

/// Insert an item into StableBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_StableBloomFilter_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sbf = unsafe { &mut *(ptr as *mut StableBloomFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        sbf.insert(&bytes);
    }
}

/// Check if item exists in StableBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_StableBloomFilter_contains(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let sbf = unsafe { &*(ptr as *const StableBloomFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => sbf.contains(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Get count for an item in StableBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_StableBloomFilter_get_count(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sbf = unsafe { &*(ptr as *const StableBloomFilter) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => sbf.get_count(&bytes) as jint,
        Err(_) => 0,
    }
}

/// Free StableBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_StableBloomFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut StableBloomFilter) };
    }
}

// ============================================================================
// QUANTILES - DDSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new DDSketch
/// Args: relative_accuracy (0.0-1.0)
/// Returns: pointer to native DDSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_DDSketch_new(
    _env: JNIEnv,
    _: JClass,
    relative_accuracy: jdouble,
) -> jlong {
    match DDSketch::new(relative_accuracy) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Add a value to the DDSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_DDSketch_add(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    value: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut DDSketch) };
    sketch.add(value);
}

/// Get quantile estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_DDSketch_quantile(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    q: jdouble,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const DDSketch) };
    sketch.quantile(q).unwrap_or(0.0)
}

/// Get minimum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_DDSketch_min(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const DDSketch) };
    sketch.min().unwrap_or(0.0)
}

/// Get maximum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_DDSketch_max(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const DDSketch) };
    sketch.max().unwrap_or(0.0)
}

/// Get count of values
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_DDSketch_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const DDSketch) };
    sketch.count() as jlong
}

/// Free DDSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_DDSketch_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut DDSketch) };
    }
}

// ============================================================================
// QUANTILES - KllSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new KllSketch
/// Args: k (sketch size parameter)
/// Returns: pointer to native KllSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_new(
    _env: JNIEnv,
    _: JClass,
    k: jint,
) -> jlong {
    match KllSketch::new(k as u16) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update KllSketch with a value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_update(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    value: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut KllSketch) };
    sketch.update(value);
}

/// Get quantile estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_quantile(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    rank: jdouble,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &mut *(ptr as *mut KllSketch) };
    sketch.quantile(rank).unwrap_or(0.0)
}

/// Get minimum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_min(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const KllSketch) };
    sketch.min()
}

/// Get maximum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_max(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const KllSketch) };
    sketch.max()
}

/// Get count of values
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const KllSketch) };
    sketch.count() as jlong
}

/// Serialize to binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_serialize(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jbyteArray {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    let sketch = unsafe { &mut *(ptr as *mut KllSketch) };
    let data = sketch.to_bytes();
    match env.byte_array_from_slice(&data) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize from binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_deserialize(
    env: JNIEnv,
    _: JClass,
    data: jbyteArray,
) -> jlong {
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(data_vec) = env.convert_byte_array(arr) {
        if let Ok(sketch) = KllSketch::from_bytes(&data_vec) {
            return Box::into_raw(Box::new(sketch)) as jlong;
        }
    }
    0
}

/// Free KllSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_KllSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut KllSketch) };
    }
}

// ============================================================================
// QUANTILES - ReqSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new ReqSketch
/// Args: max_k (maximum number of compactors)
/// Returns: pointer to native ReqSketch instance as long
/// Note: Uses HighRankAccuracy mode (zero error at maximum/p100)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReqSketch_new(
    _env: JNIEnv,
    _: JClass,
    max_k: jint,
) -> jlong {
    match ReqSketch::new(max_k as usize, ReqMode::HighRankAccuracy) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update ReqSketch with a value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReqSketch_update(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    value: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut ReqSketch) };
    sketch.update(value);
}

/// Get quantile estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReqSketch_quantile(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    q: jdouble,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const ReqSketch) };
    sketch.quantile(q).unwrap_or(0.0)
}

/// Get minimum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReqSketch_min(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const ReqSketch) };
    sketch.min().unwrap_or(0.0)
}

/// Get maximum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReqSketch_max(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const ReqSketch) };
    sketch.max().unwrap_or(0.0)
}

/// Merge another ReqSketch into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReqSketch_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let sketch1 = unsafe { &*(ptr1 as *const ReqSketch) };
    let sketch2 = unsafe { &*(ptr2 as *const ReqSketch) };
    if let Ok(merged) = sketch1.merge(sketch2) {
        let ptr = ptr1 as *mut ReqSketch;
        let _ = std::mem::replace(unsafe { &mut *ptr }, merged);
    }
}

/// Free ReqSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReqSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut ReqSketch) };
    }
}

// ============================================================================
// QUANTILES - SplineSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new SplineSketch
/// Args: max_samples (maximum number of samples to keep)
/// Returns: pointer to native SplineSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SplineSketch_new(
    _env: JNIEnv,
    _: JClass,
    max_samples: jlong,
) -> jlong {
    Box::into_raw(Box::new(SplineSketch::new(max_samples as usize))) as jlong
}

/// Update SplineSketch with a value and weight
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SplineSketch_update(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    value: jlong,
    weight: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut SplineSketch) };
    sketch.update(value as u64, weight);
}

/// Get minimum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SplineSketch_min(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SplineSketch) };
    sketch.min().unwrap_or(0) as jlong
}

/// Get maximum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SplineSketch_max(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SplineSketch) };
    sketch.max().unwrap_or(0) as jlong
}

/// Get maximum samples parameter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SplineSketch_max_samples(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SplineSketch) };
    sketch.max_samples() as jlong
}

/// Merge another SplineSketch into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SplineSketch_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let sketch1 = unsafe { &mut *(ptr1 as *mut SplineSketch) };
    let sketch2 = unsafe { &*(ptr2 as *const SplineSketch) };
    sketch1.merge_into(sketch2);
}

/// Free SplineSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SplineSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut SplineSketch) };
    }
}

// ============================================================================
// QUANTILES - TDigest (v0.1.6 Addition)
// ============================================================================

/// Create a new TDigest
/// Args: compression (compression factor, typically 100-1000)
/// Returns: pointer to native TDigest instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_new(
    _env: JNIEnv,
    _: JClass,
    compression: jdouble,
) -> jlong {
    Box::into_raw(Box::new(TDigest::new(compression))) as jlong
}

/// Update TDigest with a single value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_update(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    value: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut TDigest) };
    sketch.update(value);
}

/// Update TDigest with a weighted value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_update_weighted(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    value: jdouble,
    weight: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut TDigest) };
    sketch.update_weighted(value, weight);
}

/// Get quantile estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_quantile(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    q: jdouble,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &mut *(ptr as *mut TDigest) };
    sketch.quantile(q)
}

/// Get cumulative distribution function value at point
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_cdf(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    value: jdouble,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &mut *(ptr as *mut TDigest) };
    sketch.cdf(value)
}

/// Get minimum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_min(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const TDigest) };
    sketch.min()
}

/// Get maximum value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_max(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const TDigest) };
    sketch.max()
}

/// Get count of values
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const TDigest) };
    sketch.count()
}

/// Serialize to binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_serialize(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jbyteArray {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    let sketch = unsafe { &mut *(ptr as *mut TDigest) };
    let data = sketch.to_bytes();
    match env.byte_array_from_slice(&data) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize from binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_deserialize(
    env: JNIEnv,
    _: JClass,
    data: jbyteArray,
) -> jlong {
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(data_vec) = env.convert_byte_array(arr) {
        if let Ok(sketch) = TDigest::from_bytes(&data_vec) {
            return Box::into_raw(Box::new(sketch)) as jlong;
        }
    }
    0
}

/// Free TDigest
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_TDigest_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut TDigest) };
    }
}

// ============================================================================
// FREQUENCY ESTIMATION - CountSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new CountSketch
/// Args: epsilon, delta (error parameters)
/// Returns: pointer to native CountSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountSketch_new(
    _env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match CountSketch::new(epsilon, delta) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update CountSketch with an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountSketch_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    delta: jlong,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut CountSketch) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        sketch.update(&hash, delta as i64);
    }
}

/// Get frequency estimate for an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountSketch_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const CountSketch) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let hash = hasher.finish();
            sketch.estimate(&hash) as jlong
        }
        Err(_) => 0,
    }
}

/// Get width parameter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountSketch_width(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const CountSketch) };
    sketch.width() as jint
}

/// Get depth parameter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountSketch_depth(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const CountSketch) };
    sketch.depth() as jint
}

/// Free CountSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut CountSketch) };
    }
}

// ============================================================================
// FREQUENCY ESTIMATION - ConservativeCountMin (v0.1.6 Addition)
// ============================================================================

/// Create a new ConservativeCountMin
/// Args: epsilon, delta (error parameters)
/// Returns: pointer to native ConservativeCountMin instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ConservativeCountMin_new(
    _env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match ConservativeCountMin::new(epsilon, delta) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update ConservativeCountMin with an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ConservativeCountMin_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut ConservativeCountMin) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        sketch.update(&hash);
    }
}

/// Get frequency estimate for an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ConservativeCountMin_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ConservativeCountMin) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let hash = hasher.finish();
            sketch.estimate(&hash) as jlong
        }
        Err(_) => 0,
    }
}

/// Get total count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ConservativeCountMin_total_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ConservativeCountMin) };
    sketch.total_count() as jlong
}

/// Free ConservativeCountMin
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ConservativeCountMin_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut ConservativeCountMin) };
    }
}

// ============================================================================
// FREQUENCY ESTIMATION - ElasticSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new ElasticSketch
/// Args: bucket_count, depth
/// Returns: pointer to native ElasticSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ElasticSketch_new(
    _env: JNIEnv,
    _: JClass,
    bucket_count: jint,
    depth: jint,
) -> jlong {
    match ElasticSketch::new(bucket_count as usize, depth as usize) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update ElasticSketch with an item and count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ElasticSketch_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    count: jlong,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut ElasticSketch) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        sketch.update(&bytes, count as u64);
    }
}

/// Get frequency estimate for an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ElasticSketch_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ElasticSketch) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => sketch.estimate(&bytes) as jlong,
        Err(_) => 0,
    }
}

/// Get bucket count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ElasticSketch_bucket_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ElasticSketch) };
    sketch.bucket_count() as jint
}

/// Free ElasticSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ElasticSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut ElasticSketch) };
    }
}

// ============================================================================
// FREQUENCY ESTIMATION - SALSA (v0.1.6 Addition)
// ============================================================================

/// Create a new SALSA sketch
/// Args: epsilon, delta (error parameters)
/// Returns: pointer to native SALSA instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SALSA_new(
    _env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match SALSA::new(epsilon, delta) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update SALSA with an item and count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SALSA_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    count: jlong,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut SALSA) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        sketch.update(&hash, count as u64);
    }
}

/// Get frequency estimate for an item (returns estimate as jlong)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SALSA_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SALSA) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let hash = hasher.finish();
            let (estimate, _confidence) = sketch.estimate(&hash);
            estimate as jlong
        }
        Err(_) => 0,
    }
}

/// Free SALSA
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SALSA_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut SALSA) };
    }
}

// ============================================================================
// FREQUENCY ESTIMATION - RemovableUniversalSketch (v0.1.6 Addition)
// ============================================================================

/// Create a new RemovableUniversalSketch
/// Args: epsilon, delta (error parameters)
/// Returns: pointer to native RemovableUniversalSketch instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RemovableUniversalSketch_new(
    _env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match RemovableUniversalSketch::new(epsilon, delta) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update RemovableUniversalSketch with an item (supports positive/negative deltas)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RemovableUniversalSketch_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    delta: jint,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut RemovableUniversalSketch) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        sketch.update(&hash, delta);
    }
}

/// Get frequency estimate for an item (returns signed i64)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RemovableUniversalSketch_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const RemovableUniversalSketch) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            let hash = hasher.finish();
            sketch.estimate(&hash)
        }
        Err(_) => 0,
    }
}

/// Free RemovableUniversalSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RemovableUniversalSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut RemovableUniversalSketch) };
    }
}

// ============================================================================
// FREQUENCY ESTIMATION - HeavyKeeper (v0.1.6 Addition)
// ============================================================================

/// Create a new HeavyKeeper sketch
/// Args: k (number of heavy hitters to track), epsilon, delta
/// Returns: pointer to native HeavyKeeper instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HeavyKeeper_new(
    _env: JNIEnv,
    _: JClass,
    k: jint,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match HeavyKeeper::new(k as usize, epsilon, delta) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update HeavyKeeper with an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HeavyKeeper_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut HeavyKeeper) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        sketch.update(&bytes);
    }
}

/// Get frequency estimate for an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HeavyKeeper_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const HeavyKeeper) };
    let arr = unsafe { JByteArray::from_raw(data) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => sketch.estimate(&bytes) as jint,
        Err(_) => 0,
    }
}

/// Apply decay to the sketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HeavyKeeper_decay(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut HeavyKeeper) };
    sketch.decay();
}

/// Free HeavyKeeper
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HeavyKeeper_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut HeavyKeeper) };
    }
}

// ============================================================================
// SIMILARITY - MinHash (v0.1.6 Addition)
// ============================================================================

/// Create a new MinHash sketch
/// Args: num_perm (number of hash permutations, typically 128+)
/// Returns: pointer to native MinHash instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MinHash_new(
    _env: JNIEnv,
    _: JClass,
    num_perm: jint,
) -> jlong {
    match MinHash::new(num_perm as usize) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update MinHash with an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MinHash_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut MinHash) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        sketch.update(&hash);
    }
}

/// Get Jaccard similarity with another MinHash
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MinHash_jaccard_similarity(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) -> jdouble {
    if ptr1 == 0 || ptr2 == 0 {
        return 0.0;
    }
    let sketch1 = unsafe { &*(ptr1 as *const MinHash) };
    let sketch2 = unsafe { &*(ptr2 as *const MinHash) };
    sketch1.jaccard_similarity(sketch2).unwrap_or(0.0)
}

/// Get number of permutations
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MinHash_num_perm(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const MinHash) };
    sketch.num_perm() as jint
}

/// Free MinHash
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MinHash_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut MinHash) };
    }
}

// ============================================================================
// SIMILARITY - SimHash (v0.1.6 Addition)
// ============================================================================

/// Create a new SimHash sketch
/// Returns: pointer to native SimHash instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SimHash_new(_env: JNIEnv, _: JClass) -> jlong {
    Box::into_raw(Box::new(SimHash::new())) as jlong
}

/// Update SimHash with a feature
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SimHash_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut SimHash) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        sketch.update(&hash);
    }
}

/// Get SimHash fingerprint
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SimHash_fingerprint(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &mut *(ptr as *mut SimHash) };
    sketch.fingerprint() as jlong
}

/// Get Hamming distance to another SimHash
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SimHash_hamming_distance(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) -> jint {
    if ptr1 == 0 || ptr2 == 0 {
        return 0;
    }
    let sketch1 = unsafe { &mut *(ptr1 as *mut SimHash) };
    let sketch2 = unsafe { &mut *(ptr2 as *mut SimHash) };
    sketch1.hamming_distance(sketch2) as jint
}

/// Get Similarity to another SimHash
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SimHash_similarity(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) -> jdouble {
    if ptr1 == 0 || ptr2 == 0 {
        return 0.0;
    }
    let sketch1 = unsafe { &mut *(ptr1 as *mut SimHash) };
    let sketch2 = unsafe { &mut *(ptr2 as *mut SimHash) };
    sketch1.similarity(sketch2)
}

/// Get number of features
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SimHash_len(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SimHash) };
    sketch.len() as jlong
}

/// Free SimHash
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SimHash_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut SimHash) };
    }
}

// ============================================================================
// STREAMING - SlidingWindowCounter (v0.1.6 Addition)
// ============================================================================

/// Create a new SlidingWindowCounter
/// Args: window_size, epsilon
/// Returns: pointer to native SlidingWindowCounter instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingWindowCounter_new(
    _env: JNIEnv,
    _: JClass,
    window_size: jlong,
    epsilon: jdouble,
) -> jlong {
    match SlidingWindowCounter::new(window_size as u64, epsilon) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Increment counter at timestamp
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingWindowCounter_increment(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    timestamp: jlong,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut SlidingWindowCounter) };
    sketch.increment(timestamp as u64);
}

/// Increment by count at timestamp
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingWindowCounter_increment_by(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    timestamp: jlong,
    count: jlong,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut SlidingWindowCounter) };
    sketch.increment_by(timestamp as u64, count as u64);
}

/// Get count in window
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingWindowCounter_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    current_time: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SlidingWindowCounter) };
    sketch.count(current_time as u64) as jlong
}

/// Get window size
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingWindowCounter_window_size(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SlidingWindowCounter) };
    sketch.window_size() as jlong
}

/// Free SlidingWindowCounter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingWindowCounter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut SlidingWindowCounter) };
    }
}

// ============================================================================
// STREAMING - ExponentialHistogram (v0.1.6 Addition)
// ============================================================================

/// Create a new ExponentialHistogram
/// Args: window_size, epsilon
/// Returns: pointer to native ExponentialHistogram instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ExponentialHistogram_new(
    _env: JNIEnv,
    _: JClass,
    window_size: jlong,
    epsilon: jdouble,
) -> jlong {
    match ExponentialHistogram::new(window_size as u64, epsilon) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Insert event with count at timestamp
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ExponentialHistogram_insert(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    timestamp: jlong,
    count: jlong,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut ExponentialHistogram) };
    sketch.insert(timestamp as u64, count as u64);
}

/// Get count in current window (returns estimate as jlong)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ExponentialHistogram_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    current_time: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ExponentialHistogram) };
    let (estimate, _lower, _upper) = sketch.count(current_time as u64);
    estimate as jlong
}

/// Get window size
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ExponentialHistogram_window_size(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ExponentialHistogram) };
    sketch.window_size() as jlong
}

/// Free ExponentialHistogram
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ExponentialHistogram_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut ExponentialHistogram) };
    }
}

// ============================================================================
// STREAMING - SlidingHyperLogLog (v0.1.6 Addition)
// ============================================================================

/// Create a new SlidingHyperLogLog
/// Args: precision, max_window_seconds
/// Returns: pointer to native SlidingHyperLogLog instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingHyperLogLog_new(
    _env: JNIEnv,
    _: JClass,
    precision: jint,
    max_window_seconds: jlong,
) -> jlong {
    match SlidingHyperLogLog::new(precision as u8, max_window_seconds as u64) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update SlidingHyperLogLog with item at timestamp
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingHyperLogLog_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    timestamp: jlong,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut SlidingHyperLogLog) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();
        let _ = sketch.update(&hash, timestamp as u64);
    }
}

/// Estimate cardinality in window
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingHyperLogLog_estimate_window(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    current_time: jlong,
    window_seconds: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const SlidingHyperLogLog) };
    sketch.estimate_window(current_time as u64, window_seconds as u64)
}

/// Get precision
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingHyperLogLog_precision(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SlidingHyperLogLog) };
    sketch.precision() as jint
}

/// Free SlidingHyperLogLog
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SlidingHyperLogLog_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut SlidingHyperLogLog) };
    }
}

// ============================================================================
// RECONCILIATION - RatelessIBLT (v0.1.6 Addition)
// ============================================================================

/// Create a new RatelessIBLT
/// Args: expected_diff, cell_size
/// Returns: pointer to native RatelessIBLT instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RatelessIBLT_new(
    _env: JNIEnv,
    _: JClass,
    expected_diff: jint,
    cell_size: jint,
) -> jlong {
    match RatelessIBLT::new(expected_diff as usize, cell_size as usize) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Insert key-value pair
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RatelessIBLT_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jbyteArray,
    value: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut RatelessIBLT) };
    let key_arr = unsafe { JByteArray::from_raw(key) };
    let val_arr = unsafe { JByteArray::from_raw(value) };
    if let (Ok(key_bytes), Ok(val_bytes)) = (
        env.convert_byte_array(key_arr),
        env.convert_byte_array(val_arr),
    ) {
        let _ = sketch.insert(&key_bytes, &val_bytes);
    }
}

/// Delete key-value pair
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RatelessIBLT_delete(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jbyteArray,
    value: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut RatelessIBLT) };
    let key_arr = unsafe { JByteArray::from_raw(key) };
    let val_arr = unsafe { JByteArray::from_raw(value) };
    if let (Ok(key_bytes), Ok(val_bytes)) = (
        env.convert_byte_array(key_arr),
        env.convert_byte_array(val_arr),
    ) {
        let _ = sketch.delete(&key_bytes, &val_bytes);
    }
}

/// Free RatelessIBLT
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_RatelessIBLT_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut RatelessIBLT) };
    }
}

// ============================================================================
// UNIVERSAL - UnivMon (v0.1.6 Addition)
// ============================================================================

/// Create a new UnivMon sketch
/// Args: max_stream_size, epsilon, delta
/// Returns: pointer to native UnivMon instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_new(
    _env: JNIEnv,
    _: JClass,
    max_stream_size: jlong,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match UnivMon::new(max_stream_size as u64, epsilon, delta) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update UnivMon with item and value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    value: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut UnivMon) };
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let _ = sketch.update(&bytes, value);
    }
}

/// Estimate L1 (sum of all values)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_estimate_l1(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const UnivMon) };
    sketch.estimate_l1()
}

/// Estimate L2 (sum of squared values)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_estimate_l2(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const UnivMon) };
    sketch.estimate_l2()
}

/// Estimate entropy
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_estimate_entropy(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const UnivMon) };
    sketch.estimate_entropy()
}

/// Get number of layers
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_num_layers(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const UnivMon) };
    sketch.num_layers() as jint
}

/// Get total updates
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_total_updates(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const UnivMon) };
    sketch.total_updates() as jlong
}

/// Free UnivMon
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut UnivMon) };
    }
}

// ============================================================================
// RANGE FILTERS - MementoFilter (v0.1.6 Addition)
// ============================================================================

/// Create a new MementoFilter
/// Args: expected_elements, fpr
/// Returns: pointer to native MementoFilter instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MementoFilter_new(
    _env: JNIEnv,
    _: JClass,
    expected_elements: jlong,
    fpr: jdouble,
) -> jlong {
    match MementoFilter::new(expected_elements as usize, fpr) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Insert range (key, value) into MementoFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MementoFilter_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jlong,
    value: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut MementoFilter) };
    let arr = unsafe { JByteArray::from_raw(value) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let _ = sketch.insert(key as u64, &bytes);
    }
}

/// Check if range may contain value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MementoFilter_may_contain_range(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    low: jlong,
    high: jlong,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const MementoFilter) };
    sketch.may_contain_range(low as u64, high as u64) as jboolean
}

/// Get number of elements
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MementoFilter_len(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const MementoFilter) };
    sketch.len() as jlong
}

/// Free MementoFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_MementoFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut MementoFilter) };
    }
}

// ============================================================================
// RANGE FILTERS - Grafite (v0.1.6 Addition)
// ============================================================================

/// Create a new Grafite from keys (build-once sketch)
/// Args: keys array (encoded as bytes), bits_per_key
/// Note: In JNI, we pass keys as a concatenated byte array of u64s in native order
/// Returns: pointer to native Grafite instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_Grafite_new(
    env: JNIEnv,
    _: JClass,
    keys: jbyteArray,
    bits_per_key: jint,
) -> jlong {
    let arr = unsafe { JByteArray::from_raw(keys) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        // Decode bytes as array of u64 (8 bytes each, little-endian)
        let mut key_vec = Vec::new();
        for chunk in bytes.chunks_exact(8) {
            if let Ok(arr) = <[u8; 8]>::try_from(chunk) {
                key_vec.push(u64::from_le_bytes(arr));
            }
        }

        if !key_vec.is_empty() {
            if let Ok(sketch) = Grafite::build(&key_vec, bits_per_key as usize) {
                return Box::into_raw(Box::new(sketch)) as jlong;
            }
        }
    }
    0
}

/// Check if range may contain value in Grafite
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_Grafite_may_contain_range(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    low: jlong,
    high: jlong,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const Grafite) };
    sketch.may_contain_range(low as u64, high as u64) as jboolean
}

/// Check if value may be in Grafite
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_Grafite_may_contain(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jlong,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const Grafite) };
    sketch.may_contain(key as u64) as jboolean
}

/// Get key count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_Grafite_key_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const Grafite) };
    sketch.key_count() as jlong
}

/// Get bits per key
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_Grafite_bits_per_key(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const Grafite) };
    sketch.bits_per_key() as jint
}

/// Free Grafite
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_Grafite_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut Grafite) };
    }
}

// ============================================================================
// RANGE FILTERS - GRF (Gorilla Range Filter) (v0.1.6 Addition)
// ============================================================================

/// Create a new GRF from keys
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_new(
    env: JNIEnv,
    _: JClass,
    keys: jbyteArray,
    bits_per_key: jint,
) -> jlong {
    let arr = unsafe { JByteArray::from_raw(keys) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let mut key_vec = Vec::new();
        for chunk in bytes.chunks_exact(8) {
            if let Ok(arr) = <[u8; 8]>::try_from(chunk) {
                key_vec.push(u64::from_le_bytes(arr));
            }
        }
        if !key_vec.is_empty() && bits_per_key > 0 {
            if let Ok(sketch) = GRF::build(&key_vec, bits_per_key as usize) {
                return Box::into_raw(Box::new(sketch)) as jlong;
            }
        }
    }
    0
}

/// Check if key may be in GRF
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_may_contain(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jlong,
) -> jboolean {
    if ptr == 0 {
        return JNI_FALSE;
    }
    let sketch = unsafe { &*(ptr as *const GRF) };
    sketch.may_contain(key as u64) as jboolean
}

/// Check if range may contain keys
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_may_contain_range(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    low: jlong,
    high: jlong,
) -> jboolean {
    if ptr == 0 {
        return JNI_FALSE;
    }
    let sketch = unsafe { &*(ptr as *const GRF) };
    sketch.may_contain_range(low as u64, high as u64) as jboolean
}

/// Get key count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_key_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const GRF) };
    sketch.key_count() as jlong
}

/// Get bits per key
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_bits_per_key(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const GRF) };
    sketch.bits_per_key() as jint
}

/// Free GRF
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut GRF) };
    }
}

// ============================================================================
// FREQUENCY - SpaceSaving (v0.1.6 Addition)
// ============================================================================

/// Create a new SpaceSaving sketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SpaceSaving_new(
    _env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
) -> jlong {
    if epsilon <= 0.0 || epsilon >= 1.0 {
        return 0;
    }
    match SpaceSaving::<u64>::new(epsilon) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update SpaceSaving with item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SpaceSaving_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        let sketch = unsafe { &mut *(ptr as *mut SpaceSaving<u64>) };
        sketch.update(hash);
    }
}

/// Estimate frequency
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SpaceSaving_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        let sketch = unsafe { &*(ptr as *const SpaceSaving<u64>) };
        match sketch.estimate(&hash) {
            Some((count, _error)) => count as jlong,
            None => 0,
        }
    } else {
        0
    }
}

/// Get capacity
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SpaceSaving_capacity(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SpaceSaving<u64>) };
    sketch.capacity() as jint
}

/// Get stream length
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SpaceSaving_stream_length(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const SpaceSaving<u64>) };
    sketch.stream_length() as jlong
}

/// Free SpaceSaving
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_SpaceSaving_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut SpaceSaving<u64>) };
    }
}

// ============================================================================
// FREQUENCY - FrequentItems (v0.1.6 Addition)
// ============================================================================

/// Create a new FrequentItems sketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_FrequentItems_new(
    _env: JNIEnv,
    _: JClass,
    max_size: jint,
) -> jlong {
    if max_size <= 0 {
        return 0;
    }
    match FrequentItems::<u64>::new(max_size as usize) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update FrequentItems with item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_FrequentItems_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        let sketch = unsafe { &mut *(ptr as *mut FrequentItems<u64>) };
        sketch.update(hash);
    }
}

/// Update with count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_FrequentItems_update_by(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    count: jlong,
) {
    if ptr == 0 {
        return;
    }
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        let sketch = unsafe { &mut *(ptr as *mut FrequentItems<u64>) };
        sketch.update_by(hash, count as u64);
    }
}

/// Estimate frequency
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_FrequentItems_estimate(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        let sketch = unsafe { &*(ptr as *const FrequentItems<u64>) };
        match sketch.get_estimate(&hash) {
            Some((count, _error)) => count as jlong,
            None => 0,
        }
    } else {
        0
    }
}

/// Get number of items
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_FrequentItems_num_items(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const FrequentItems<u64>) };
    sketch.num_items() as jint
}

/// Get max size
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_FrequentItems_max_size(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const FrequentItems<u64>) };
    sketch.max_size() as jint
}

/// Free FrequentItems
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_FrequentItems_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut FrequentItems<u64>) };
    }
}

// ============================================================================
// SAMPLING - ReservoirSampling (v0.1.6 Addition)
// ============================================================================

/// Create a new ReservoirSampling sketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReservoirSampling_new(
    _env: JNIEnv,
    _: JClass,
    capacity: jint,
) -> jlong {
    if capacity <= 0 {
        return 0;
    }
    let sketch = ReservoirSampling::<u64>::new(capacity as usize);
    Box::into_raw(Box::new(sketch)) as jlong
}

/// Update ReservoirSampling with item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReservoirSampling_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        let sketch = unsafe { &mut *(ptr as *mut ReservoirSampling<u64>) };
        sketch.update(hash);
    }
}

/// Get sample length
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReservoirSampling_len(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ReservoirSampling<u64>) };
    sketch.len() as jlong
}

/// Get capacity
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReservoirSampling_capacity(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ReservoirSampling<u64>) };
    sketch.capacity() as jint
}

/// Get count (stream length)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReservoirSampling_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const ReservoirSampling<u64>) };
    sketch.count() as jlong
}

/// Free ReservoirSampling
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_ReservoirSampling_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut ReservoirSampling<u64>) };
    }
}

// ============================================================================
// SAMPLING - VarOptSampling (v0.1.6 Addition)
// ============================================================================

/// Create a new VarOptSampling sketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VarOptSampling_new(
    _env: JNIEnv,
    _: JClass,
    capacity: jint,
) -> jlong {
    if capacity <= 0 {
        return 0;
    }
    let sketch = VarOptSampling::<u64>::new(capacity as usize);
    Box::into_raw(Box::new(sketch)) as jlong
}

/// Update VarOptSampling with weighted item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VarOptSampling_update(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    weight: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let arr = unsafe { JByteArray::from_raw(data) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        let mut hasher = DefaultHasher::new();
        bytes.hash(&mut hasher);
        let hash = hasher.finish();

        let sketch = unsafe { &mut *(ptr as *mut VarOptSampling<u64>) };
        sketch.update(hash, weight);
    }
}

/// Get sample length
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VarOptSampling_len(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const VarOptSampling<u64>) };
    sketch.len() as jlong
}

/// Get capacity
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VarOptSampling_capacity(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const VarOptSampling<u64>) };
    sketch.capacity() as jint
}

/// Get count (stream length)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VarOptSampling_count(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const VarOptSampling<u64>) };
    sketch.count() as jlong
}

/// Get total weight
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VarOptSampling_total_weight(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const VarOptSampling<u64>) };
    sketch.total_weight()
}

/// Free VarOptSampling
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VarOptSampling_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut VarOptSampling<u64>) };
    }
}

// ============================================================================
// MEMBERSHIP - BinaryFuseFilter (Static Filter)
// ============================================================================

/// Create a new BinaryFuseFilter from an array of items
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_membership_BinaryFuseFilter_new(
    env: JNIEnv,
    _: JClass,
    items: jni::sys::jlongArray,
    bits_per_entry: jint,
) -> jlong {
    let items_arr = unsafe { jni::objects::JPrimitiveArray::<i64>::from_raw(items) };
    let len = match env.get_array_length(&items_arr) {
        Ok(l) => l as usize,
        Err(_) => return 0,
    };

    let mut items_vec = vec![0i64; len];
    if let Ok(_) = env.get_long_array_region(&items_arr, 0, &mut items_vec) {
        let hashes: Vec<u64> = items_vec.iter().map(|&i| i as u64).collect();
        match BinaryFuseFilter::from_items(hashes, bits_per_entry as u8) {
            Ok(filter) => Box::into_raw(Box::new(filter)) as jlong,
            Err(_) => 0,
        }
    } else {
        0
    }
}

/// Check if an item is in the BinaryFuseFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_membership_BinaryFuseFilter_contains(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    item_hash: jlong,
) -> jboolean {
    if ptr == 0 {
        return jni::sys::JNI_FALSE;
    }
    let filter = unsafe { &*(ptr as *const BinaryFuseFilter) };
    if filter.contains(&(item_hash as u64)) {
        jni::sys::JNI_TRUE as jboolean
    } else {
        jni::sys::JNI_FALSE as jboolean
    }
}

/// Serialize BinaryFuseFilter to bytes (stub)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_membership_BinaryFuseFilter_serialize(
    env: JNIEnv,
    _: JObject,
    _ptr: jlong,
) -> jbyteArray {
    // BinaryFuseFilter doesn't support serialization in current library version
    match env.byte_array_from_slice(&[]) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize BinaryFuseFilter from bytes (stub)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_membership_BinaryFuseFilter_deserialize(
    _env: JNIEnv,
    _: JClass,
    _data: jbyteArray,
) -> jlong {
    // BinaryFuseFilter doesn't support deserialization
    0
}

/// Free BinaryFuseFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_membership_BinaryFuseFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut BinaryFuseFilter) };
    }
}

// ============================================================================
// MEMBERSHIP - LearnedBloomFilter
// ============================================================================

/// Create a new LearnedBloomFilter from training data
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_LearnedBloomFilter_new(
    mut env: JNIEnv,
    _: JClass,
    training_keys: jni::sys::jobjectArray,
    fpr: jdouble,
) -> jlong {
    let array = unsafe { jni::objects::JObjectArray::from_raw(training_keys) };
    let len = match env.get_array_length(&array) {
        Ok(l) => l as usize,
        Err(_) => return 0,
    };

    let mut keys_vec = Vec::new();
    for i in 0..len {
        match env.get_object_array_element(&array, i as i32) {
            Ok(elem) => {
                let arr = unsafe { JByteArray::from_raw(elem.into_raw()) };
                match env.convert_byte_array(arr) {
                    Ok(bytes) => keys_vec.push(bytes),
                    Err(_) => return 0,
                }
            }
            Err(_) => return 0,
        }
    }

    match LearnedBloomFilter::new(&keys_vec, fpr as f64) {
        Ok(filter) => Box::into_raw(Box::new(filter)) as jlong,
        Err(_) => 0,
    }
}

/// Check if item is in LearnedBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_LearnedBloomFilter_contains(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return jni::sys::JNI_FALSE;
    }
    let filter = unsafe { &*(ptr as *const LearnedBloomFilter) };
    let arr = unsafe { jni::objects::JByteArray::from_raw(key) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => {
            if filter.contains(&bytes) {
                jni::sys::JNI_TRUE as jboolean
            } else {
                jni::sys::JNI_FALSE as jboolean
            }
        }
        Err(_) => jni::sys::JNI_FALSE as jboolean,
    }
}

/// Get memory usage of LearnedBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_LearnedBloomFilter_memoryUsage(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let filter = unsafe { &*(ptr as *const LearnedBloomFilter) };
    filter.memory_usage() as jlong
}

/// Free LearnedBloomFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_LearnedBloomFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut LearnedBloomFilter) };
    }
}

// ============================================================================
// MEMBERSHIP - VacuumFilter
// ============================================================================

/// Create a new VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_new(
    _env: JNIEnv,
    _: JClass,
    capacity: jlong,
    fpr: jdouble,
) -> jlong {
    match VacuumFilter::new(capacity as usize, fpr as f64) {
        Ok(filter) => Box::into_raw(Box::new(filter)) as jlong,
        Err(_) => 0,
    }
}

/// Insert an item into VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_insert(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let filter = unsafe { &mut *(ptr as *mut VacuumFilter) };
    let arr = unsafe { jni::objects::JByteArray::from_raw(key) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        filter.insert(&bytes);
    }
}

/// Check if item is in VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_contains(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return jni::sys::JNI_FALSE;
    }
    let filter = unsafe { &*(ptr as *const VacuumFilter) };
    let arr = unsafe { jni::objects::JByteArray::from_raw(key) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => {
            if filter.contains(&bytes) {
                jni::sys::JNI_TRUE as jboolean
            } else {
                jni::sys::JNI_FALSE as jboolean
            }
        }
        Err(_) => jni::sys::JNI_FALSE as jboolean,
    }
}

/// Delete an item from VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_delete(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return jni::sys::JNI_FALSE;
    }
    let filter = unsafe { &mut *(ptr as *mut VacuumFilter) };
    let arr = unsafe { jni::objects::JByteArray::from_raw(key) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => {
            if filter.delete(&bytes).unwrap_or(false) {
                jni::sys::JNI_TRUE as jboolean
            } else {
                jni::sys::JNI_FALSE as jboolean
            }
        }
        Err(_) => jni::sys::JNI_FALSE as jboolean,
    }
}

/// Clear all items from VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_clear(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let filter = unsafe { &mut *(ptr as *mut VacuumFilter) };
        filter.clear();
    }
}

/// Get load factor of VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_loadFactor(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let filter = unsafe { &*(ptr as *const VacuumFilter) };
    filter.load_factor() as jdouble
}

/// Free VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut VacuumFilter) };
    }
}

// ============================================================================
// FREQUENCY - NitroSketch
// ============================================================================

/// Create a new NitroSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_new(
    _env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
    delta: jdouble,
    sample_rate: jdouble,
) -> jlong {
    let base_sketch = match CountMinSketch::new(epsilon as f64, delta as f64) {
        Ok(sketch) => sketch,
        Err(_) => return 0,
    };
    match NitroSketch::new(base_sketch, sample_rate as f64) {
        Ok(sketch) => Box::into_raw(Box::new(sketch)) as jlong,
        Err(_) => 0,
    }
}

/// Update NitroSketch with sampled item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_updateSampled(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jbyteArray,
) {
    if ptr == 0 {
        return;
    }
    let sketch = unsafe { &mut *(ptr as *mut NitroSketch<CountMinSketch>) };
    let arr = unsafe { jni::objects::JByteArray::from_raw(key) };
    if let Ok(bytes) = env.convert_byte_array(arr) {
        sketch.update_sampled(&bytes);
    }
}

/// Query frequency from NitroSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_query(
    env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const NitroSketch<CountMinSketch>) };
    let arr = unsafe { jni::objects::JByteArray::from_raw(key) };
    match env.convert_byte_array(arr) {
        Ok(bytes) => sketch.query(&bytes) as jlong,
        Err(_) => 0,
    }
}

/// Synchronize NitroSketch for accurate estimates
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_sync(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    unsampled_weight: jdouble,
) {
    if ptr != 0 {
        let sketch = unsafe { &mut *(ptr as *mut NitroSketch<CountMinSketch>) };
        sketch.sync(unsampled_weight as f64);
    }
}

/// Get sample rate from NitroSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_sampleRate(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let sketch = unsafe { &*(ptr as *const NitroSketch<CountMinSketch>) };
    sketch.sample_rate() as jdouble
}

/// Get sampled count from NitroSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_sampledCount(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const NitroSketch<CountMinSketch>) };
    sketch.sampled_count() as jlong
}

/// Get unsampled count from NitroSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_unsampledCount(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let sketch = unsafe { &*(ptr as *const NitroSketch<CountMinSketch>) };
    sketch.unsampled_count() as jlong
}

/// Reset stats in NitroSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_resetStats(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let sketch = unsafe { &mut *(ptr as *mut NitroSketch<CountMinSketch>) };
        sketch.reset_stats();
    }
}

/// Free NitroSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut NitroSketch<CountMinSketch>) };
    }
}
