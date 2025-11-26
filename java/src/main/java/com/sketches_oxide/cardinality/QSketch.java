package com.sketches_oxide.cardinality;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * QSketch - Weighted Cardinality Estimation.
 *
 * <p>QSketch provides cardinality estimation with support for weighted items,
 * making it suitable for scenarios where items have varying importance or
 * occurrence weights. It uses a sampling-based approach with a configurable
 * maximum number of samples.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>QSketch maintains a weighted reservoir of samples, where each item can
 * contribute different weights to the cardinality estimate. This is particularly
 * useful for:
 * <ul>
 *   <li>Counting unique users weighted by their activity level</li>
 *   <li>Estimating cardinality where some items are more significant</li>
 *   <li>Scenarios requiring weighted distinct counting</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Weighted unique visitor counting in analytics</li>
 *   <li>Revenue-weighted customer cardinality</li>
 *   <li>Activity-weighted user counting</li>
 *   <li>Importance-sampled cardinality estimation</li>
 * </ul>
 *
 * <p><strong>Performance:</strong>
 * <ul>
 *   <li>Space: O(maxSamples) bytes</li>
 *   <li>Time: O(log(maxSamples)) per update (amortized)</li>
 *   <li>Accuracy: Improves with more samples</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (QSketch qs = new QSketch(10000)) {
 *     // Add items with default weight (1.0)
 *     qs.update("user1");
 *     qs.update("user2");
 *
 *     // Add items with custom weights
 *     qs.update("premium_user", 5.0);  // Weighted higher
 *     qs.update("vip_user", 10.0);     // Weighted even higher
 *
 *     System.out.println("Weighted cardinality: " + qs.estimate());
 * }
 * </pre>
 *
 * @see HyperLogLog
 * @see CpcSketch
 */
public final class QSketch extends NativeSketch implements MergeableSketch<QSketch> {

    /** Minimum allowed maxSamples value. */
    public static final long MIN_MAX_SAMPLES = 16;

    /** Maximum allowed maxSamples value. */
    public static final long MAX_MAX_SAMPLES = 1_000_000_000L;

    /** Default maxSamples value providing good balance. */
    public static final long DEFAULT_MAX_SAMPLES = 10000;

    /** The maxSamples parameter used to create this sketch. */
    private final long maxSamples;

    /**
     * Create a new QSketch with default maxSamples (10000).
     *
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public QSketch() {
        this(DEFAULT_MAX_SAMPLES);
    }

    /**
     * Create a new QSketch.
     *
     * @param maxSamples the maximum number of samples to retain (16 to 1,000,000,000)
     *                   Higher values provide better accuracy but use more memory.
     * @throws IllegalArgumentException if maxSamples is not in valid range
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public QSketch(long maxSamples) {
        if (maxSamples < MIN_MAX_SAMPLES || maxSamples > MAX_MAX_SAMPLES) {
            throw new IllegalArgumentException(
                    "maxSamples must be between " + MIN_MAX_SAMPLES + " and " + MAX_MAX_SAMPLES +
                            ", got: " + maxSamples);
        }

        this.maxSamples = maxSamples;
        this.nativePtr = SketchOxideNative.qsketch_new(maxSamples);
        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate QSketch");
        }
    }

    /**
     * Add an item to the sketch with default weight (1.0).
     *
     * @param item the byte array data to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.qsketch_update(nativePtr, item);
    }

    /**
     * Add a string item to the sketch with default weight (1.0).
     *
     * @param item the string to add
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
     * Add an item to the sketch with a custom weight.
     *
     * <p>Items with higher weights contribute more to the cardinality estimate.
     * This is useful for scenarios where some items are more significant than others.
     *
     * @param item the byte array data to add
     * @param weight the weight for this item (must be positive)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     * @throws IllegalArgumentException if weight is not positive
     */
    public void update(byte[] item, double weight) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        if (weight <= 0 || Double.isNaN(weight) || Double.isInfinite(weight)) {
            throw new IllegalArgumentException(
                    "weight must be a positive finite number, got: " + weight);
        }
        SketchOxideNative.qsketch_updateWeighted(nativePtr, item, weight);
    }

    /**
     * Add a string item to the sketch with a custom weight.
     *
     * @param item the string to add
     * @param weight the weight for this item (must be positive)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     * @throws IllegalArgumentException if weight is not positive
     */
    public void update(String item, double weight) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        update(item.getBytes(), weight);
    }

    /**
     * Get the estimated weighted cardinality.
     *
     * <p>Returns the estimated number of unique items, weighted by their
     * respective weights. For unweighted items (added with weight 1.0),
     * this is similar to standard cardinality estimation.
     *
     * @return estimated weighted cardinality as a double
     * @throws IllegalStateException if the sketch has been closed
     */
    public double estimate() {
        checkAlive();
        return SketchOxideNative.qsketch_estimate(nativePtr);
    }

    /**
     * Get the estimated cardinality as a long.
     *
     * @return estimated cardinality, rounded to nearest long
     * @throws IllegalStateException if the sketch has been closed
     */
    public long estimateLong() {
        return Math.round(estimate());
    }

    /**
     * Merge another QSketch into this one.
     *
     * <p>After merging, this sketch represents the combined weighted cardinality
     * of both sketches. Both sketches must have the same maxSamples value.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if maxSamples values don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(QSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.maxSamples != other.maxSamples) {
            throw new IllegalArgumentException(
                    "Cannot merge QSketches with different maxSamples: " +
                            this.maxSamples + " vs " + other.maxSamples);
        }

        SketchOxideNative.qsketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the maximum number of samples.
     *
     * @return the maxSamples parameter used at creation
     */
    public long maxSamples() {
        return maxSamples;
    }

    /**
     * Get the current number of samples stored.
     *
     * @return the number of samples currently retained
     * @throws IllegalStateException if the sketch has been closed
     */
    public long sampleCount() {
        checkAlive();
        return SketchOxideNative.qsketch_sampleCount(nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.qsketch_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new QSketch instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static QSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.qsketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized QSketch data");
        }

        // Create a dummy object and replace its native pointer
        QSketch qs = new QSketch(DEFAULT_MAX_SAMPLES);
        SketchOxideNative.qsketch_free(qs.nativePtr);
        qs.nativePtr = ptr;
        return qs;
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
            SketchOxideNative.qsketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("QSketch(maxSamples=%d, sampleCount=%d, estimate=%.2f)",
                        maxSamples, sampleCount(), estimate());
            }
        } catch (IllegalStateException e) {
            // Sketch is closed
        }
        return "QSketch(closed)";
    }
}
