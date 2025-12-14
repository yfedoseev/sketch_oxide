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
    frequency::CountMinSketch,
    // Membership Testing
    membership::{
        BlockedBloomFilter, BloomFilter, CountingBloomFilter, CuckooFilter, RibbonFilter,
        StableBloomFilter,
    },
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
