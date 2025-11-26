package com.sketches_oxide.native;

/**
 * Native JNI bindings for sketch_oxide library.
 *
 * This class provides low-level access to native Rust implementations
 * via JNI. These methods should not be called directly - use the
 * high-level wrapper classes instead.
 */
public class SketchOxideNative {
    // Static initializer to load the native library
    static {
        NativeLibraryLoader.load();
    }

    // ============================================================================
    // CARDINALITY ESTIMATION - HyperLogLog
    // ============================================================================

    public static native long hyperLogLog_new(int precision);

    public static native void hyperLogLog_update(long ptr, byte[] data);

    public static native void hyperLogLog_updateDirect(long ptr, long address, long length);

    public static native double hyperLogLog_estimate(long ptr);

    public static native void hyperLogLog_merge(long ptr1, long ptr2);

    public static native int hyperLogLog_precision(long ptr);

    public static native byte[] hyperLogLog_serialize(long ptr);

    public static native long hyperLogLog_deserialize(byte[] data);

    public static native void hyperLogLog_free(long ptr);

    // ============================================================================
    // CARDINALITY ESTIMATION - UltraLogLog
    // ============================================================================

    public static native long ultraLogLog_new(int precision);

    public static native void ultraLogLog_update(long ptr, byte[] data);

    public static native void ultraLogLog_updateDirect(long ptr, long address, long length);

    public static native double ultraLogLog_estimate(long ptr);

    public static native void ultraLogLog_merge(long ptr1, long ptr2);

    public static native int ultraLogLog_precision(long ptr);

    public static native byte[] ultraLogLog_serialize(long ptr);

    public static native long ultraLogLog_deserialize(byte[] data);

    public static native void ultraLogLog_free(long ptr);

    // ============================================================================
    // FREQUENCY ESTIMATION - CountMinSketch
    // ============================================================================

    public static native long countMinSketch_new(double epsilon, double delta);

    public static native void countMinSketch_update(long ptr, byte[] data);

    public static native long countMinSketch_estimate(long ptr, byte[] data);

    public static native void countMinSketch_merge(long ptr1, long ptr2);

    public static native int countMinSketch_width(long ptr);

    public static native int countMinSketch_depth(long ptr);

    public static native byte[] countMinSketch_serialize(long ptr);

    public static native long countMinSketch_deserialize(byte[] data);

    public static native void countMinSketch_free(long ptr);

    // ============================================================================
    // MEMBERSHIP TESTING - BloomFilter
    // ============================================================================

    public static native long bloomFilter_new(long n, double fpr);

    public static native void bloomFilter_insert(long ptr, byte[] data);

    public static native boolean bloomFilter_contains(long ptr, byte[] data);

    public static native void bloomFilter_merge(long ptr1, long ptr2);

    public static native byte[] bloomFilter_serialize(long ptr);

    public static native long bloomFilter_deserialize(byte[] data);

    public static native void bloomFilter_free(long ptr);

    // ============================================================================
    // QUANTILES - DDSketch
    // ============================================================================

    public static native long ddsketch_new(double relativeAccuracy);

    public static native void ddsketch_update(long ptr, double value);

    public static native double ddsketch_quantile(long ptr, double q);

    public static native void ddsketch_merge(long ptr1, long ptr2);

    public static native byte[] ddsketch_serialize(long ptr);

    public static native long ddsketch_deserialize(byte[] data);

    public static native void ddsketch_free(long ptr);

    // ============================================================================
    // Placeholder stubs for remaining algorithms
    // Implementation follows the same pattern:
    // 1. _new(...) constructor with appropriate parameters
    // 2. _update(...) for adding items
    // 3. _estimate() or query method for results
    // 4. _merge() for combining sketches
    // 5. _serialize() / _deserialize() for persistence
    // 6. _free() for cleanup
    //
    // Categories to implement:
    // - CpcSketch, QSketch, ThetaSketch (cardinality)
    // - CountSketch, ConservativeCountMin, SpaceSaving, FrequentItems,
    //   ElasticSketch, SALSA, RemovableUniversalSketch (frequency)
    // - BinaryFuseFilter, BlockedBloomFilter, CountingBloomFilter,
    //   CuckooFilter, RibbonFilter, StableBloomFilter (membership)
    // - ReqSketch, TDigest, KllSketch, SplineSketch (quantiles)
    // - SlidingWindowCounter, ExponentialHistogram (streaming)
    // - MinHash, SimHash (similarity)
    // - ReservoirSampling, VarOptSampling (sampling)
    // ============================================================================

    // CARDINALITY - CpcSketch
    public static native long cpcSketch_new(int lgK);

