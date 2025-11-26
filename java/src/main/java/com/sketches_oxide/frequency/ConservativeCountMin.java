package com.sketches_oxide.frequency;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * ConservativeCountMin - Conservative Update Count-Min Sketch.
 *
 * <p>ConservativeCountMin is an improved variant of Count-Min Sketch that uses
 * conservative updates to reduce overestimation. It only increments counters
 * that would affect the minimum, resulting in more accurate frequency estimates.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>Instead of incrementing all hash positions like standard Count-Min Sketch,
 * conservative update:
 * <ol>
 *   <li>Finds the minimum count across all hash positions</li>
 *   <li>Only increments positions that equal the minimum</li>
 *   <li>This reduces unnecessary counter inflation from collisions</li>
 * </ol>
 *
 * <p><strong>Advantages over Standard Count-Min:</strong>
 * <ul>
 *   <li>Significantly reduced overestimation</li>
 *   <li>Better accuracy for low-frequency items</li>
 *   <li>Same space and query complexity</li>
 *   <li>Still never underestimates</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Heavy hitter detection with better accuracy</li>
 *   <li>Network flow monitoring</li>
 *   <li>Database query frequency analysis</li>
 *   <li>Any Count-Min use case where accuracy is important</li>
 * </ul>
 *
 * <p><strong>Space Complexity:</strong> O(1/epsilon * ln(1/delta))
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (ConservativeCountMin ccm = new ConservativeCountMin(0.01, 0.01)) {
 *     ccm.update("apple");
 *     ccm.update("apple");
 *     ccm.update("banana");
 *
 *     System.out.println("Apple count (>= true): " + ccm.estimate("apple"));
 *     System.out.println("Total items: " + ccm.totalCount());
 * }
 * </pre>
 *
 * @see CountMinSketch
 */
public final class ConservativeCountMin extends NativeSketch implements MergeableSketch<ConservativeCountMin> {

    private final double epsilon;
    private final double delta;

    /**
     * Create a new ConservativeCountMin sketch.
     *
     * @param epsilon relative error bound (0 < epsilon < 1)
     *                Smaller values mean more accurate but larger space
     * @param delta failure probability (0 < delta < 1)
     *              Smaller values mean more confident but larger space
     * @throws IllegalArgumentException if parameters are invalid
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public ConservativeCountMin(double epsilon, double delta) {
        if (epsilon <= 0 || epsilon >= 1) {
            throw new IllegalArgumentException("epsilon must be in range (0, 1), got: " + epsilon);
        }
        if (delta <= 0 || delta >= 1) {
            throw new IllegalArgumentException("delta must be in range (0, 1), got: " + delta);
        }

        this.epsilon = epsilon;
        this.delta = delta;
        this.nativePtr = SketchOxideNative.conservativecountmin_new(epsilon, delta);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate ConservativeCountMin");
        }
    }

    /**
     * Update the frequency of an item (increment by 1).
     *
     * <p>Uses conservative update strategy: only increments counters
     * that equal the current minimum.
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
        SketchOxideNative.conservativecountmin_update(nativePtr, item);
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
     * <p>Returns an estimate that is >= the true frequency (never underestimates).
     * Due to conservative updates, overestimation is typically less than
     * standard Count-Min Sketch.
     *
     * @param item the item to query
     * @return estimated frequency (always >= true count)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long estimate(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.conservativecountmin_estimate(nativePtr, item);
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
     * Get the total count of all updates.
     *
     * <p>This is the exact count of all update() calls made to the sketch.
     *
     * @return total number of updates
     * @throws IllegalStateException if the sketch has been closed
     */
    public long totalCount() {
        checkAlive();
        return SketchOxideNative.conservativecountmin_totalCount(nativePtr);
    }

    /**
     * Merge another ConservativeCountMin into this one.
     *
     * <p>Both sketches must have the same epsilon and delta parameters.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(ConservativeCountMin other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.epsilon != other.epsilon || this.delta != other.delta) {
            throw new IllegalArgumentException(
                    "Cannot merge ConservativeCountMin sketches with different parameters: " +
                            String.format("(%.3f, %.3f) vs (%.3f, %.3f)",
                                    this.epsilon, this.delta, other.epsilon, other.delta));
        }

        SketchOxideNative.conservativecountmin_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the width parameter (number of counters per row).
     *
     * @return the width
     * @throws IllegalStateException if the sketch has been closed
     */
    public int width() {
        checkAlive();
        return SketchOxideNative.conservativecountmin_width(nativePtr);
    }

    /**
     * Get the depth parameter (number of hash functions/rows).
     *
     * @return the depth
     * @throws IllegalStateException if the sketch has been closed
     */
    public int depth() {
        checkAlive();
        return SketchOxideNative.conservativecountmin_depth(nativePtr);
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
        return SketchOxideNative.conservativecountmin_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new ConservativeCountMin instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static ConservativeCountMin deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.conservativecountmin_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized ConservativeCountMin data");
        }

        ConservativeCountMin ccm = new ConservativeCountMin(0.01, 0.01);
        SketchOxideNative.conservativecountmin_free(ccm.nativePtr);
        ccm.nativePtr = ptr;
        return ccm;
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
            SketchOxideNative.conservativecountmin_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("ConservativeCountMin(epsilon=%.3f, delta=%.3f, width=%d, depth=%d, total=%d)",
                        epsilon, delta, width(), depth(), totalCount());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "ConservativeCountMin(closed)";
    }
}
