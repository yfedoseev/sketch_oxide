package com.sketches_oxide.cardinality;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * ThetaSketch - Set Operations with Jaccard Similarity.
 *
 * <p>ThetaSketch provides cardinality estimation with efficient set operations
 * (union, intersection, difference) and Jaccard similarity computation. It uses
 * a theta threshold approach that enables exact set algebra on sketches.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>ThetaSketch maintains a set of hash values below a dynamically adjusted
 * threshold (theta). This design enables:
 * <ul>
 *   <li>Union: Simply merge hash sets and adjust theta</li>
 *   <li>Intersection: Keep hashes present in both sketches</li>
 *   <li>Difference (A not B): Keep hashes in A but not in B</li>
 *   <li>Jaccard similarity: |A intersect B| / |A union B|</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Computing overlap between two datasets</li>
 *   <li>Finding unique elements across multiple sets</li>
 *   <li>Measuring similarity between user segments</li>
 *   <li>A/B test audience overlap analysis</li>
 *   <li>Deduplication across data sources</li>
 * </ul>
 *
 * <p><strong>Performance:</strong>
 * <ul>
 *   <li>Space: O(2^lgK) bytes</li>
 *   <li>Time: O(1) per update, O(k) for set operations</li>
 *   <li>Error: ~1/sqrt(k) where k = 2^lgK</li>
 * </ul>
 *
 * <p><strong>Set Operations Example:</strong>
 * <pre>
 * try (ThetaSketch setA = new ThetaSketch(12);
 *      ThetaSketch setB = new ThetaSketch(12)) {
 *
 *     // Populate sets
 *     for (String user : groupA) setA.update(user);
 *     for (String user : groupB) setB.update(user);
 *
 *     // Compute Jaccard similarity
 *     double similarity = setA.jaccardSimilarity(setB);
 *     System.out.println("Overlap similarity: " + similarity);
 *
 *     // Compute intersection
 *     try (ThetaSketch intersection = setA.intersect(setB)) {
 *         System.out.println("Common users: " + intersection.estimateLong());
 *     }
 *
 *     // Compute difference (users in A but not B)
 *     try (ThetaSketch diff = setA.aNotB(setB)) {
 *         System.out.println("Users only in A: " + diff.estimateLong());
 *     }
 * }
 * </pre>
 *
 * @see <a href="https://datasketches.apache.org/docs/Theta/ThetaSketchFramework.html">Theta Sketch Framework</a>
 * @see HyperLogLog
 * @see CpcSketch
 */
public final class ThetaSketch extends NativeSketch implements MergeableSketch<ThetaSketch> {

    /** Minimum allowed lgK value. */
    public static final int MIN_LGK = 4;

    /** Maximum allowed lgK value. */
    public static final int MAX_LGK = 26;

    /** Default lgK value providing good balance. */
    public static final int DEFAULT_LGK = 12;

    /** The lgK parameter used to create this sketch. */
    private final int lgK;

    /**
     * Create a new ThetaSketch with default lgK (12).
     *
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public ThetaSketch() {
        this(DEFAULT_LGK);
    }

    /**
     * Create a new ThetaSketch.
     *
     * @param lgK the log2 of nominal entries (4-26)
     *            Determines sketch size: k = 2^lgK
     *            Higher values provide better accuracy but use more memory.
     * @throws IllegalArgumentException if lgK is not in range [4, 26]
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public ThetaSketch(int lgK) {
        if (lgK < MIN_LGK || lgK > MAX_LGK) {
            throw new IllegalArgumentException(
                    "lgK must be between " + MIN_LGK + " and " + MAX_LGK +
                            ", got: " + lgK);
        }

        this.lgK = lgK;
        this.nativePtr = SketchOxideNative.thetasketch_new(lgK);
        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate ThetaSketch");
        }
    }

    /**
     * Private constructor for internal use (set operations).
     *
     * @param nativePtr the native pointer
     * @param lgK the lgK value
     */
    private ThetaSketch(long nativePtr, int lgK) {
        this.nativePtr = nativePtr;
        this.lgK = lgK;
    }

    /**
     * Add an item to the sketch.
     *
     * @param item the byte array data to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.thetasketch_update(nativePtr, item);
    }

    /**
     * Add a string item to the sketch.
     *
     * @param item the string to add
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
     * Get the estimated cardinality.
     *
     * @return estimated number of unique items as a double
     * @throws IllegalStateException if the sketch has been closed
     */
    public double estimate() {
        checkAlive();
        return SketchOxideNative.thetasketch_estimate(nativePtr);
    }

    /**
     * Get the estimated cardinality as a long.
     *
     * @return estimated number of unique items, rounded to nearest long
     * @throws IllegalStateException if the sketch has been closed
     */
    public long estimateLong() {
        return Math.round(estimate());
    }

