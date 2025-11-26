package com.sketches_oxide.frequency;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * RemovableUniversalSketch - Stream Deletions Support with L2 Norm Estimation.
 *
 * <p>RemovableUniversalSketch is a frequency estimation data structure that supports
 * both positive and negative updates (insertions and deletions). It also provides
 * L2 norm estimation of the frequency vector, useful for detecting significant changes.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>RemovableUniversalSketch extends count-sketch-style algorithms to support:
 * <ul>
 *   <li>Positive updates (insertions): increment(item, delta)</li>
 *   <li>Negative updates (deletions): increment(item, -delta)</li>
 *   <li>L2 norm estimation: sqrt(sum of squared frequencies)</li>
 * </ul>
 *
 * <p>This is achieved using signed counters and careful analysis of the
 * sign-randomized sum of updates.
 *
 * <p><strong>Key Features:</strong>
 * <ul>
 *   <li>Supports stream deletions (negative frequencies possible)</li>
 *   <li>L2 norm estimation for change detection</li>
 *   <li>Unbiased frequency estimates</li>
 *   <li>Useful for turnstile streaming model</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Database transaction logs with INSERT/DELETE</li>
 *   <li>Inventory tracking with additions and removals</li>
 *   <li>Network flow monitoring with connection open/close</li>
 *   <li>Change detection in streaming data</li>
 *   <li>Anomaly detection using L2 norm</li>
 * </ul>
 *
 * <p><strong>Space Complexity:</strong> O(1/epsilon^2 * ln(1/delta))
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (RemovableUniversalSketch rus = new RemovableUniversalSketch(0.01, 0.01)) {
 *     // Add items
 *     rus.update("item1");
 *     rus.update("item1");
 *     rus.update("item2");
 *
 *     // Remove an item
 *     rus.update("item1", -1);  // Decrement by 1
 *
 *     // Query frequency (may be negative if more deletions than insertions)
 *     System.out.println("item1 count: " + rus.estimate("item1"));  // ~1
 *
 *     // Check L2 norm
 *     if (rus.hasL2Norm()) {
 *         System.out.println("L2 norm: " + rus.l2Norm());
 *     }
 * }
 * </pre>
 *
 * @see CountSketch
 * @see CountMinSketch
 */
public final class RemovableUniversalSketch extends NativeSketch implements MergeableSketch<RemovableUniversalSketch> {

    private final double epsilon;
    private final double delta;

    /**
     * Create a new RemovableUniversalSketch.
     *
     * @param epsilon relative error bound (0 < epsilon < 1)
     *                Smaller values mean more accurate but larger space
     * @param delta failure probability (0 < delta < 1)
     *              Smaller values mean more confident but larger space
     * @throws IllegalArgumentException if parameters are invalid
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public RemovableUniversalSketch(double epsilon, double delta) {
        if (epsilon <= 0 || epsilon >= 1) {
            throw new IllegalArgumentException("epsilon must be in range (0, 1), got: " + epsilon);
        }
        if (delta <= 0 || delta >= 1) {
            throw new IllegalArgumentException("delta must be in range (0, 1), got: " + delta);
        }

        this.epsilon = epsilon;
        this.delta = delta;
        this.nativePtr = SketchOxideNative.removableuniversalsketch_new(epsilon, delta);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate RemovableUniversalSketch");
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
        SketchOxideNative.removableuniversalsketch_update(nativePtr, item);
    }

    /**
     * Update with a string item (increment by 1).
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
     * Update the frequency of an item with a delta value.
     *
     * <p>Positive delta adds to the frequency (insertion).
     * Negative delta subtracts from the frequency (deletion).
     *
     * @param item the item to update
     * @param delta the change in frequency (can be negative for deletions)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item, long delta) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.removableuniversalsketch_updateWithDelta(nativePtr, item, delta);
    }

    /**
     * Update a string item with a delta value.
     *
     * @param item the string to update
     * @param delta the change in frequency (can be negative for deletions)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(String item, long delta) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        update(item.getBytes(), delta);
    }

    /**
     * Estimate the frequency of an item.
     *
     * <p>Returns an unbiased estimate that may be negative if there were
     * more deletions than insertions for this item.
     *
     * @param item the item to query
     * @return estimated frequency (may be negative)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long estimate(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.removableuniversalsketch_estimate(nativePtr, item);
    }

    /**
     * Estimate the frequency of a string item.
     *
     * @param item the string to query
     * @return estimated frequency (may be negative)
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
     * Check if L2 norm estimation is available.
     *
     * <p>L2 norm estimation may not be available in all states of the sketch.
     *
     * @return true if L2 norm can be computed
     * @throws IllegalStateException if the sketch has been closed
     */
    public boolean hasL2Norm() {
        checkAlive();
        return SketchOxideNative.removableuniversalsketch_hasL2Norm(nativePtr);
    }

    /**
     * Get the estimated L2 norm of the frequency vector.
     *
     * <p>L2 norm is sqrt(sum of squared frequencies), useful for:
     * <ul>
     *   <li>Detecting significant changes in the stream</li>
     *   <li>Measuring overall activity level</li>
     *   <li>Anomaly detection</li>
     * </ul>
     *
     * @return estimated L2 norm
     * @throws IllegalStateException if the sketch has been closed or L2 norm unavailable
     */
    public double l2Norm() {
        checkAlive();
        if (!hasL2Norm()) {
            throw new IllegalStateException("L2 norm is not available in current sketch state");
        }
        return SketchOxideNative.removableuniversalsketch_l2Norm(nativePtr);
    }

    /**
     * Merge another RemovableUniversalSketch into this one.
     *
     * <p>Both sketches must have the same epsilon and delta parameters.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(RemovableUniversalSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.epsilon != other.epsilon || this.delta != other.delta) {
            throw new IllegalArgumentException(
                    "Cannot merge RemovableUniversalSketch instances with different parameters: " +
                            String.format("(%.3f, %.3f) vs (%.3f, %.3f)",
                                    this.epsilon, this.delta, other.epsilon, other.delta));
        }

        SketchOxideNative.removableuniversalsketch_merge(this.nativePtr, other.nativePtr);
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
        return SketchOxideNative.removableuniversalsketch_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new RemovableUniversalSketch instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static RemovableUniversalSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.removableuniversalsketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized RemovableUniversalSketch data");
        }

        RemovableUniversalSketch rus = new RemovableUniversalSketch(0.01, 0.01);
        SketchOxideNative.removableuniversalsketch_free(rus.nativePtr);
        rus.nativePtr = ptr;
        return rus;
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
            SketchOxideNative.removableuniversalsketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                String l2Info = hasL2Norm() ? String.format(", l2Norm=%.2f", l2Norm()) : "";
                return String.format("RemovableUniversalSketch(epsilon=%.3f, delta=%.3f%s)",
                        epsilon, delta, l2Info);
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "RemovableUniversalSketch(closed)";
    }
}