    public static native void cpcSketch_update(long ptr, byte[] data);

    public static native double cpcSketch_estimate(long ptr);

    public static native void cpcSketch_merge(long ptr1, long ptr2);

    public static native byte[] cpcSketch_serialize(long ptr);

    public static native long cpcSketch_deserialize(byte[] data);

    public static native void cpcSketch_free(long ptr);

    // CARDINALITY - QSketch
    public static native long qsketch_new(long maxSamples);

    public static native void qsketch_update(long ptr, byte[] data);

    public static native void qsketch_updateWeighted(long ptr, byte[] data, double weight);

    public static native double qsketch_estimate(long ptr);

    public static native long qsketch_maxSamples(long ptr);

    public static native long qsketch_sampleCount(long ptr);

    public static native void qsketch_merge(long ptr1, long ptr2);

    public static native byte[] qsketch_serialize(long ptr);

    public static native long qsketch_deserialize(byte[] data);

    public static native void qsketch_free(long ptr);

    // CARDINALITY - ThetaSketch
    public static native long thetasketch_new(int lgK);

    public static native void thetasketch_update(long ptr, byte[] data);

    public static native double thetasketch_estimate(long ptr);

    public static native double thetasketch_theta(long ptr);

    public static native long thetasketch_retainedHashCount(long ptr);

    public static native int thetasketch_lgK(long ptr);

    public static native long thetasketch_intersect(long ptr1, long ptr2);

    public static native long thetasketch_aNotB(long ptr1, long ptr2);

    public static native double thetasketch_jaccardSimilarity(long ptr1, long ptr2);

    public static native void thetasketch_merge(long ptr1, long ptr2);

    public static native byte[] thetasketch_serialize(long ptr);

    public static native long thetasketch_deserialize(byte[] data);

    public static native void thetasketch_free(long ptr);

    // FREQUENCY - CountSketch
    public static native long countsketch_new(double epsilon, double delta);

    public static native void countsketch_update(long ptr, byte[] data);

    public static native long countsketch_estimate(long ptr, byte[] data);

    public static native int countsketch_width(long ptr);

    public static native int countsketch_depth(long ptr);

    public static native void countsketch_merge(long ptr1, long ptr2);

    public static native byte[] countsketch_serialize(long ptr);

    public static native long countsketch_deserialize(byte[] data);

    public static native void countsketch_free(long ptr);

    // FREQUENCY - ConservativeCountMin
    public static native long conservativecountmin_new(double epsilon, double delta);

    public static native void conservativecountmin_update(long ptr, byte[] data);

    public static native long conservativecountmin_estimate(long ptr, byte[] data);

    public static native int conservativecountmin_width(long ptr);

    public static native int conservativecountmin_depth(long ptr);

    public static native long conservativecountmin_totalCount(long ptr);

    public static native void conservativecountmin_merge(long ptr1, long ptr2);

    public static native byte[] conservativecountmin_serialize(long ptr);

    public static native long conservativecountmin_deserialize(byte[] data);

    public static native void conservativecountmin_free(long ptr);

    // FREQUENCY - SpaceSaving
    public static native long spacesaving_new(double epsilon);

    public static native void spacesaving_update(long ptr, byte[] data);

    public static native long spacesaving_estimate(long ptr, byte[] data);

    public static native byte[][] spacesaving_topK(long ptr, int k);

    public static native int spacesaving_capacity(long ptr);

    public static native int spacesaving_size(long ptr);

    public static native void spacesaving_merge(long ptr1, long ptr2);

    public static native byte[] spacesaving_serialize(long ptr);

    public static native long spacesaving_deserialize(byte[] data);

    public static native void spacesaving_free(long ptr);

    // FREQUENCY - FrequentItems
    public static native long frequentitems_new(long maxSize);

    public static native void frequentitems_update(long ptr, byte[] data);

    public static native long frequentitems_estimate(long ptr, byte[] data);

    public static native byte[][] frequentitems_getFrequentItems(long ptr, long threshold);

    public static native long frequentitems_maxSize(long ptr);

    public static native long frequentitems_currentSize(long ptr);

    public static native long frequentitems_lowerBound(long ptr, byte[] data);

    public static native long frequentitems_upperBound(long ptr, byte[] data);

    public static native void frequentitems_merge(long ptr1, long ptr2);

    public static native byte[] frequentitems_serialize(long ptr);

    public static native long frequentitems_deserialize(byte[] data);

    public static native void frequentitems_free(long ptr);

    // FREQUENCY - ElasticSketch
    public static native long elasticsketch_new(long bucketCount, long depth);

