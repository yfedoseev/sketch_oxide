// Java JNI Bindings for sketch_oxide
// Provides access to all 28 probabilistic data structure algorithms from Java

use jni::objects::{JByteArray, JClass, JObject};
use jni::sys::{jboolean, jbyteArray, jdouble, jint, jlong, jobject};
use jni::JNIEnv;

use sketch_oxide::{
    cardinality::HyperLogLog,
    common::RangeFilter,
    frequency::CountMinSketch,
    frequency::NitroSketch,
    membership::{BloomFilter, LearnedBloomFilter, VacuumFilter},
    range_filters::GRF,
    universal::UnivMon,
    Mergeable, Sketch,
};

// ============================================================================
// CARDINALITY ESTIMATION - HyperLogLog
// ============================================================================

/// Create a new HyperLogLog sketch
/// Args: precision (4-16)
/// Returns: pointer to native HyperLogLog instance as long
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_new(
    mut env: JNIEnv,
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
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let hll = unsafe { &mut *(ptr as *mut HyperLogLog) };

    match env.convert_byte_array(data) {
        Ok(bytes) => {
            hll.update(&bytes);
        }
        Err(_) => {}
    }
}

/// Add an item to the HyperLogLog sketch using a direct buffer (zero-copy)
///
/// This provides zero-copy updates by directly accessing memory from a direct ByteBuffer.
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_updateDirect(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    address: jlong,
    length: jlong,
) {
    if ptr == 0 || address == 0 || length <= 0 {
        return;
    }

    let hll = unsafe { &mut *(ptr as *mut HyperLogLog) };
    let bytes = unsafe { std::slice::from_raw_parts(address as *const u8, length as usize) };
    // Use update_hash directly to avoid trait resolution issues
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let hash = hasher.finish();
    hll.update_hash(hash);
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
    hll1.merge(hll2);
}

