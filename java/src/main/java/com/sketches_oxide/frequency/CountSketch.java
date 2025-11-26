package com.sketches_oxide.frequency;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * CountSketch - Unbiased Frequency Estimation.
 *
 * <p>CountSketch provides unbiased frequency estimation with bounded error.
 * Unlike Count-Min Sketch which only overestimates, CountSketch can both
 * over- and under-estimate but provides an unbiased estimate on average.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>CountSketch uses multiple hash functions with random sign flips (+1/-1).
 * Each item is hashed to multiple positions, and a random sign is applied
 * before incrementing. The estimate is the median of signed counts, which
 * cancels out collisions in expectation.
 *
 * <p><strong>Key Differences from Count-Min Sketch:</strong>
 * <ul>
 *   <li>Unbiased estimates (can under-estimate or over-estimate)</li>
 *   <li>Uses median instead of minimum for estimation</li>
 *   <li>Better for scenarios where unbiased estimates are preferred</li>
 *   <li>Supports negative frequency updates</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Heavy hitter detection with unbiased estimates</li>
 *   <li>Network traffic monitoring</li>
 *   <li>Natural language processing word frequencies</li>
 *   <li>Scenarios requiring unbiased frequency estimation</li>
 * </ul>
 *
 * <p><strong>Space Complexity:</strong> O(1/epsilon^2 * ln(1/delta))
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (CountSketch cs = new CountSketch(0.01, 0.01)) {
 *     cs.update("apple");
 *     cs.update("apple");
 *     cs.update("banana");
 *
 *     // Estimate is unbiased (may be slightly above or below true count)
 *     System.out.println("Estimated apple count: " + cs.estimate("apple"));
 * }
 * </pre>
 *
 * @see CountMinSketch
 */
public final class CountSketch extends NativeSketch implements MergeableSketch<CountSketch> {

    private final double epsilon;
    private final double delta;

    /**
     * Create a new CountSketch.
     *
     * @param epsilon relative error bound (0 < epsilon < 1)
     *                Smaller values mean more accurate but larger space
     * @param delta failure probability (0 < delta < 1)
     *              Smaller values mean more confident but larger space
     * @throws IllegalArgumentException if parameters are invalid
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public CountSketch(double epsilon, double delta) {
        if (epsilon <= 0 || epsilon >= 1) {
            throw new IllegalArgumentException("epsilon must be in range (0, 1), got: " + epsilon);
        }
        if (delta <= 0 || delta >= 1) {
            throw new IllegalArgumentException("delta must be in range (0, 1), got: " + delta);
        }

        this.epsilon = epsilon;
        this.delta = delta;
        this.nativePtr = SketchOxideNative.countsketch_new(epsilon, delta);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate CountSketch");
        }
    }

    /**
     * Update the frequency of an item (increment by 1).
     *
     * @param item the item to count
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.countsketch_update(nativePtr, item);
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
     * <p>Returns an unbiased estimate that may be above or below the true count.
     * The estimate is the median of signed counts across all hash functions.
     *
     * @param item the item to query
     * @return estimated frequency (may be negative due to hash collisions)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long estimate(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.countsketch_estimate(nativePtr, item);
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
     * Merge another CountSketch into this one.
     *
     * <p>Both sketches must have the same epsilon and delta parameters.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(CountSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.epsilon != other.epsilon || this.delta != other.delta) {
            throw new IllegalArgumentException(
                    "Cannot merge CountSketches with different parameters: " +
                            String.format("(%.3f, %.3f) vs (%.3f, %.3f)",
                                    this.epsilon, this.delta, other.epsilon, other.delta));
        }

        SketchOxideNative.countsketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the width parameter (number of counters per row).
     *
     * @return the width
     * @throws IllegalStateException if the sketch has been closed
     */
    public int width() {
        checkAlive();
        return SketchOxideNative.countsketch_width(nativePtr);
    }

    /**
     * Get the depth parameter (number of hash functions/rows).
     *
     * @return the depth
     * @throws IllegalStateException if the sketch has been closed
     */
    public int depth() {
        checkAlive();
        return SketchOxideNative.countsketch_depth(nativePtr);
    }

    /**
     * Get the epsilon parameter.
     *
     * @return the epsilon (relative error bound)
     */
    public double epsilon() {
        return epsilon;
    }

    /**
     * Get the delta parameter.
     *
     * @return the delta (failure probability)
     */
    public double delta() {
        return delta;
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.countsketch_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new CountSketch instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static CountSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.countsketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized CountSketch data");
        }

        CountSketch cs = new CountSketch(0.01, 0.01);
        SketchOxideNative.countsketch_free(cs.nativePtr);
        cs.nativePtr = ptr;
        return cs;
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
            SketchOxideNative.countsketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("CountSketch(epsilon=%.3f, delta=%.3f, width=%d, depth=%d)",
                        epsilon, delta, width(), depth());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "CountSketch(closed)";
    }
}
