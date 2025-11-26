package com.sketches_oxide.frequency;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * SpaceSaving - Heavy Hitter Detection Algorithm.
 *
 * <p>SpaceSaving is an efficient algorithm for finding heavy hitters (most frequent items)
 * in a data stream using limited memory. It guarantees finding all items with frequency
 * above a threshold while using space proportional to 1/epsilon.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>SpaceSaving maintains a fixed number of counters (approximately 1/epsilon):
 * <ol>
 *   <li>If an item is monitored, increment its counter</li>
 *   <li>If not monitored and space available, add it with count 1</li>
 *   <li>If not monitored and no space, replace the minimum counter item,
 *       inheriting its count + 1</li>
 * </ol>
 *
 * <p><strong>Guarantees:</strong>
 * <ul>
 *   <li>All items with frequency > epsilon * N are guaranteed to be found</li>
 *   <li>Maximum overestimation is bounded by epsilon * N</li>
 *   <li>No false negatives for heavy hitters</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Finding top-K most frequent items</li>
 *   <li>Detecting popular URLs in web logs</li>
 *   <li>Identifying trending hashtags</li>
 *   <li>Network traffic heavy hitter detection</li>
 *   <li>Database hot key identification</li>
 * </ul>
 *
 * <p><strong>Space Complexity:</strong> O(1/epsilon)
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (SpaceSaving ss = new SpaceSaving(0.001)) {  // Find items > 0.1% of stream
 *     for (String item : stream) {
 *         ss.update(item);
 *     }
 *
 *     // Get top 10 frequent items
 *     byte[][] topItems = ss.topK(10);
 *     for (byte[] item : topItems) {
 *         String key = new String(item);
 *         System.out.println(key + ": ~" + ss.estimate(key));
 *     }
 * }
 * </pre>
 *
 * @see FrequentItems
 * @see CountMinSketch
 */
public final class SpaceSaving extends NativeSketch implements MergeableSketch<SpaceSaving> {

    private final double epsilon;

    /**
     * Create a new SpaceSaving sketch.
     *
     * @param epsilon error bound (0 < epsilon < 1)
     *                Determines the threshold for guaranteed heavy hitter detection.
     *                Items with frequency > epsilon * N are guaranteed to be found.
     *                Smaller values mean more accurate but larger space.
     * @throws IllegalArgumentException if epsilon is invalid
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public SpaceSaving(double epsilon) {
        if (epsilon <= 0 || epsilon >= 1) {
            throw new IllegalArgumentException("epsilon must be in range (0, 1), got: " + epsilon);
        }

        this.epsilon = epsilon;
        this.nativePtr = SketchOxideNative.spacesaving_new(epsilon);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate SpaceSaving");
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
        SketchOxideNative.spacesaving_update(nativePtr, item);
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
     * <p>Returns an estimate that may slightly overestimate the true count.
     * The maximum overestimation is bounded by epsilon * N.
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
        return SketchOxideNative.spacesaving_estimate(nativePtr, item);
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
     * Get the top-K most frequent items.
     *
     * <p>Returns the K items with the highest estimated frequencies.
     * The returned items are sorted by frequency in descending order.
     *
     * @param k the number of top items to return
     * @return array of the top-K items as byte arrays
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if k is not positive
     */
    public byte[][] topK(int k) {
        checkAlive();
        if (k <= 0) {
            throw new IllegalArgumentException("k must be positive, got: " + k);
        }
        return SketchOxideNative.spacesaving_topK(nativePtr, k);
    }

    /**
     * Get the top-K most frequent items as strings.
     *
     * @param k the number of top items to return
     * @return array of the top-K items as strings
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if k is not positive
     */
    public String[] topKStrings(int k) {
        byte[][] items = topK(k);
        String[] result = new String[items.length];
        for (int i = 0; i < items.length; i++) {
            result[i] = new String(items[i]);
        }
        return result;
    }

    /**
     * Merge another SpaceSaving into this one.
     *
     * <p>Both sketches must have the same epsilon parameter.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if epsilon values don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(SpaceSaving other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.epsilon != other.epsilon) {
            throw new IllegalArgumentException(
                    "Cannot merge SpaceSaving sketches with different epsilon: " +
                            this.epsilon + " vs " + other.epsilon);
        }

        SketchOxideNative.spacesaving_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the capacity (maximum number of monitored items).
     *
     * @return the capacity (approximately 1/epsilon)
     * @throws IllegalStateException if the sketch has been closed
     */
    public int capacity() {
        checkAlive();
        return SketchOxideNative.spacesaving_capacity(nativePtr);
    }

    /**
     * Get the current number of monitored items.
     *
     * @return the number of items currently being tracked
     * @throws IllegalStateException if the sketch has been closed
     */
    public int size() {
        checkAlive();
        return SketchOxideNative.spacesaving_size(nativePtr);
    }

    /**
     * Get the epsilon parameter.
     *
     * @return the epsilon (error bound)
     */
    public double epsilon() {
        return epsilon;
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.spacesaving_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new SpaceSaving instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static SpaceSaving deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.spacesaving_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized SpaceSaving data");
        }

        SpaceSaving ss = new SpaceSaving(0.01);
        SketchOxideNative.spacesaving_free(ss.nativePtr);
        ss.nativePtr = ptr;
        return ss;
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
            SketchOxideNative.spacesaving_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("SpaceSaving(epsilon=%.4f, capacity=%d, size=%d)",
                        epsilon, capacity(), size());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "SpaceSaving(closed)";
    }
}
