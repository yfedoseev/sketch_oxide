package com.sketches_oxide.similarity;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * MinHash - Jaccard Similarity Estimation.
 *
 * Efficiently estimates the Jaccard similarity between sets using locality-sensitive hashing.
 * The Jaccard similarity J(A,B) = |A intersection B| / |A union B| measures set overlap.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Maintains k minimum hash values across all elements</li>
 *   <li>Similarity estimated by counting matching minimums</li>
 *   <li>Unbiased estimator with variance O(1/k)</li>
 *   <li>Supports union operation via merge</li>
 * </ul>
 *
 * <p><strong>Properties:</strong>
 * <ul>
 *   <li>Error: Standard error = 1/sqrt(numPerm)</li>
 *   <li>Space: O(numPerm) hash values</li>
 *   <li>Time: O(numPerm) per update and query</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Near-duplicate document detection</li>
 *   <li>Plagiarism detection</li>
 *   <li>Clustering similar items</li>
 *   <li>Recommendation systems (user/item similarity)</li>
 *   <li>DNA sequence comparison</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (MinHash mh1 = new MinHash(128);
 *      MinHash mh2 = new MinHash(128)) {
 *
 *     // Add elements to set 1
 *     mh1.update("apple".getBytes());
 *     mh1.update("banana".getBytes());
 *     mh1.update("cherry".getBytes());
 *
 *     // Add elements to set 2
 *     mh2.update("banana".getBytes());
 *     mh2.update("cherry".getBytes());
 *     mh2.update("date".getBytes());
 *
 *     // Estimate Jaccard similarity
 *     double similarity = mh1.jaccardSimilarity(mh2);
 *     System.out.printf("Jaccard similarity: %.2f%n", similarity);
 *     // Expected: ~0.5 (2 common out of 4 total)
 * }
 * </pre>
 *
 * @see SimHash
 * @see <a href="https://en.wikipedia.org/wiki/MinHash">MinHash</a>
 */
public final class MinHash extends NativeSketch implements MergeableSketch<MinHash> {

    private final long numPerm;

    /**
     * Create a new MinHash sketch.
     *
     * @param numPerm the number of permutations (hash functions)
     *                More permutations = higher accuracy but more memory
     *                Standard error = 1/sqrt(numPerm)
     *                Typical values: 64, 128, 256, 512
     * @throws IllegalArgumentException if numPerm &lt;= 0
     */
    public MinHash(long numPerm) {
        if (numPerm <= 0) {
            throw new IllegalArgumentException("numPerm must be positive, got: " + numPerm);
        }

        this.numPerm = numPerm;
        this.nativePtr = SketchOxideNative.minhash_new(numPerm);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate MinHash");
        }
    }

    /**
     * Add an element to the set.
     *
     * Each element's hash is computed with all permutations,
     * and minimum values are retained.
     *
     * @param item the element to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.minhash_update(nativePtr, item);
    }

    /**
     * Add a string element to the set.
     *
     * @param item the string element to add
     */
    public void update(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        update(item.getBytes());
    }

    /**
     * Estimate the Jaccard similarity with another MinHash.
     *
     * The Jaccard similarity J(A,B) = |A intersection B| / |A union B|
     * ranges from 0 (disjoint sets) to 1 (identical sets).
     *
     * @param other the other MinHash to compare with
     * @return estimated Jaccard similarity in range [0, 1]
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if numPerm values don't match
     * @throws NullPointerException if other is null
     */
    public double jaccardSimilarity(MinHash other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.numPerm != other.numPerm) {
            throw new IllegalArgumentException(
                    "Cannot compute similarity between MinHash with different numPerm: " +
                            this.numPerm + " vs " + other.numPerm);
        }

        return SketchOxideNative.minhash_jaccardsimilarity(this.nativePtr, other.nativePtr);
    }

    /**
     * Merge another MinHash into this one.
     *
     * After merging, this MinHash represents the union of both sets.
     * Both sketches must have the same numPerm.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if numPerm values don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(MinHash other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.numPerm != other.numPerm) {
            throw new IllegalArgumentException(
                    "Cannot merge MinHash with different numPerm: " +
                            this.numPerm + " vs " + other.numPerm);
        }

        SketchOxideNative.minhash_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the number of permutations.
     *
     * @return the numPerm parameter
     */
    public long numPerm() {
        return numPerm;
    }

    /**
     * Get the expected standard error.
     *
     * @return 1/sqrt(numPerm)
     */
    public double getStandardError() {
        return 1.0 / Math.sqrt(numPerm);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.minhash_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new MinHash instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static MinHash deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.minhash_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized MinHash data");
        }

        // Create with dummy value, then replace pointer
        MinHash mh = new MinHash(128);
        SketchOxideNative.minhash_free(mh.nativePtr);
        mh.nativePtr = ptr;
        return mh;
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
            SketchOxideNative.minhash_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("MinHash(numPerm=%d, stdError=%.4f)",
                        numPerm, getStandardError());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "MinHash(closed)";
    }
}