    /**
     * Get the theta value.
     *
     * <p>Theta is the threshold below which hash values are retained.
     * It ranges from 0.0 to 1.0, where 1.0 means all hashes are kept
     * (sketch is not yet at capacity).
     *
     * <p>The cardinality estimate is: retainedHashCount / theta
     *
     * @return the current theta value (0.0 to 1.0)
     * @throws IllegalStateException if the sketch has been closed
     */
    public double theta() {
        checkAlive();
        return SketchOxideNative.thetasketch_theta(nativePtr);
    }

    /**
     * Get the number of hash values currently retained.
     *
     * <p>This is the actual number of unique hashes stored in the sketch.
     * Combined with theta(), this determines the cardinality estimate.
     *
     * @return the number of retained hash values
     * @throws IllegalStateException if the sketch has been closed
     */
    public long retainedHashCount() {
        checkAlive();
        return SketchOxideNative.thetasketch_retainedHashCount(nativePtr);
    }

    /**
     * Merge another ThetaSketch into this one (union operation).
     *
     * <p>After merging, this sketch represents the union of both sketches.
     * Both sketches must have the same lgK value.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if lgK values don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(ThetaSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.lgK != other.lgK) {
            throw new IllegalArgumentException(
                    "Cannot merge ThetaSketches with different lgK values: " +
                            this.lgK + " vs " + other.lgK);
        }

        SketchOxideNative.thetasketch_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Compute the intersection of this sketch and another.
     *
     * <p>Returns a new sketch representing items present in both sketches.
     * Both sketches must have the same lgK value.
     *
     * @param other the other sketch
     * @return a new ThetaSketch representing the intersection
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if lgK values don't match
     * @throws NullPointerException if other is null
     */
    public ThetaSketch intersect(ThetaSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.lgK != other.lgK) {
            throw new IllegalArgumentException(
                    "Cannot intersect ThetaSketches with different lgK values: " +
                            this.lgK + " vs " + other.lgK);
        }

        long resultPtr = SketchOxideNative.thetasketch_intersect(this.nativePtr, other.nativePtr);
        if (resultPtr == 0) {
            throw new OutOfMemoryException("Failed to allocate intersection result");
        }
        return new ThetaSketch(resultPtr, this.lgK);
    }

    /**
     * Compute the set difference (A not B).
     *
     * <p>Returns a new sketch representing items in this sketch but not in the other.
     * Both sketches must have the same lgK value.
     *
     * @param other the sketch to subtract
     * @return a new ThetaSketch representing (this - other)
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if lgK values don't match
     * @throws NullPointerException if other is null
     */
    public ThetaSketch aNotB(ThetaSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.lgK != other.lgK) {
            throw new IllegalArgumentException(
                    "Cannot compute A-not-B for ThetaSketches with different lgK values: " +
                            this.lgK + " vs " + other.lgK);
        }

        long resultPtr = SketchOxideNative.thetasketch_aNotB(this.nativePtr, other.nativePtr);
        if (resultPtr == 0) {
            throw new OutOfMemoryException("Failed to allocate A-not-B result");
        }
        return new ThetaSketch(resultPtr, this.lgK);
    }

    /**
     * Compute the Jaccard similarity between this sketch and another.
     *
     * <p>Jaccard similarity is defined as |A intersect B| / |A union B|,
     * representing the overlap ratio between two sets.
     *
     * <p>Results range from 0.0 (no overlap) to 1.0 (identical sets).
     *
     * @param other the other sketch
     * @return the Jaccard similarity coefficient (0.0 to 1.0)
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if lgK values don't match
     * @throws NullPointerException if other is null
     */
    public double jaccardSimilarity(ThetaSketch other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.lgK != other.lgK) {
            throw new IllegalArgumentException(
                    "Cannot compute Jaccard similarity for ThetaSketches with different lgK values: " +
                            this.lgK + " vs " + other.lgK);
        }

        return SketchOxideNative.thetasketch_jaccardSimilarity(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the lgK parameter.
     *
     * @return the lgK parameter used at creation
     */
    public int lgK() {
        return lgK;
    }

    /**
     * Get the k parameter (nominal entries).
     *
     * @return k = 2^lgK
     */
    public int k() {
        return 1 << lgK;
    }

    /**
     * Calculate the theoretical standard error for this sketch's lgK.
     *
     * @return the theoretical relative standard error
     */
    public double theoreticalError() {
        return 1.0 / Math.sqrt(k());
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.thetasketch_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new ThetaSketch instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static ThetaSketch deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.thetasketch_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized ThetaSketch data");
        }

        // Create a dummy object and replace its native pointer
        ThetaSketch ts = new ThetaSketch(DEFAULT_LGK);
        SketchOxideNative.thetasketch_free(ts.nativePtr);
        ts.nativePtr = ptr;
        return ts;
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
            SketchOxideNative.thetasketch_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("ThetaSketch(lgK=%d, theta=%.4f, retained=%d, estimate=%.0f)",
                        lgK, theta(), retainedHashCount(), estimate());
            }
        } catch (IllegalStateException e) {
            // Sketch is closed
        }
        return "ThetaSketch(closed)";
    }
}
