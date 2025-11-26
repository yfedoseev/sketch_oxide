package com.sketches_oxide.streaming;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * SlidingWindowCounter - Time-Bounded Event Counting.
 *
 * Counts events within a sliding time window using space-efficient data structures.
 * Provides approximate counts with (1+epsilon) multiplicative error guarantee.
 * Based on the Datar-Gionis-Indyk-Motwani (DGIM) algorithm.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Maintains a compressed representation of recent events</li>
 *   <li>Uses exponentially-spaced buckets for space efficiency</li>
 *   <li>Provides (1+epsilon) approximation using O((1/epsilon) * log^2(N)) space</li>
 *   <li>Events outside the window are automatically expired</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Rate limiting (requests per minute/hour)</li>
 *   <li>Anomaly detection (traffic spikes)</li>
 *   <li>Rolling statistics for dashboards</li>
 *   <li>Network flow monitoring</li>
 * </ul>
 *
 * <p><strong>Performance:</strong>
 * <ul>
 *   <li>Space: O((1/epsilon) * log^2(windowSize))</li>
 *   <li>Time: O(log(windowSize)) per operation</li>
 *   <li>Error: (1+epsilon) multiplicative factor</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * // Count events in last 1000 time units with 1% error
 * try (SlidingWindowCounter counter = new SlidingWindowCounter(1000, 0.01)) {
 *     for (int i = 0; i &lt; 500; i++) {
 *         counter.increment();
 *     }
 *     System.out.println("Events in window: " + counter.count());
 *
 *     // Get result with bounds
 *     StreamingResult result = counter.result();
 *     System.out.printf("Count: %d [%d, %d]%n",
 *         result.getEstimate(), result.getLowerBound(), result.getUpperBound());
 * }
 * </pre>
 *
 * @see StreamingResult
 * @see ExponentialHistogram
 * @see <a href="https://en.wikipedia.org/wiki/Streaming_algorithm">Streaming Algorithms</a>
 */
public final class SlidingWindowCounter extends NativeSketch {

    private final long windowSize;
    private final double epsilon;

    /**
     * Create a new SlidingWindowCounter.
     *
     * @param windowSize the size of the sliding window (in time units)
     * @param epsilon the error parameter (0 &lt; epsilon &lt; 1)
     *                Smaller epsilon = more accurate but more memory
     *                Typical values: 0.01 (1% error) to 0.1 (10% error)
     * @throws IllegalArgumentException if windowSize &lt;= 0 or epsilon is out of range
     */
    public SlidingWindowCounter(long windowSize, double epsilon) {
        if (windowSize <= 0) {
            throw new IllegalArgumentException("windowSize must be positive, got: " + windowSize);
        }
        if (epsilon <= 0 || epsilon >= 1) {
            throw new IllegalArgumentException("epsilon must be in range (0, 1), got: " + epsilon);
        }

        this.windowSize = windowSize;
        this.epsilon = epsilon;
        this.nativePtr = SketchOxideNative.slidingwindowcounter_new(windowSize, epsilon);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate SlidingWindowCounter");
        }
    }

    /**
     * Increment the counter by 1.
     *
     * Records a single event at the current logical time.
     *
     * @throws IllegalStateException if the counter has been closed
     */
    public void increment() {
        checkAlive();
        SketchOxideNative.slidingwindowcounter_increment(nativePtr);
    }

    /**
     * Increment the counter by a specified delta.
     *
     * Records multiple events at the current logical time.
     *
     * @param delta the number of events to add (must be positive)
     * @throws IllegalArgumentException if delta is not positive
     * @throws IllegalStateException if the counter has been closed
     */
    public void incrementBy(long delta) {
        checkAlive();
        if (delta <= 0) {
            throw new IllegalArgumentException("delta must be positive, got: " + delta);
        }
        for (long i = 0; i < delta; i++) {
            SketchOxideNative.slidingwindowcounter_increment(nativePtr);
        }
    }

    /**
     * Get the estimated count of events in the current window.
     *
     * Returns an approximate count with (1+epsilon) multiplicative error.
     *
     * @return the estimated event count
     * @throws IllegalStateException if the counter has been closed
     */
    public long count() {
        checkAlive();
        return SketchOxideNative.slidingwindowcounter_count(nativePtr);
    }

    /**
     * Get the count result with error bounds.
     *
     * @return a StreamingResult containing estimate and bounds
     * @throws IllegalStateException if the counter has been closed
     */
    public StreamingResult result() {
        checkAlive();
        long estimate = count();
        return StreamingResult.fromEstimate(estimate, epsilon);
    }

    /**
     * Expire events older than the given time.
     *
     * Removes events that fall outside the sliding window based on
     * the current logical time.
     *
     * @param time the current logical time
     * @throws IllegalStateException if the counter has been closed
     */
    public void expire(long time) {
        checkAlive();
        SketchOxideNative.slidingwindowcounter_expire(nativePtr, time);
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
     * Serialize the counter to binary format.
     *
     * Useful for:
     * <ul>
     *   <li>Checkpointing stream processing state</li>
     *   <li>Distributed aggregation</li>
     *   <li>Persistence across restarts</li>
     * </ul>
     *
     * @return binary representation of the counter
     * @throws IllegalStateException if the counter has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.slidingwindowcounter_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new SlidingWindowCounter instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static SlidingWindowCounter deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.slidingwindowcounter_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized SlidingWindowCounter data");
        }

        // Create with dummy values, then replace pointer
        SlidingWindowCounter counter = new SlidingWindowCounter(1000, 0.01);
        SketchOxideNative.slidingwindowcounter_free(counter.nativePtr);
        counter.nativePtr = ptr;
        return counter;
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
            SketchOxideNative.slidingwindowcounter_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("SlidingWindowCounter(windowSize=%d, epsilon=%.4f, count=%d)",
                        windowSize, epsilon, count());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "SlidingWindowCounter(closed)";
    }
}
