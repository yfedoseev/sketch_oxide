package com.sketches_oxide.quantiles;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * ReqSketch - Relative Error Quantile Sketch with Extreme Accuracy at Tails.
 *
 * A quantile sketch designed for scenarios where accuracy at the extremes
 * (very low or very high quantiles) is critical. The sketch provides
 * zero error at rank 0 and rank 1, with gracefully degrading accuracy
 * toward the middle.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Uses a hierarchy of compactors with varying sampling rates</li>
 *   <li>Higher compactor levels sample more aggressively</li>
 *   <li>Mode determines which extreme has higher accuracy</li>
 *   <li>Provides relative error guarantees that improve near extremes</li>
 * </ul>
 *
 * <p><strong>Accuracy Modes:</strong>
 * <ul>
 *   <li>HIGH_RANK_ACCURACY (0): Better accuracy for high quantiles (p99, p99.9)</li>
 *   <li>LOW_RANK_ACCURACY (1): Better accuracy for low quantiles (p1, p0.1)</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>SLA monitoring requiring extreme percentile accuracy</li>
 *   <li>Tail latency analysis (p99.9, p99.99)</li>
 *   <li>Anomaly detection at distribution extremes</li>
 *   <li>Financial risk metrics (VaR, CVaR)</li>
 * </ul>
 *
 * <p><strong>Compared to Other Sketches:</strong>
 * <ul>
 *   <li>vs KLL: Better accuracy at extremes, slightly more space</li>
 *   <li>vs DDSketch: Rank-based vs value-based error guarantees</li>
 *   <li>vs TDigest: More predictable error bounds</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * // Monitor tail latencies with high accuracy at p99+
 * try (ReqSketch sketch = new ReqSketch(12, ReqSketch.HIGH_RANK_ACCURACY)) {
 *     for (double latency : latencies) {
 *         sketch.update(latency);
 *     }
 *
 *     // High accuracy at these extreme quantiles
 *     System.out.println("p99: " + sketch.quantile(0.99));
 *     System.out.println("p99.9: " + sketch.quantile(0.999));
 *     System.out.println("p99.99: " + sketch.quantile(0.9999));
 * }
 * </pre>
 *
 * @see <a href="https://arxiv.org/abs/2004.01668">ReqSketch Paper</a>
 */
public final class ReqSketch extends NativeSketch implements MergeableSketch<ReqSketch> {

    /**
     * Mode for high accuracy at high ranks (p99, p99.9, etc.).
     * Use this when you care most about upper percentiles.
     */
    public static final int HIGH_RANK_ACCURACY = 0;

    /**
     * Mode for high accuracy at low ranks (p1, p0.1, etc.).
     * Use this when you care most about lower percentiles.
     */
    public static final int LOW_RANK_ACCURACY = 1;

    private final int k;
    private final int mode;

    /**
     * Create a new ReqSketch.
     *
     * @param k the accuracy parameter, controls space/accuracy tradeoff
     *          Higher values = more accurate but more space
     *          Typical values: 6-50, common default is 12
     * @param mode accuracy mode: {@link #HIGH_RANK_ACCURACY} or {@link #LOW_RANK_ACCURACY}
     * @throws IllegalArgumentException if k &lt;= 0 or mode is invalid
     * @throws RuntimeException if native memory allocation fails
     */
    public ReqSketch(int k, int mode) {
        if (k <= 0) {
            throw new IllegalArgumentException("k must be positive, got: " + k);
        }
        if (mode != HIGH_RANK_ACCURACY && mode != LOW_RANK_ACCURACY) {
            throw new IllegalArgumentException(
                    "mode must be HIGH_RANK_ACCURACY (0) or LOW_RANK_ACCURACY (1), got: " + mode);
        }

        this.k = k;
        this.mode = mode;
        this.nativePtr = SketchOxideNative.reqsketch_new(k, mode);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate ReqSketch");
        }
    }

    /**
     * Create a ReqSketch with high rank accuracy mode.
     *
     * Convenience constructor for the common case of monitoring tail percentiles.
     *
     * @param k the accuracy parameter
     * @throws IllegalArgumentException if k &lt;= 0
     */
    public ReqSketch(int k) {
        this(k, HIGH_RANK_ACCURACY);
    }

    /**
     * Private constructor for deserialization.
     */
    private ReqSketch(long nativePtr, int k, int mode) {
        this.nativePtr = nativePtr;
        this.k = k;
        this.mode = mode;
    }

    /**
     * Add a value to the sketch.
     *
     * @param value the value to add
     * @throws IllegalStateException if the sketch has been closed
     */
    public void update(double value) {
        checkAlive();
        SketchOxideNative.reqsketch_update(nativePtr, value);
    }

    /**
     * Get the estimated value at a given quantile.
     *
     * Accuracy is highest near rank 0 and rank 1, degrading toward the middle.
     * The specific extreme with highest accuracy depends on the mode setting.
     *
     * @param q the quantile to query (0 &lt;= q &lt;= 1)
     * @return the estimated value at quantile q
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if q not in [0, 1]
     */
    public double quantile(double q) {
        checkAlive();
        if (q < 0 || q > 1) {
            throw new IllegalArgumentException("quantile must be in [0, 1], got: " + q);
        }
        return SketchOxideNative.reqsketch_quantile(nativePtr, q);
    }

    /**
     * Merge another ReqSketch into this one.
     *
     * Both sketches must have the same k and mode parameters.
     *
     * @param other the sketch to merge into this one
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(ReqSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.k != other.k || this.mode != other.mode) {
            throw new IllegalArgumentException(
                    "Cannot merge ReqSketches with different parameters: " +
                            String.format("(k=%d, mode=%d) vs (k=%d, mode=%d)",
                                    this.k, this.mode, other.k, other.mode));
        }

        SketchOxideNative.reqsketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.reqsketch_serialize(nativePtr);
    }

    /**
     * Deserialize a ReqSketch from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new ReqSketch instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static ReqSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.reqsketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized ReqSketch data");
        }

        return new ReqSketch(ptr, 12, HIGH_RANK_ACCURACY);
    }

    /**
     * Get the k parameter.
     *
     * @return the k value used at creation
     */
    public int getK() {
        return k;
    }

    /**
     * Get the accuracy mode.
     *
     * @return {@link #HIGH_RANK_ACCURACY} or {@link #LOW_RANK_ACCURACY}
     */
    public int getMode() {
        return mode;
    }

    /**
     * Get a human-readable description of the mode.
     *
     * @return "HighRankAccuracy" or "LowRankAccuracy"
     */
    public String getModeDescription() {
        return mode == HIGH_RANK_ACCURACY ? "HighRankAccuracy" : "LowRankAccuracy";
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
            SketchOxideNative.reqsketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("ReqSketch(k=%d, mode=%s)", k, getModeDescription());
        }
        return "ReqSketch(closed)";
    }
}
