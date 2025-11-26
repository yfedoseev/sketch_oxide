package com.sketches_oxide.quantiles;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * SplineSketch - Quantile Estimation using Monotone Spline Interpolation.
 *
 * A quantile sketch that uses monotone cubic spline interpolation between
 * stored bucket boundaries. This approach provides smooth quantile estimates
 * and can achieve high accuracy with relatively few buckets.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Maintains a set of buckets representing the distribution</li>
 *   <li>Uses monotone cubic Hermite spline interpolation</li>
 *   <li>Guarantees monotonicity: quantile(q1) &lt;= quantile(q2) if q1 &lt;= q2</li>
 *   <li>Smooth estimates between bucket boundaries</li>
 *   <li>Space: O(maxBuckets) storage</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Smooth percentile curves for visualization</li>
 *   <li>High-precision quantile estimation</li>
 *   <li>Applications requiring monotonic guarantees</li>
 *   <li>Statistical analysis requiring smooth CDFs</li>
 * </ul>
 *
 * <p><strong>Advantages:</strong>
 * <ul>
 *   <li>Smooth, monotonic quantile estimates</li>
 *   <li>No discontinuities in quantile function</li>
 *   <li>Good accuracy with moderate bucket count</li>
 *   <li>Intuitive parameter (number of buckets)</li>
 * </ul>
 *
 * <p><strong>Note:</strong> This sketch uses {@link #query(double)} instead of
 * the typical quantile() method to emphasize the spline-based interpolation.
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (SplineSketch sketch = new SplineSketch(100)) {
 *     // Add observations
 *     for (double value : values) {
 *         sketch.update(value);
 *     }
 *
 *     // Query quantiles (uses spline interpolation)
 *     System.out.println("p50: " + sketch.query(0.5));
 *     System.out.println("p95: " + sketch.query(0.95));
 *     System.out.println("p99: " + sketch.query(0.99));
 *
 *     // Generate smooth percentile curve
 *     for (double q = 0; q &lt;= 1.0; q += 0.01) {
 *         System.out.println(q + "," + sketch.query(q));
 *     }
 * }
 * </pre>
 *
 * @see DDSketch
 * @see KllSketch
 * @see TDigest
 */
public final class SplineSketch extends NativeSketch implements MergeableSketch<SplineSketch> {

    private final long maxBuckets;

    /**
     * Create a new SplineSketch.
     *
     * @param maxBuckets the maximum number of buckets to use
     *                   More buckets = higher accuracy but more space
     *                   Typical values: 50-500, recommended default is 100
     * @throws IllegalArgumentException if maxBuckets &lt;= 0
     * @throws RuntimeException if native memory allocation fails
     */
    public SplineSketch(long maxBuckets) {
        if (maxBuckets <= 0) {
            throw new IllegalArgumentException("maxBuckets must be positive, got: " + maxBuckets);
        }

        this.maxBuckets = maxBuckets;
        this.nativePtr = SketchOxideNative.splinesketch_new(maxBuckets);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate SplineSketch");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private SplineSketch(long nativePtr, long maxBuckets) {
        this.nativePtr = nativePtr;
        this.maxBuckets = maxBuckets;
    }

    /**
     * Add a value to the sketch.
     *
     * @param value the value to add
     * @throws IllegalStateException if the sketch has been closed
     */
    public void update(double value) {
        checkAlive();
        SketchOxideNative.splinesketch_update(nativePtr, value);
    }

    /**
     * Query the estimated value at a given quantile using spline interpolation.
     *
     * Uses monotone cubic Hermite spline interpolation to provide smooth
     * estimates between bucket boundaries. Guarantees monotonicity:
     * query(q1) &lt;= query(q2) when q1 &lt;= q2.
     *
     * @param q the quantile to query (0 &lt;= q &lt;= 1)
     * @return the estimated value at quantile q
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if q not in [0, 1]
     */
    public double query(double q) {
        checkAlive();
        if (q < 0 || q > 1) {
            throw new IllegalArgumentException("quantile must be in [0, 1], got: " + q);
        }
        return SketchOxideNative.splinesketch_query(nativePtr, q);
    }

    /**
     * Alias for query() to match standard quantile sketch interface.
     *
     * @param q the quantile to query (0 &lt;= q &lt;= 1)
     * @return the estimated value at quantile q
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if q not in [0, 1]
     */
    public double quantile(double q) {
        return query(q);
    }

    /**
     * Merge another SplineSketch into this one.
     *
     * After merging, this sketch represents the combined distribution.
     * Both sketches should have the same maxBuckets parameter.
     *
     * @param other the sketch to merge into this one
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if maxBuckets parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(SplineSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.maxBuckets != other.maxBuckets) {
            throw new IllegalArgumentException(
                    "Cannot merge SplineSketches with different maxBuckets: " +
                            this.maxBuckets + " vs " + other.maxBuckets);
        }

        SketchOxideNative.splinesketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.splinesketch_serialize(nativePtr);
    }

    /**
     * Deserialize a SplineSketch from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new SplineSketch instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static SplineSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.splinesketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized SplineSketch data");
        }

        return new SplineSketch(ptr, 100);
    }

    /**
     * Get the maximum buckets parameter.
     *
     * @return the maxBuckets value used at creation
     */
    public long getMaxBuckets() {
        return maxBuckets;
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
            SketchOxideNative.splinesketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("SplineSketch(maxBuckets=%d)", maxBuckets);
        }
        return "SplineSketch(closed)";
    }
}
