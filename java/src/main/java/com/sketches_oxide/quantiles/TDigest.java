package com.sketches_oxide.quantiles;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * TDigest - High Accuracy Quantile Estimation at Distribution Tails.
 *
 * A quantile sketch that provides high accuracy at the extreme quantiles
 * (near 0 and 1) with gracefully degrading accuracy toward the median.
 * Uses a clever clustering approach that naturally allocates more precision
 * to the tails.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Maintains a set of weighted centroids (mean, weight pairs)</li>
 *   <li>Centroids near the edges are smaller (higher precision)</li>
 *   <li>Centroids near the median are larger (lower precision)</li>
 *   <li>Compression parameter controls total number of centroids</li>
 *   <li>Merging combines centroids while respecting size constraints</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Latency percentile monitoring (especially p99, p99.9)</li>
 *   <li>Real-time analytics dashboards</li>
 *   <li>Database query optimization</li>
 *   <li>Any application needing tail accuracy</li>
 * </ul>
 *
 * <p><strong>Features:</strong>
 * <ul>
 *   <li>Excellent accuracy at extreme percentiles</li>
 *   <li>Fast merging for distributed systems</li>
 *   <li>Supports CDF queries (inverse of quantile)</li>
 *   <li>Trimmed mean calculation for robust statistics</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (TDigest td = new TDigest(100)) {
 *     // Add observations
 *     for (double value : values) {
 *         td.update(value);
 *     }
 *
 *     // Query percentiles
 *     System.out.println("Median: " + td.quantile(0.5));
 *     System.out.println("p95: " + td.quantile(0.95));
 *     System.out.println("p99.9: " + td.quantile(0.999));
 *
 *     // CDF: what fraction of values are below 100ms?
 *     System.out.println("CDF(100): " + td.cdf(100.0));
 *
 *     // Trimmed mean (robust to outliers)
 *     System.out.println("Trimmed mean (5-95%): " + td.trimmedMean(0.05, 0.95));
 * }
 * </pre>
 *
 * @see DDSketch
 * @see KllSketch
 * @see <a href="https://github.com/tdunning/t-digest">TDigest Original Implementation</a>
 */
public final class TDigest extends NativeSketch implements MergeableSketch<TDigest> {

    private final double compression;

    /**
     * Create a new TDigest.
     *
     * @param compression the compression parameter (typically 100-500)
     *                    Higher values = more centroids = higher accuracy but more space
     *                    Recommended: 100 for general use, 200+ for high precision
     * @throws IllegalArgumentException if compression &lt;= 0
     * @throws RuntimeException if native memory allocation fails
     */
    public TDigest(double compression) {
        if (compression <= 0) {
            throw new IllegalArgumentException("compression must be positive, got: " + compression);
        }

        this.compression = compression;
        this.nativePtr = SketchOxideNative.tdigest_new(compression);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate TDigest");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private TDigest(long nativePtr, double compression) {
        this.nativePtr = nativePtr;
        this.compression = compression;
    }

    /**
     * Add a value to the digest.
     *
     * @param value the value to add
     * @throws IllegalStateException if the digest has been closed
     */
    public void update(double value) {
        checkAlive();
        SketchOxideNative.tdigest_update(nativePtr, value);
    }

    /**
     * Get the estimated value at a given quantile.
     *
     * Accuracy is highest near the extremes (0 and 1) and lowest near the median.
     *
     * @param q the quantile to query (0 &lt;= q &lt;= 1)
     * @return the estimated value at quantile q
     * @throws IllegalStateException if the digest has been closed
     * @throws IllegalArgumentException if q not in [0, 1]
     */
    public double quantile(double q) {
        checkAlive();
        if (q < 0 || q > 1) {
            throw new IllegalArgumentException("quantile must be in [0, 1], got: " + q);
        }
        return SketchOxideNative.tdigest_quantile(nativePtr, q);
    }

