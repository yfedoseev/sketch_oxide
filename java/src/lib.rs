// Java JNI Bindings for sketch_oxide
// Provides access to probabilistic data structure algorithms from Java
// v0.1.6 Expansion: Complete multi-language support

use jni::objects::{JByteArray, JClass, JObject};
use jni::sys::{jboolean, jbyteArray, jdouble, jint, jlong};
use jni::JNIEnv;

use sketch_oxide::{
    // Cardinality Estimation
    cardinality::{CpcSketch, HyperLogLog, QSketch, ThetaSketch, UltraLogLog},
    // Frequency Estimation
    frequency::{
        ConservativeCountMin, CountMinSketch, CountSketch, ElasticSketch, HeavyKeeper,
        RemovableUniversalSketch, SALSA,
    },
    // Membership Testing
    membership::{
        BlockedBloomFilter, BloomFilter, CountingBloomFilter, CuckooFilter, RibbonFilter,
        StableBloomFilter,
    },
    // Quantiles
    quantiles::{DDSketch, KllSketch, ReqMode, ReqSketch, SplineSketch, TDigest},
    // Range Filters
    range_filters::{Grafite, GRF},
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
pub extern "system" fn Java_com_sketches_oxide_SALSA_free(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
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
pub extern "system" fn Java_com_sketches_oxide_HeavyKeeper_decay(_env: JNIEnv, _: JObject, ptr: jlong) {
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