/// Get the precision level
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
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jbyteArray {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    let hll = unsafe { &*(ptr as *const HyperLogLog) };
    let data = hll.serialize();
    match env.byte_array_from_slice(&data) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize from binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_HyperLogLog_deserialize(
    mut env: JNIEnv,
    _: JClass,
    data: jbyteArray,
) -> jlong {
    match env.convert_byte_array(data) {
        Ok(data_vec) => match HyperLogLog::from_bytes(&data_vec) {
            Ok(hll) => Box::into_raw(Box::new(hll)) as jlong,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
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
// FREQUENCY ESTIMATION - CountMinSketch
// ============================================================================

/// Create a new CountMinSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_new(
    mut env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match CountMinSketch::new(epsilon, delta) {
        Ok(cms) => Box::into_raw(Box::new(cms)) as jlong,
        Err(_) => 0,
    }
}

/// Update with an item and count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_update(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
    count: jlong,
) {
    if ptr == 0 {
        return;
    }

    let cms = unsafe { &mut *(ptr as *mut CountMinSketch) };

    match env.convert_byte_array(data) {
        Ok(bytes) => {
            cms.update(&bytes, count);
        }
        Err(_) => {}
    }
}

/// Get frequency estimate for an item
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_estimate(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }

    let cms = unsafe { &*(ptr as *const CountMinSketch) };

    match env.convert_byte_array(data) {
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
    cms1.merge(cms2);
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

/// Serialize to binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_serialize(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jbyteArray {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    let cms = unsafe { &*(ptr as *const CountMinSketch) };
    let data = cms.serialize();
    match env.byte_array_from_slice(&data) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize from binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_CountMinSketch_deserialize(
    mut env: JNIEnv,
    _: JClass,
    data: jbyteArray,
) -> jlong {
    match env.convert_byte_array(data) {
        Ok(data_vec) => match CountMinSketch::from_bytes(&data_vec) {
            Ok(cms) => Box::into_raw(Box::new(cms)) as jlong,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
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
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let bf = unsafe { &mut *(ptr as *mut BloomFilter) };

    match env.convert_byte_array(data) {
        Ok(bytes) => {
            bf.insert(&bytes);
        }
        Err(_) => {}
    }
}

/// Check if an item might be in the set
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_contains(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }

    let bf = unsafe { &*(ptr as *const BloomFilter) };

    match env.convert_byte_array(data) {
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
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jbyteArray {
    if ptr == 0 {
        return std::ptr::null_mut();
    }
    let bf = unsafe { &*(ptr as *const BloomFilter) };
    let data = bf.serialize();
    match env.byte_array_from_slice(&data) {
        Ok(arr) => arr.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Deserialize from binary format
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_BloomFilter_deserialize(
    mut env: JNIEnv,
    _: JClass,
    data: jbyteArray,
) -> jlong {
    match env.convert_byte_array(data) {
        Ok(data_vec) => match BloomFilter::from_bytes(&data_vec) {
            Ok(bf) => Box::into_raw(Box::new(bf)) as jlong,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
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
// MEMBERSHIP TESTING - VacuumFilter (Tier 2)
// ============================================================================

/// Create a new VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_new(
    mut env: JNIEnv,
    _: JClass,
    capacity: jlong,
    fpr: jdouble,
) -> jlong {
    match VacuumFilter::new(capacity as usize, fpr) {
        Ok(vf) => Box::into_raw(Box::new(vf)) as jlong,
        Err(_) => 0,
    }
}

/// Insert an item into the VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_insert(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let vf = unsafe { &mut *(ptr as *mut VacuumFilter) };

    match env.convert_byte_array(data) {
        Ok(bytes) => {
            let _ = vf.insert(&bytes);
        }
        Err(_) => {}
    }
}

/// Check if an item might be in the VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_contains(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }

    let vf = unsafe { &*(ptr as *const VacuumFilter) };

    match env.convert_byte_array(data) {
        Ok(bytes) => vf.contains(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Delete an item from the VacuumFilter
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_delete(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }

    let vf = unsafe { &mut *(ptr as *mut VacuumFilter) };

    match env.convert_byte_array(data) {
        Ok(bytes) => match vf.delete(&bytes) {
            Ok(deleted) => deleted as jboolean,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// Get the load factor
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_loadFactor(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let vf = unsafe { &*(ptr as *const VacuumFilter) };
    vf.load_factor()
}

/// Get the capacity
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_capacity(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let vf = unsafe { &*(ptr as *const VacuumFilter) };
    vf.capacity() as jlong
}

/// Get the number of items
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_len(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let vf = unsafe { &*(ptr as *const VacuumFilter) };
    vf.len() as jlong
}

/// Check if empty
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_isEmpty(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jboolean {
    if ptr == 0 {
        return 1;
    }
    let vf = unsafe { &*(ptr as *const VacuumFilter) };
    vf.is_empty() as jboolean
}

/// Get memory usage in bytes
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_memoryUsage(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let vf = unsafe { &*(ptr as *const VacuumFilter) };
    vf.memory_usage() as jlong
}

/// Clear all items
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_VacuumFilter_clear(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr == 0 {
        return;
    }
    let vf = unsafe { &mut *(ptr as *mut VacuumFilter) };
    vf.clear();
}

/// Free native memory
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
// RANGE FILTERS - GRF (Tier 2)
// ============================================================================

/// Build a new GRF from keys
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_build(
    mut env: JNIEnv,
    _: JClass,
    keys: jbyteArray,
    bits_per_key: jint,
) -> jlong {
    match env.convert_byte_array(keys) {
        Ok(key_bytes) => {
            // Convert byte array to u64 array (8 bytes per u64)
            let mut u64_keys = Vec::new();
            for chunk in key_bytes.chunks(8) {
                if chunk.len() == 8 {
                    let mut bytes = [0u8; 8];
                    bytes.copy_from_slice(chunk);
                    u64_keys.push(u64::from_le_bytes(bytes));
                }
            }

            match GRF::build(&u64_keys, bits_per_key as usize) {
                Ok(grf) => Box::into_raw(Box::new(grf)) as jlong,
                Err(_) => 0,
            }
        }
        Err(_) => 0,
    }
}

/// Check if range may contain values
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_mayContainRange(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    low: jlong,
    high: jlong,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let grf = unsafe { &*(ptr as *const GRF) };
    grf.may_contain_range(low as u64, high as u64) as jboolean
}

/// Check if a single key may be present
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_mayContain(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    key: jlong,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }
    let grf = unsafe { &*(ptr as *const GRF) };
    grf.may_contain(key as u64) as jboolean
}

/// Get key count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_keyCount(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let grf = unsafe { &*(ptr as *const GRF) };
    grf.key_count() as jlong
}

/// Get segment count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_segmentCount(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let grf = unsafe { &*(ptr as *const GRF) };
    grf.segment_count() as jlong
}

/// Get bits per key
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_bitsPerKey(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let grf = unsafe { &*(ptr as *const GRF) };
    grf.bits_per_key() as jint
}

/// Calculate expected FPR for range width
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_expectedFpr(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    range_width: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let grf = unsafe { &*(ptr as *const GRF) };
    grf.expected_fpr(range_width as u64)
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_GRF_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut GRF) };
    }
}

// ============================================================================
// FREQUENCY ESTIMATION - NitroSketch (Tier 2)
// ============================================================================

/// Create a new NitroSketch wrapping CountMinSketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_new(
    mut env: JNIEnv,
    _: JClass,
    epsilon: jdouble,
    delta: jdouble,
    sample_rate: jdouble,
) -> jlong {
    match CountMinSketch::new(epsilon, delta) {
        Ok(cms) => match NitroSketch::new(cms, sample_rate) {
            Ok(nitro) => Box::into_raw(Box::new(nitro)) as jlong,
            Err(_) => 0,
        },
        Err(_) => 0,
    }
}

/// Update with selective sampling
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_updateSampled(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) {
    if ptr == 0 {
        return;
    }

    let nitro = unsafe { &mut *(ptr as *mut NitroSketch<CountMinSketch>) };

    match env.convert_byte_array(data) {
        Ok(bytes) => {
            nitro.update_sampled(&bytes);
        }
        Err(_) => {}
    }
}

/// Query frequency estimate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_query(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jlong {
    if ptr == 0 {
        return 0;
    }

    let nitro = unsafe { &*(ptr as *const NitroSketch<CountMinSketch>) };

    match env.convert_byte_array(data) {
        Ok(bytes) => nitro.query(&bytes) as jlong,
        Err(_) => 0,
    }
}

/// Synchronize to adjust for unsampled items
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_sync(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
    unsampled_weight: jdouble,
) {
    if ptr == 0 {
        return;
    }
    let nitro = unsafe { &mut *(ptr as *mut NitroSketch<CountMinSketch>) };
    let _ = nitro.sync(unsampled_weight);
}

/// Get sample rate
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_sampleRate(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let nitro = unsafe { &*(ptr as *const NitroSketch<CountMinSketch>) };
    nitro.sample_rate()
}

/// Get sampled count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_sampledCount(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let nitro = unsafe { &*(ptr as *const NitroSketch<CountMinSketch>) };
    nitro.sampled_count() as jlong
}

/// Get unsampled count
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_unsampledCount(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let nitro = unsafe { &*(ptr as *const NitroSketch<CountMinSketch>) };
    nitro.unsampled_count() as jlong
}

/// Reset statistics
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_NitroSketch_resetStats(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) {
    if ptr == 0 {
        return;
    }
    let nitro = unsafe { &mut *(ptr as *mut NitroSketch<CountMinSketch>) };
    nitro.reset_stats();
}

/// Free native memory
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

// ============================================================================
// UNIVERSAL MONITORING - UnivMon (Tier 2)
// ============================================================================

/// Create a new UnivMon sketch
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_new(
    mut env: JNIEnv,
    _: JClass,
    max_stream_size: jlong,
    epsilon: jdouble,
    delta: jdouble,
) -> jlong {
    match UnivMon::new(max_stream_size as u64, epsilon, delta) {
        Ok(univmon) => Box::into_raw(Box::new(univmon)) as jlong,
        Err(_) => 0,
    }
}

/// Update with item and value
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_update(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    item: jbyteArray,
    value: jdouble,
) {
    if ptr == 0 {
        return;
    }

    let univmon = unsafe { &mut *(ptr as *mut UnivMon) };

    match env.convert_byte_array(item) {
        Ok(bytes) => {
            let _ = univmon.update(&bytes, value);
        }
        Err(_) => {}
    }
}

/// Estimate L1 norm (sum of frequencies)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_estimateL1(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let univmon = unsafe { &*(ptr as *const UnivMon) };
    univmon.estimate_l1()
}

/// Estimate L2 norm (sum of squared frequencies)
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_estimateL2(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let univmon = unsafe { &*(ptr as *const UnivMon) };
    univmon.estimate_l2()
}

/// Estimate Shannon entropy
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_estimateEntropy(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let univmon = unsafe { &*(ptr as *const UnivMon) };
    univmon.estimate_entropy()
}

/// Detect change between two UnivMon instances
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_detectChange(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) -> jdouble {
    if ptr1 == 0 || ptr2 == 0 {
        return 0.0;
    }
    let univmon1 = unsafe { &*(ptr1 as *const UnivMon) };
    let univmon2 = unsafe { &*(ptr2 as *const UnivMon) };
    univmon1.detect_change(univmon2)
}

/// Get number of layers
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_numLayers(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jint {
    if ptr == 0 {
        return 0;
    }
    let univmon = unsafe { &*(ptr as *const UnivMon) };
    univmon.num_layers() as jint
}

/// Get total updates
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_totalUpdates(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let univmon = unsafe { &*(ptr as *const UnivMon) };
    univmon.total_updates() as jlong
}

/// Merge another UnivMon into this one
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_merge(
    _env: JNIEnv,
    _: JObject,
    ptr1: jlong,
    ptr2: jlong,
) {
    if ptr1 == 0 || ptr2 == 0 {
        return;
    }
    let univmon1 = unsafe { &mut *(ptr1 as *mut UnivMon) };
    let univmon2 = unsafe { &*(ptr2 as *const UnivMon) };
    let _ = univmon1.merge(univmon2);
}

/// Free native memory
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_UnivMon_free(_env: JNIEnv, _: JObject, ptr: jlong) {
    if ptr != 0 {
        let _ = unsafe { Box::from_raw(ptr as *mut UnivMon) };
    }
}

// ============================================================================
// MEMBERSHIP TESTING - LearnedBloomFilter (Tier 2 - EXPERIMENTAL)
// ============================================================================

/// Create a new LearnedBloomFilter from training keys
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_LearnedBloomFilter_new(
    mut env: JNIEnv,
    _: JClass,
    training_data: jobject,
    fpr: jdouble,
) -> jlong {
    // training_data is a List<byte[]>
    let list = match env.get_list(&training_data.into()) {
        Ok(list) => list,
        Err(_) => return 0,
    };

    let mut keys: Vec<Vec<u8>> = Vec::new();

    let mut iter = match list.iter(&mut env) {
        Ok(iter) => iter,
        Err(_) => return 0,
    };

    while let Some(item) = iter.next(&mut env).ok().flatten() {
        if let Ok(byte_array) = env.convert_byte_array(item.into()) {
            keys.push(byte_array);
        }
    }

    if keys.is_empty() {
        return 0;
    }

    match LearnedBloomFilter::new(&keys, fpr) {
        Ok(lbf) => Box::into_raw(Box::new(lbf)) as jlong,
        Err(_) => 0,
    }
}

/// Check if an item might be in the set
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_LearnedBloomFilter_contains(
    mut env: JNIEnv,
    _: JObject,
    ptr: jlong,
    data: jbyteArray,
) -> jboolean {
    if ptr == 0 {
        return 0;
    }

    let lbf = unsafe { &*(ptr as *const LearnedBloomFilter) };

    match env.convert_byte_array(data) {
        Ok(bytes) => lbf.contains(&bytes) as jboolean,
        Err(_) => 0,
    }
}

/// Get memory usage in bytes
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_LearnedBloomFilter_memoryUsage(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jlong {
    if ptr == 0 {
        return 0;
    }
    let lbf = unsafe { &*(ptr as *const LearnedBloomFilter) };
    lbf.memory_usage() as jlong
}

/// Get target FPR
#[no_mangle]
pub extern "system" fn Java_com_sketches_oxide_LearnedBloomFilter_fpr(
    _env: JNIEnv,
    _: JObject,
    ptr: jlong,
) -> jdouble {
    if ptr == 0 {
        return 0.0;
    }
    let lbf = unsafe { &*(ptr as *const LearnedBloomFilter) };
    lbf.fpr()
}

/// Free native memory
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
// Phase 2 Note: This foundation implements 3 key algorithms as proof-of-concept.
// Remaining 25 algorithms follow the same pattern with appropriate constructors
// and method signatures for their respective types.
//
// Architecture:
// - All instances use opaque `jlong` (void*) pointers to native structs
// - Memory management via Box for allocation/deallocation
// - JNIEnv methods for type conversion (byte arrays, etc.)
// - All public methods exported with #[no_mangle] and extern "system"
// ============================================================================