    /**
     * Get estimated values at multiple quantiles efficiently.
     *
     * @param quantiles array of quantiles to query (each in [0, 1])
     * @return array of estimated values, one per input quantile
     * @throws IllegalStateException if the digest has been closed
     * @throws IllegalArgumentException if any quantile not in [0, 1]
     * @throws NullPointerException if quantiles is null
     */
    public double[] quantiles(double[] quantiles) {
        checkAlive();
        if (quantiles == null) {
            throw new NullPointerException("quantiles cannot be null");
        }

        double[] results = new double[quantiles.length];
        for (int i = 0; i < quantiles.length; i++) {
            if (quantiles[i] < 0 || quantiles[i] > 1) {
                throw new IllegalArgumentException(
                        "quantile must be in [0, 1], got: " + quantiles[i] + " at index " + i);
            }
            results[i] = SketchOxideNative.tdigest_quantile(nativePtr, quantiles[i]);
        }
        return results;
    }

    /**
     * Get the cumulative distribution function value at a point.
     *
     * Returns the estimated fraction of values less than or equal to the given value.
     * This is the inverse of the quantile function.
     *
     * @param value the value to query
     * @return the estimated fraction of values &lt;= value (in range [0, 1])
     * @throws IllegalStateException if the digest has been closed
     */
    public double cdf(double value) {
        checkAlive();
        // CDF is the inverse of quantile - find q such that quantile(q) = value
        // We use binary search to approximate this
        double lo = 0.0, hi = 1.0;
        for (int i = 0; i < 50; i++) { // 50 iterations gives ~15 decimal places
            double mid = (lo + hi) / 2;
            if (quantile(mid) < value) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        return (lo + hi) / 2;
    }

    /**
     * Calculate the trimmed mean of the distribution.
     *
     * The trimmed mean excludes the lowest and highest portions of the distribution,
     * making it more robust to outliers than the regular mean.
     *
     * @param lower the lower quantile cutoff (e.g., 0.05 for 5%)
     * @param upper the upper quantile cutoff (e.g., 0.95 for 95%)
     * @return the mean of values between the lower and upper quantiles
     * @throws IllegalStateException if the digest has been closed
     * @throws IllegalArgumentException if lower &gt;= upper or either not in [0, 1]
     */
    public double trimmedMean(double lower, double upper) {
        checkAlive();
        if (lower < 0 || lower > 1) {
            throw new IllegalArgumentException("lower must be in [0, 1], got: " + lower);
        }
        if (upper < 0 || upper > 1) {
            throw new IllegalArgumentException("upper must be in [0, 1], got: " + upper);
        }
        if (lower >= upper) {
            throw new IllegalArgumentException(
                    "lower must be less than upper, got: " + lower + " >= " + upper);
        }

        // Approximate trimmed mean by sampling quantiles
        int samples = 100;
        double sum = 0;
        double step = (upper - lower) / samples;
        for (int i = 0; i < samples; i++) {
            double q = lower + (i + 0.5) * step;
            sum += quantile(q);
        }
        return sum / samples;
    }

    /**
     * Merge another TDigest into this one.
     *
     * After merging, this digest represents the combined distribution.
     * Both digests should have similar compression parameters for best results.
     *
     * @param other the digest to merge into this one
     * @throws IllegalStateException if either digest is closed
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(TDigest other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        SketchOxideNative.tdigest_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Serialize the digest to binary format.
     *
     * @return binary representation of the digest
     * @throws IllegalStateException if the digest has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.tdigest_serialize(nativePtr);
    }

    /**
     * Deserialize a TDigest from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new TDigest instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static TDigest deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.tdigest_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized TDigest data");
        }

        return new TDigest(ptr, 100);
    }

    /**
     * Get the compression parameter.
     *
     * @return the compression value used at creation
     */
    public double getCompression() {
        return compression;
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
            SketchOxideNative.tdigest_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("TDigest(compression=%.1f)", compression);
        }
        return "TDigest(closed)";
    }
}
