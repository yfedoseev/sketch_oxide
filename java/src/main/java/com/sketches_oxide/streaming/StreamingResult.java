package com.sketches_oxide.streaming;

/**
 * StreamingResult - Result Container for Streaming Sketches with Error Bounds.
 *
 * Encapsulates the result of a streaming count query along with formal error bounds.
 * Streaming algorithms like Exponential Histogram provide (1+epsilon) approximation
 * guarantees, and this class captures both the estimate and these bounds.
 *
 * <p><strong>Error Model:</strong>
 * <ul>
 *   <li>The true count N satisfies: lowerBound &lt;= N &lt;= upperBound</li>
 *   <li>For (1+epsilon) approximation: lowerBound = estimate/(1+epsilon), upperBound = estimate*(1+epsilon)</li>
 *   <li>The relative error is bounded by epsilon</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Confidence intervals for streaming counts</li>
 *   <li>Error-aware decision making in analytics</li>
 *   <li>Quality assessment of streaming estimates</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * StreamingResult result = histogram.result();
 * System.out.printf("Count: %d (range: [%d, %d])%n",
 *     result.getEstimate(), result.getLowerBound(), result.getUpperBound());
 * System.out.printf("Relative error: +/- %.2f%%%n", result.getRelativeError() * 100);
 * </pre>
 *
 * @see ExponentialHistogram
 * @see SlidingWindowCounter
 */
public final class StreamingResult {

    private final long estimate;
    private final long lowerBound;
    private final long upperBound;
    private final double epsilon;

    /**
     * Create a streaming result with explicit bounds.
     *
     * @param estimate the estimated count
     * @param lowerBound the lower bound of the count
     * @param upperBound the upper bound of the count
     * @param epsilon the error parameter used
     */
    public StreamingResult(long estimate, long lowerBound, long upperBound, double epsilon) {
        this.estimate = estimate;
        this.lowerBound = lowerBound;
        this.upperBound = upperBound;
        this.epsilon = epsilon;
    }

    /**
     * Create a streaming result from an estimate and epsilon.
     *
     * Computes bounds as: lower = estimate/(1+epsilon), upper = estimate*(1+epsilon)
     *
     * @param estimate the estimated count
     * @param epsilon the error parameter
     * @return a new StreamingResult with computed bounds
     */
    public static StreamingResult fromEstimate(long estimate, double epsilon) {
        long lower = (long) Math.floor(estimate / (1.0 + epsilon));
        long upper = (long) Math.ceil(estimate * (1.0 + epsilon));
        return new StreamingResult(estimate, lower, upper, epsilon);
    }

    /**
     * Create an exact result with no error.
     *
     * @param count the exact count
     * @return a new StreamingResult with zero error
     */
    public static StreamingResult exact(long count) {
        return new StreamingResult(count, count, count, 0.0);
    }

    /**
     * Get the estimated count.
     *
     * @return the count estimate
     */
    public long getEstimate() {
        return estimate;
    }

    /**
     * Get the lower bound of the count.
     *
     * The true count is guaranteed to be at least this value.
     *
     * @return the lower bound
     */
    public long getLowerBound() {
        return lowerBound;
    }

    /**
     * Get the upper bound of the count.
     *
     * The true count is guaranteed to be at most this value.
     *
     * @return the upper bound
     */
    public long getUpperBound() {
        return upperBound;
    }

    /**
     * Get the epsilon parameter used.
     *
     * @return the error parameter
     */
    public double getEpsilon() {
        return epsilon;
    }

    /**
     * Get the relative error bound.
     *
     * The estimate is within +/- this fraction of the true value.
     *
     * @return the relative error (e.g., 0.01 for 1% error)
     */
    public double getRelativeError() {
        return epsilon;
    }

    /**
     * Get the absolute error bound.
     *
     * @return the maximum absolute difference between estimate and true count
     */
    public long getAbsoluteError() {
        return Math.max(upperBound - estimate, estimate - lowerBound);
    }

    /**
     * Check if the bounds contain a specific value.
     *
     * @param value the value to check
     * @return true if lowerBound &lt;= value &lt;= upperBound
     */
    public boolean contains(long value) {
        return value >= lowerBound && value <= upperBound;
    }

    /**
     * Get the width of the confidence interval.
     *
     * @return upperBound - lowerBound
     */
    public long getIntervalWidth() {
        return upperBound - lowerBound;
    }

    @Override
    public String toString() {
        if (epsilon == 0.0) {
            return String.format("StreamingResult(exact=%d)", estimate);
        }
        return String.format("StreamingResult(estimate=%d, bounds=[%d, %d], epsilon=%.4f)",
                estimate, lowerBound, upperBound, epsilon);
    }

    @Override
    public boolean equals(Object obj) {
        if (this == obj) return true;
        if (obj == null || getClass() != obj.getClass()) return false;
        StreamingResult other = (StreamingResult) obj;
        return estimate == other.estimate
                && lowerBound == other.lowerBound
                && upperBound == other.upperBound
                && Double.compare(other.epsilon, epsilon) == 0;
    }

    @Override
    public int hashCode() {
        int result = Long.hashCode(estimate);
        result = 31 * result + Long.hashCode(lowerBound);
        result = 31 * result + Long.hashCode(upperBound);
        result = 31 * result + Double.hashCode(epsilon);
        return result;
    }
}