    public static native void elasticsketch_update(long ptr, byte[] data);

    public static native long elasticsketch_estimate(long ptr, byte[] data);

    public static native long elasticsketch_bucketCount(long ptr);

    public static native long elasticsketch_depth(long ptr);

    public static native void elasticsketch_merge(long ptr1, long ptr2);

    public static native byte[] elasticsketch_serialize(long ptr);

    public static native long elasticsketch_deserialize(byte[] data);

    public static native void elasticsketch_free(long ptr);

    // FREQUENCY - SALSA
    public static native long salsa_new(double epsilon, double delta);

    public static native void salsa_update(long ptr, byte[] data);

    public static native long salsa_estimate(long ptr, byte[] data);

    public static native double salsa_confidenceMetric(long ptr);

    public static native int salsa_adaptationLevel(long ptr);

    public static native void salsa_merge(long ptr1, long ptr2);

    public static native byte[] salsa_serialize(long ptr);

    public static native long salsa_deserialize(byte[] data);

    public static native void salsa_free(long ptr);

    // FREQUENCY - RemovableUniversalSketch
    public static native long removableuniversalsketch_new(double epsilon, double delta);

    public static native void removableuniversalsketch_update(long ptr, byte[] data);

    public static native void removableuniversalsketch_updateWithDelta(long ptr, byte[] data, long delta);

    public static native long removableuniversalsketch_estimate(long ptr, byte[] data);

    public static native double removableuniversalsketch_l2Norm(long ptr);

    public static native boolean removableuniversalsketch_hasL2Norm(long ptr);

    public static native void removableuniversalsketch_merge(long ptr1, long ptr2);

    public static native byte[] removableuniversalsketch_serialize(long ptr);

    public static native long removableuniversalsketch_deserialize(byte[] data);

    public static native void removableuniversalsketch_free(long ptr);

    // MEMBERSHIP - BinaryFuseFilter
    public static native long binaryfusefilter_new(long[] items, int bitsPerEntry);

    public static native boolean binaryfusefilter_contains(long ptr, long item);

    public static native byte[] binaryfusefilter_serialize(long ptr);

    public static native long binaryfusefilter_deserialize(byte[] data);

    public static native void binaryfusefilter_free(long ptr);

    // MEMBERSHIP - BlockedBloomFilter
    public static native long blockedbloomfilter_new(long n, double fpr);

    public static native void blockedbloomfilter_insert(long ptr, byte[] data);

    public static native boolean blockedbloomfilter_contains(long ptr, byte[] data);

    public static native void blockedbloomfilter_merge(long ptr1, long ptr2);

    public static native byte[] blockedbloomfilter_serialize(long ptr);

    public static native long blockedbloomfilter_deserialize(byte[] data);

    public static native void blockedbloomfilter_free(long ptr);

    // MEMBERSHIP - CountingBloomFilter
    public static native long countingbloomfilter_new(long n, double fpr);

    public static native void countingbloomfilter_insert(long ptr, byte[] data);

    public static native void countingbloomfilter_remove(long ptr, byte[] data);

    public static native boolean countingbloomfilter_contains(long ptr, byte[] data);

    public static native byte[] countingbloomfilter_serialize(long ptr);

    public static native long countingbloomfilter_deserialize(byte[] data);

    public static native void countingbloomfilter_free(long ptr);

    // MEMBERSHIP - CuckooFilter
    public static native long cuckoofilter_new(long capacity);

    public static native void cuckoofilter_insert(long ptr, byte[] data);

    public static native boolean cuckoofilter_contains(long ptr, byte[] data);

    public static native void cuckoofilter_remove(long ptr, byte[] data);

    public static native byte[] cuckoofilter_serialize(long ptr);

    public static native long cuckoofilter_deserialize(byte[] data);

    public static native void cuckoofilter_free(long ptr);

    // MEMBERSHIP - RibbonFilter
    public static native long ribbonfilter_new(long n, double fpr);

    public static native void ribbonfilter_insert(long ptr, byte[] data);

    public static native void ribbonfilter_build(long ptr);

    public static native boolean ribbonfilter_contains(long ptr, byte[] data);

    public static native byte[] ribbonfilter_serialize(long ptr);

    public static native long ribbonfilter_deserialize(byte[] data);

    public static native void ribbonfilter_free(long ptr);

    // MEMBERSHIP - StableBloomFilter
    public static native long stablebloomfilter_new(long maxBytes, long logSizeOfArray, double fpr);

    public static native void stablebloomfilter_insert(long ptr, byte[] data);

    public static native boolean stablebloomfilter_contains(long ptr, byte[] data);

    public static native void stablebloomfilter_merge(long ptr1, long ptr2);

