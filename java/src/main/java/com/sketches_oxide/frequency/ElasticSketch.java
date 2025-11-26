package com.sketches_oxide.frequency;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * ElasticSketch - Adaptive Bucket Allocation for Network Monitoring.
 *
 * <p>ElasticSketch is a frequency estimation data structure optimized for network
 * traffic monitoring. It uses adaptive bucket allocation to efficiently handle
 * both heavy hitters and light flows in the same structure.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>ElasticSketch combines two components:
 * <ol>
 *   <li><strong>Heavy Part</strong>: A hash table for tracking heavy hitters
 *       with exact counts when possible</li>
 *   <li><strong>Light Part</strong>: A Count-Min Sketch variant for tracking
 *       light flows</li>
 * </ol>
 *
 * <p>Items are dynamically moved between parts based on their frequency,
 * providing accuracy for both common and rare items.
 *
 * <p><strong>Key Features:</strong>
 * <ul>
 *   <li>Adaptive allocation between heavy and light flows</li>
 *   <li>High accuracy for heavy hitters</li>
 *   <li>Memory-efficient for light flows</li>
 *   <li>Optimized for network traffic patterns</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Network traffic flow monitoring</li>
 *   <li>DDoS attack detection</li>
 *   <li>Load balancing decisions</li>
 *   <li>Network anomaly detection</li>
 *   <li>Per-flow statistics collection</li>
 * </ul>
 *
 * <p><strong>Space Complexity:</strong> O(bucketCount * depth)
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (ElasticSketch es = new ElasticSketch(4096, 4)) {
 *     // Process network packets
 *     for (byte[] flowKey : packets) {
 *         es.update(flowKey);
 *     }
 *
 *     // Query flow frequency
 *     long count = es.estimate(flowKey);
 *     System.out.println("Flow packets: " + count);
 * }
 * </pre>
 *
 * @see <a href="https://dl.acm.org/doi/10.1145/3230543.3230544">ElasticSketch Paper</a>
 * @see CountMinSketch
 * @see SpaceSaving
 */
public final class ElasticSketch extends NativeSketch implements MergeableSketch<ElasticSketch> {

    /** Minimum allowed bucket count. */
    public static final long MIN_BUCKET_COUNT = 1;

    /** Maximum allowed bucket count. */
    public static final long MAX_BUCKET_COUNT = 1_000_000_000L;

    /** Minimum allowed depth. */
    public static final long MIN_DEPTH = 1;

    /** Maximum allowed depth. */
    public static final long MAX_DEPTH = 32;

    /** Default bucket count. */
    public static final long DEFAULT_BUCKET_COUNT = 4096;

    /** Default depth. */
    public static final long DEFAULT_DEPTH = 4;

    private final long bucketCountParam;
    private final long depthParam;

    /**
     * Create a new ElasticSketch with default parameters.
     *
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public ElasticSketch() {
        this(DEFAULT_BUCKET_COUNT, DEFAULT_DEPTH);
    }

    /**
     * Create a new ElasticSketch.
     *
     * @param bucketCount number of buckets in the heavy part (1 to 1,000,000,000)
     *                    Higher values provide better accuracy but use more memory.
     * @param depth number of hash functions/rows (1 to 32)
     *              Higher values reduce collision errors.
     * @throws IllegalArgumentException if parameters are invalid
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public ElasticSketch(long bucketCount, long depth) {
        if (bucketCount < MIN_BUCKET_COUNT || bucketCount > MAX_BUCKET_COUNT) {
            throw new IllegalArgumentException(
                    "bucketCount must be between " + MIN_BUCKET_COUNT + " and " + MAX_BUCKET_COUNT +
                            ", got: " + bucketCount);
        }
        if (depth < MIN_DEPTH || depth > MAX_DEPTH) {
            throw new IllegalArgumentException(
                    "depth must be between " + MIN_DEPTH + " and " + MAX_DEPTH +
                            ", got: " + depth);
        }

        this.bucketCountParam = bucketCount;
        this.depthParam = depth;
        this.nativePtr = SketchOxideNative.elasticsketch_new(bucketCount, depth);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate ElasticSketch");
        }
    }

    /**
     * Update the frequency of an item (increment by 1).
     *
     * @param item the item to count (typically a flow key)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.elasticsketch_update(nativePtr, item);
    }

    /**
     * Update with a string item.
     *
     * @param item the string to count
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        update(item.getBytes());
    }

    /**
     * Estimate the frequency of an item.
     *
     * <p>Returns an estimate that is accurate for heavy hitters and
     * bounded for light flows.
     *
     * @param item the item to query
     * @return estimated frequency
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long estimate(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.elasticsketch_estimate(nativePtr, item);
    }

    /**
     * Estimate the frequency of a string item.
     *
     * @param item the string to query
     * @return estimated frequency
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long estimate(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return estimate(item.getBytes());
    }

    /**
     * Merge another ElasticSketch into this one.
     *
     * <p>Both sketches must have the same bucketCount and depth parameters.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(ElasticSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.bucketCountParam != other.bucketCountParam || this.depthParam != other.depthParam) {
            throw new IllegalArgumentException(
                    "Cannot merge ElasticSketches with different parameters: " +
                            String.format("(%d, %d) vs (%d, %d)",
                                    this.bucketCountParam, this.depthParam,
                                    other.bucketCountParam, other.depthParam));
        }

        SketchOxideNative.elasticsketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the bucket count parameter.
     *
     * @return the number of buckets in the heavy part
     * @throws IllegalStateException if the sketch has been closed
     */
    public long bucketCount() {
        checkAlive();
        return SketchOxideNative.elasticsketch_bucketCount(nativePtr);
    }

    /**
     * Get the depth parameter.
     *
     * @return the number of hash functions/rows
     * @throws IllegalStateException if the sketch has been closed
     */
    public long depth() {
        checkAlive();
        return SketchOxideNative.elasticsketch_depth(nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.elasticsketch_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new ElasticSketch instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static ElasticSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.elasticsketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized ElasticSketch data");
        }

        ElasticSketch es = new ElasticSketch(DEFAULT_BUCKET_COUNT, DEFAULT_DEPTH);
        SketchOxideNative.elasticsketch_free(es.nativePtr);
        es.nativePtr = ptr;
        return es;
    }

    @Override
    public void close() {
        if (nativePtr != 0) {
            freeNative();
            nativePtr = 0;
        }
    }

    @Override
    protected void freeNative() {
        if (nativePtr != 0) {
            SketchOxideNative.elasticsketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("ElasticSketch(bucketCount=%d, depth=%d)",
                        bucketCount(), depth());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "ElasticSketch(closed)";
    }
}
