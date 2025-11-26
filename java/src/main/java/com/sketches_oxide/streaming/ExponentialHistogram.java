package com.sketches_oxide.streaming;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * ExponentialHistogram - Counting with Formal Error Bounds.
 *
 * Maintains an approximate count of 1-bits in a sliding window over a binary stream.
 * Provides a (1+epsilon) approximation guarantee with provable error bounds.
 * This is the classic algorithm by Datar et al. (DGIM) for the basic counting problem.
 *
 * <p><strong>Algorithm Details:</strong>
 * <ul>
 *   <li>Maintains buckets of exponentially increasing sizes</li>
 *   <li>Each bucket represents a timestamp and count of 1s</li>
 *   <li>At most O(1/epsilon * log(N)) buckets are maintained</li>
 *   <li>Merging buckets maintains the approximation invariant</li>
 * </ul>
 *
 * <p><strong>Error Guarantee:</strong>
 * Let C be the true count. The estimate E satisfies:
 * <pre>
 *   C / (1 + epsilon) &lt;= E &lt;= C * (1 + epsilon)
 * </pre>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Counting events in sliding windows</li>
 *   <li>Network traffic analysis</li>
 *   <li>Stream processing with memory constraints</li>
 *   <li>Time-series anomaly detection</li>
 * </ul>
 *
 * <p><strong>Performance:</strong>
 * <ul>
 *   <li>Space: O((1/epsilon) * log(windowSize))</li>
 *   <li>Time: O(log(1/epsilon)) amortized per insert</li>
 *   <li>Query: O(1)</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * // Track events in last 10000 time units with 5% error
 * try (ExponentialHistogram histogram = new ExponentialHistogram(10000, 0.05)) {
 *     // Insert events
 *     for (int i = 0; i &lt; 1000; i++) {
 *         histogram.insert();
 *     }
 *
 *     // Get count with error bounds
 *     StreamingResult result = histogram.result();
 *     System.out.printf("Events: %d [%d, %d]%n",
 *         result.getEstimate(), result.getLowerBound(), result.getUpperBound());
 *
 *     // Expire old events (advance time to 5000)
 *     histogram.expire(5000);
 * }
 * </pre>
 *
 * @see StreamingResult
 * @see SlidingWindowCounter
 * @see <a href="https://doi.org/10.1137/S0097539701398363">DGIM Paper</a>
 */
public final class ExponentialHistogram extends NativeSketch {

    private final long windowSize;
    private final double epsilon;

    /**
     * Create a new ExponentialHistogram.
     *
     * @param windowSize the size of the sliding window (in time units)
     * @param epsilon the error parameter (0 &lt; epsilon &lt; 1)
     *                Error guarantee: (1+epsilon) multiplicative factor
     *                Typical values: 0.01 to 0.1
     * @throws IllegalArgumentException if windowSize &lt;= 0 or epsilon is out of range
     */
    public ExponentialHistogram(long windowSize, double epsilon) {
        if (windowSize <= 0) {
            throw new IllegalArgumentException("windowSize must be positive, got: " + windowSize);
        }
        if (epsilon <= 0 || epsilon >= 1) {
            throw new IllegalArgumentException("epsilon must be in range (0, 1), got: " + epsilon);
        }

        this.windowSize = windowSize;
        this.epsilon = epsilon;
        this.nativePtr = SketchOxideNative.exponentialhistogram_new(windowSize, epsilon);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate ExponentialHistogram");
        }
    }

    /**
     * Insert a 1-bit into the stream.
     *
     * Records an event at the current logical time.
     * The internal bucket structure is automatically maintained.
     *
     * @throws IllegalStateException if the histogram has been closed
     */
    public void insert() {
        checkAlive();
        SketchOxideNative.exponentialhistogram_insert(nativePtr);
    }

    /**
     * Insert multiple events.
     *
     * Convenience method for recording multiple events.
     *
     * @param count number of events to insert (must be positive)
     * @throws IllegalArgumentException if count is not positive
     * @throws IllegalStateException if the histogram has been closed
     */
    public void insertBatch(long count) {
        checkAlive();
        if (count <= 0) {
            throw new IllegalArgumentException("count must be positive, got: " + count);
        }
        for (long i = 0; i < count; i++) {
            SketchOxideNative.exponentialhistogram_insert(nativePtr);
        }
    }

    /**
     * Get the estimated count of 1-bits in the window.
     *
     * Returns an approximate count with (1+epsilon) multiplicative error.
     *
     * @return the estimated count
     * @throws IllegalStateException if the histogram has been closed
     */
    public long count() {
        checkAlive();
        return SketchOxideNative.exponentialhistogram_count(nativePtr);
    }

    /**
     * Get the count result with formal error bounds.
     *
     * The bounds are computed based on the epsilon parameter:
     * <ul>
     *   <li>Lower bound: estimate / (1 + epsilon)</li>
     *   <li>Upper bound: estimate * (1 + epsilon)</li>
     * </ul>
     *
     * @return a StreamingResult containing estimate and bounds
     * @throws IllegalStateException if the histogram has been closed
     */
    public StreamingResult result() {
        checkAlive();
        long estimate = count();
        return StreamingResult.fromEstimate(estimate, epsilon);
    }

    /**
     * Expire events older than the given time.
     *
     * Removes buckets that fall outside the sliding window.
     * Call this periodically to maintain the window size invariant.
     *
     * @param time the current logical time
     * @throws IllegalStateException if the histogram has been closed
     */
    public void expire(long time) {
        checkAlive();
        SketchOxideNative.exponentialhistogram_expire(nativePtr, time);
    }

    /**
     * Get the window size.
     *
     * @return the window size parameter
     */
    public long getWindowSize() {
        return windowSize;
    }

    /**
     * Get the epsilon parameter.
     *
     * @return the error parameter
     */
    public double getEpsilon() {
        return epsilon;
    }

    /**
     * Get the theoretical maximum error.
     *
     * @return the maximum relative error (epsilon)
     */
    public double getMaxRelativeError() {
        return epsilon;
    }

    /**
     * Serialize the histogram to binary format.
     *
     * The serialized form includes all bucket information and can be
     * used to restore the histogram state exactly.
     *
     * @return binary representation of the histogram
     * @throws IllegalStateException if the histogram has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.exponentialhistogram_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * Reconstructs an ExponentialHistogram from serialized data.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new ExponentialHistogram instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static ExponentialHistogram deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.exponentialhistogram_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized ExponentialHistogram data");
        }

        // Create with dummy values, then replace pointer
        ExponentialHistogram histogram = new ExponentialHistogram(1000, 0.01);
        SketchOxideNative.exponentialhistogram_free(histogram.nativePtr);
        histogram.nativePtr = ptr;
        return histogram;
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
            SketchOxideNative.exponentialhistogram_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                StreamingResult res = result();
                return String.format(
                        "ExponentialHistogram(windowSize=%d, epsilon=%.4f, count=%d, bounds=[%d,%d])",
                        windowSize, epsilon, res.getEstimate(),
                        res.getLowerBound(), res.getUpperBound());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "ExponentialHistogram(closed)";
    }
}
