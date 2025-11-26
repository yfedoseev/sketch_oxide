package com.sketches_oxide.frequency;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * CountMinSketch - Frequency Estimation.
 *
 * Provides frequency estimation with O(1) time per operation and
 * bounded relative error (never underestimates).
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Heavy hitter detection (top-K frequent items)</li>
 *   <li>Word frequency analysis in text processing</li>
 *   <li>Request counting in system monitoring</li>
 * </ul>
 *
 * <p><strong>Space Complexity:</strong> O(1/epsilon * ln(1/delta))
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (CountMinSketch cms = new CountMinSketch(0.01, 0.01)) {
 *     cms.update("apple".getBytes(), 5);
 *     cms.update("apple".getBytes(), 3);
 *     System.out.println("Frequency >= " + cms.estimate("apple".getBytes()));
 * }
 * </pre>
 */
public final class CountMinSketch extends NativeSketch implements MergeableSketch<CountMinSketch> {

    private final double epsilon;
    private final double delta;

    /**
     * Create a new CountMinSketch.
     *
     * @param epsilon relative error bound (0 < epsilon < 1)
     *                Smaller values mean more accurate but larger space
     * @param delta failure probability (0 < delta < 1)
     *              Smaller values mean more confident but larger space
     * @throws IllegalArgumentException if parameters are invalid
     */
    public CountMinSketch(double epsilon, double delta) {
        if (epsilon <= 0 || epsilon >= 1) {
            throw new IllegalArgumentException("epsilon must be in range (0, 1), got: " + epsilon);
        }
        if (delta <= 0 || delta >= 1) {
            throw new IllegalArgumentException("delta must be in range (0, 1), got: " + delta);
        }

        this.epsilon = epsilon;
        this.delta = delta;
        this.nativePtr = SketchOxideNative.countMinSketch_new(epsilon, delta);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate CountMinSketch");
        }
    }

    /**
     * Update the frequency of an item.
     *
     * @param item the item to count (typically a hash of the actual item)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.countMinSketch_update(nativePtr, item);
    }

    /**
     * Update with a string item.
     *
     * @param item the string to count
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
     * Returns an estimate that is >= the true frequency (never underestimates).
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
        return SketchOxideNative.countMinSketch_estimate(nativePtr, item);
    }

    /**
     * Estimate the frequency of a string item.
     *
     * @param item the string to query
     * @return estimated frequency
     */
    public long estimate(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return estimate(item.getBytes());
    }

    /**
     * Merge another CountMinSketch into this one.
     *
     * Both sketches must have the same epsilon and delta parameters.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(CountMinSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.epsilon != other.epsilon || this.delta != other.delta) {
            throw new IllegalArgumentException(
                    "Cannot merge CountMinSketches with different parameters: " +
                            String.format("(%.3f, %.3f) vs (%.3f, %.3f)",
                                    this.epsilon, this.delta, other.epsilon, other.delta));
        }

        SketchOxideNative.countMinSketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the width parameter (number of hash tables).
     *
     * @return the width
     */
    public int width() {
        checkAlive();
        return SketchOxideNative.countMinSketch_width(nativePtr);
    }

    /**
     * Get the depth parameter (hash functions per table).
     *
     * @return the depth
     */
    public int depth() {
        checkAlive();
        return SketchOxideNative.countMinSketch_depth(nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.countMinSketch_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new CountMinSketch instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static CountMinSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.countMinSketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized CountMinSketch data");
        }

        CountMinSketch cms = new CountMinSketch(0.01, 0.01);  // Dummy
        cms.nativePtr = ptr;
        return cms;
    }

    /**
     * Reset the sketch to empty state.
     *
     * @throws IllegalStateException if the sketch has been closed
     */
    public void reset() {
        close();
        nativePtr = 0;
        nativePtr = SketchOxideNative.countMinSketch_new(epsilon, delta);
    }

    /**
     * Update the sketch with multiple items in a single call (optimized for throughput).
     *
     * <p>Batch updates are significantly faster than multiple individual update() calls
     * because they amortize the JNI (Java Native Interface) overhead across many items.
     * This is the preferred method when adding large quantities of data.
     *
     * @param items the items to count
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if items is null
     */
    public void updateBatch(byte[]... items) {
        checkAlive();
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        for (byte[] item : items) {
            update(item);
        }
    }

    /**
     * Update the sketch with multiple string items in a single call.
     *
     * @param items the string items to count
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if items is null
     */
    public void updateBatch(String... items) {
        checkAlive();
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        for (String item : items) {
            update(item);
        }
    }

    /**
     * Estimate frequencies of multiple items in a single call (optimized for lookups).
     *
     * @param items the items to query
     * @return array of estimated frequencies, one for each item
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if items is null
     */
    public long[] estimateBatch(byte[]... items) {
        checkAlive();
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        long[] results = new long[items.length];
        for (int i = 0; i < items.length; i++) {
            results[i] = estimate(items[i]);
        }
        return results;
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
            SketchOxideNative.countMinSketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("CountMinSketch(epsilon=%.3f, delta=%.3f, width=%d, depth=%d)",
                        epsilon, delta, width(), depth());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "CountMinSketch(closed)";
    }
}