    public static native void stablebloomfilter_free(long ptr);

    // QUANTILES - ReqSketch
    public static native long reqsketch_new(int k, int mode);

    public static native void reqsketch_update(long ptr, double value);

    public static native double reqsketch_quantile(long ptr, double q);

    public static native void reqsketch_merge(long ptr1, long ptr2);

    public static native byte[] reqsketch_serialize(long ptr);

    public static native long reqsketch_deserialize(byte[] data);

    public static native void reqsketch_free(long ptr);

    // QUANTILES - TDigest
    public static native long tdigest_new(double compression);

    public static native void tdigest_update(long ptr, double value);

    public static native double tdigest_quantile(long ptr, double q);

    public static native void tdigest_merge(long ptr1, long ptr2);

    public static native byte[] tdigest_serialize(long ptr);

    public static native long tdigest_deserialize(byte[] data);

    public static native void tdigest_free(long ptr);

    // QUANTILES - KllSketch
    public static native long kllsketch_new(int k);

    public static native void kllsketch_update(long ptr, double value);

    public static native double kllsketch_quantile(long ptr, double q);

    public static native void kllsketch_merge(long ptr1, long ptr2);

    public static native byte[] kllsketch_serialize(long ptr);

    public static native long kllsketch_deserialize(byte[] data);

    public static native void kllsketch_free(long ptr);

    // QUANTILES - SplineSketch
    public static native long splinesketch_new(long maxBuckets);

    public static native void splinesketch_update(long ptr, double value);

    public static native double splinesketch_query(long ptr, double q);

    public static native void splinesketch_merge(long ptr1, long ptr2);

    public static native byte[] splinesketch_serialize(long ptr);

    public static native long splinesketch_deserialize(byte[] data);

    public static native void splinesketch_free(long ptr);

    // STREAMING - SlidingWindowCounter
    public static native long slidingwindowcounter_new(long windowSize, double epsilon);

    public static native void slidingwindowcounter_increment(long ptr);

    public static native long slidingwindowcounter_count(long ptr);

    public static native void slidingwindowcounter_expire(long ptr, long time);

    public static native byte[] slidingwindowcounter_serialize(long ptr);

    public static native long slidingwindowcounter_deserialize(byte[] data);

    public static native void slidingwindowcounter_free(long ptr);

    // STREAMING - ExponentialHistogram
    public static native long exponentialhistogram_new(long windowSize, double epsilon);

    public static native void exponentialhistogram_insert(long ptr);

    public static native long exponentialhistogram_count(long ptr);

    public static native void exponentialhistogram_expire(long ptr, long time);

    public static native byte[] exponentialhistogram_serialize(long ptr);

    public static native long exponentialhistogram_deserialize(byte[] data);

    public static native void exponentialhistogram_free(long ptr);

    // SIMILARITY - MinHash
    public static native long minhash_new(long numPerm);

    public static native void minhash_update(long ptr, byte[] data);

    public static native double minhash_jaccardsimilarity(long ptr1, long ptr2);

    public static native void minhash_merge(long ptr1, long ptr2);

    public static native byte[] minhash_serialize(long ptr);

    public static native long minhash_deserialize(byte[] data);

    public static native void minhash_free(long ptr);

    // SIMILARITY - SimHash
    public static native long simhash_new();

    public static native void simhash_update(long ptr, byte[] data);

    public static native long simhash_fingerprint(long ptr);

    public static native int simhash_hammingdistance(long ptr1, long ptr2);

    public static native double simhash_similarity(long ptr1, long ptr2);

    public static native byte[] simhash_serialize(long ptr);

    public static native long simhash_deserialize(byte[] data);

    public static native void simhash_free(long ptr);

    // SAMPLING - ReservoirSampling
    public static native long reservoirsampling_new(long k);

    public static native void reservoirsampling_update(long ptr, byte[] data);

    public static native byte[][] reservoirsampling_sample(long ptr);

    public static native long reservoirsampling_count(long ptr);

    public static native byte[] reservoirsampling_serialize(long ptr);

    public static native long reservoirsampling_deserialize(byte[] data);

    public static native void reservoirsampling_free(long ptr);

    // SAMPLING - VarOptSampling
    public static native long varoptsampling_new(long k);

    public static native void varoptsampling_update(long ptr, byte[] data);

    public static native byte[][] varoptsampling_sample(long ptr);

    public static native long varoptsampling_totalweight(long ptr);

    public static native byte[] varoptsampling_serialize(long ptr);

    public static native long varoptsampling_deserialize(byte[] data);

    public static native void varoptsampling_free(long ptr);
}
