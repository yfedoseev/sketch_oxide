package com.sketches_oxide.frequency;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * SALSA - Self-Adjusting Learned Sketch Algorithm.
 *
 * <p>SALSA is a frequency estimation sketch that dynamically adjusts its counter
 * sizing based on observed data patterns. It uses self-adjusting counter widths
 * to optimize accuracy for the specific workload.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>SALSA employs adaptive counter sizing:
 * <ul>
 *   <li>Counters start small and grow as needed</li>
 *   <li>High-frequency items get larger counters automatically</li>
 *   <li>Low-frequency items use minimal space</li>
 *   <li>Adaptation happens dynamically during updates</li>
 * </ul>
 *
 * <p><strong>Key Features:</strong>
 * <ul>
 *   <li>Self-adjusting counter sizes</li>
 *   <li>Workload-adaptive accuracy</li>
 *   <li>Better memory utilization than fixed-width counters</li>
 *   <li>Confidence metrics for estimate quality</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Frequency estimation with unknown distributions</li>
 *   <li>Adaptive heavy hitter detection</li>
 *   <li>Workload-specific optimization</li>
 *   <li>Memory-constrained environments</li>
 * </ul>
 *
 * <p><strong>Space Complexity:</strong> O(1/epsilon * ln(1/delta)) (adaptive)
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (SALSA salsa = new SALSA(0.01, 0.01)) {
 *     for (String item : stream) {
 *         salsa.update(item);
 *     }
 *
 *     System.out.println("Frequency: " + salsa.estimate("key"));
 *     System.out.println("Confidence: " + salsa.confidenceMetric());
 *     System.out.println("Adaptation level: " + salsa.adaptationLevel());
 * }
 * </pre>
 *
 * @see <a href="https://arxiv.org/abs/2004.07954">SALSA Paper</a>
 * @see CountMinSketch
 * @see CountSketch
 */
public final class SALSA extends NativeSketch implements MergeableSketch<SALSA> {

    private final double epsilon;
    private final double delta;

    /**
     * Create a new SALSA sketch.
     *
     * @param epsilon relative error bound (0 < epsilon < 1)
     *                Smaller values mean more accurate but larger space
     * @param delta failure probability (0 < delta < 1)
     *              Smaller values mean more confident but larger space
     * @throws IllegalArgumentException if parameters are invalid
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public SALSA(double epsilon, double delta) {
        if (epsilon <= 0 || epsilon >= 1) {
            throw new IllegalArgumentException("epsilon must be in range (0, 1), got: " + epsilon);
        }
        if (delta <= 0 || delta >= 1) {
            throw new IllegalArgumentException("delta must be in range (0, 1), got: " + delta);
        }

        this.epsilon = epsilon;
        this.delta = delta;
        this.nativePtr = SketchOxideNative.salsa_new(epsilon, delta);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate SALSA");
        }
    }

    /**
     * Update the frequency of an item (increment by 1).
     *
     * <p>The sketch may adapt its internal counter sizing based on this update.
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
        SketchOxideNative.salsa_update(nativePtr, item);
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
     * <p>Returns an estimate using the adapted counter configuration.
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
        return SketchOxideNative.salsa_estimate(nativePtr, item);
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
     * Get the confidence metric for the current sketch state.
     *
     * <p>Higher values indicate more confident estimates. This metric
     * reflects how well the sketch has adapted to the data distribution.
     *
     * @return confidence metric (0.0 to 1.0)
     * @throws IllegalStateException if the sketch has been closed
     */
    public double confidenceMetric() {
        checkAlive();
        return SketchOxideNative.salsa_confidenceMetric(nativePtr);
    }

    /**
     * Get the current adaptation level.
     *
     * <p>Indicates how much the sketch has adapted from its initial state.
     * Higher levels indicate more adaptation has occurred.
     *
     * @return adaptation level (0 = no adaptation)
     * @throws IllegalStateException if the sketch has been closed
     */
    public int adaptationLevel() {
        checkAlive();
        return SketchOxideNative.salsa_adaptationLevel(nativePtr);
    }

    /**
     * Merge another SALSA into this one.
     *
     * <p>Both sketches must have the same epsilon and delta parameters.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(SALSA other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.epsilon != other.epsilon || this.delta != other.delta) {
            throw new IllegalArgumentException(
                    "Cannot merge SALSA sketches with different parameters: " +
                            String.format("(%.3f, %.3f) vs (%.3f, %.3f)",
                                    this.epsilon, this.delta, other.epsilon, other.delta));
        }

        SketchOxideNative.salsa_merge(this.nativePtr, other.nativePtr);
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
        return SketchOxideNative.salsa_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new SALSA instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static SALSA deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.salsa_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized SALSA data");
        }

        SALSA salsa = new SALSA(0.01, 0.01);
        SketchOxideNative.salsa_free(salsa.nativePtr);
        salsa.nativePtr = ptr;
        return salsa;
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
            SketchOxideNative.salsa_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("SALSA(epsilon=%.3f, delta=%.3f, confidence=%.2f, adaptation=%d)",
                        epsilon, delta, confidenceMetric(), adaptationLevel());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "SALSA(closed)";
    }
}
