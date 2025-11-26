package com.sketches_oxide.similarity;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * SimHash - Near-Duplicate Detection via Locality-Sensitive Hashing.
 *
 * Computes a compact fingerprint for documents/data that preserves similarity.
 * Similar items will have similar fingerprints (small Hamming distance).
 * Developed by Moses Charikar at Google for web-scale duplicate detection.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Hashes each feature and maintains a running weighted sum</li>
 *   <li>Final fingerprint is the sign of each dimension</li>
 *   <li>Uses 64-bit fingerprints for compact representation</li>
 *   <li>Hamming distance correlates with cosine distance</li>
 * </ul>
 *
 * <p><strong>Properties:</strong>
 * <ul>
 *   <li>Space: O(1) - just 64 bits</li>
 *   <li>Similarity: Cosine similarity approximation</li>
 *   <li>Hamming distance of k bits indicates angle of ~k*pi/64</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Near-duplicate web page detection</li>
 *   <li>Plagiarism detection</li>
 *   <li>Image similarity (via feature vectors)</li>
 *   <li>Clustering at scale</li>
 *   <li>Content deduplication</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (SimHash sh1 = new SimHash();
 *      SimHash sh2 = new SimHash()) {
 *
 *     // Add features to document 1
 *     sh1.update("hello".getBytes());
 *     sh1.update("world".getBytes());
 *     sh1.update("java".getBytes());
 *
 *     // Add features to document 2 (similar)
 *     sh2.update("hello".getBytes());
 *     sh2.update("world".getBytes());
 *     sh2.update("python".getBytes());
 *
 *     // Compare fingerprints
 *     int distance = sh1.hammingDistance(sh2);
 *     double similarity = sh1.similarity(sh2);
 *
 *     System.out.printf("Hamming distance: %d bits%n", distance);
 *     System.out.printf("Similarity: %.2f%n", similarity);
 *
 *     // Get raw fingerprint for storage
 *     long fp = sh1.fingerprint();
 *     System.out.printf("Fingerprint: 0x%016X%n", fp);
 * }
 * </pre>
 *
 * @see MinHash
 * @see <a href="https://en.wikipedia.org/wiki/SimHash">SimHash</a>
 */
public final class SimHash extends NativeSketch {

    /**
     * Number of bits in the fingerprint.
     */
    public static final int FINGERPRINT_BITS = 64;

    /**
     * Create a new SimHash instance.
     *
     * SimHash uses a fixed 64-bit fingerprint and requires no parameters.
     */
    public SimHash() {
        this.nativePtr = SketchOxideNative.simhash_new();

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate SimHash");
        }
    }

    /**
     * Add a feature to the document.
     *
     * Each feature contributes to the final fingerprint based on its hash.
     * Order of updates does not matter.
     *
     * @param item the feature to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.simhash_update(nativePtr, item);
    }

    /**
     * Add a string feature to the document.
     *
     * @param item the string feature to add
     */
    public void update(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        update(item.getBytes());
    }

    /**
     * Get the 64-bit fingerprint.
     *
     * The fingerprint is a compact representation of all added features.
     * Can be stored and compared later without keeping the SimHash object.
     *
     * @return the 64-bit fingerprint
     * @throws IllegalStateException if the sketch has been closed
     */
    public long fingerprint() {
        checkAlive();
        return SketchOxideNative.simhash_fingerprint(nativePtr);
    }

    /**
     * Compute the Hamming distance to another SimHash.
     *
     * The Hamming distance is the number of bits that differ between
     * the two fingerprints. Range: [0, 64].
     *
     * <p>Interpretation:
     * <ul>
     *   <li>0-3 bits: Very similar (near-duplicates)</li>
     *   <li>4-10 bits: Somewhat similar</li>
     *   <li>10+ bits: Likely different</li>
     * </ul>
     *
     * @param other the other SimHash to compare with
     * @return Hamming distance in range [0, 64]
     * @throws IllegalStateException if either sketch is closed
     * @throws NullPointerException if other is null
     */
    public int hammingDistance(SimHash other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        return SketchOxideNative.simhash_hammingdistance(this.nativePtr, other.nativePtr);
    }

    /**
     * Compute the similarity with another SimHash.
     *
     * Similarity = 1 - (hammingDistance / 64), ranging from 0 to 1.
     * This approximates the cosine similarity of the original feature vectors.
     *
     * @param other the other SimHash to compare with
     * @return similarity in range [0, 1]
     * @throws IllegalStateException if either sketch is closed
     * @throws NullPointerException if other is null
     */
    public double similarity(SimHash other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        return SketchOxideNative.simhash_similarity(this.nativePtr, other.nativePtr);
    }

    /**
     * Compute Hamming distance between two fingerprints.
     *
     * Static utility method for comparing stored fingerprints
     * without needing SimHash objects.
     *
     * @param fp1 first fingerprint
     * @param fp2 second fingerprint
     * @return Hamming distance in range [0, 64]
     */
    public static int hammingDistance(long fp1, long fp2) {
        return Long.bitCount(fp1 ^ fp2);
    }

    /**
     * Compute similarity between two fingerprints.
     *
     * Static utility method for comparing stored fingerprints.
     *
     * @param fp1 first fingerprint
     * @param fp2 second fingerprint
     * @return similarity in range [0, 1]
     */
    public static double similarity(long fp1, long fp2) {
        int distance = hammingDistance(fp1, fp2);
        return 1.0 - (distance / (double) FINGERPRINT_BITS);
    }

    /**
     * Check if two fingerprints are near-duplicates.
     *
     * @param fp1 first fingerprint
     * @param fp2 second fingerprint
     * @param threshold maximum Hamming distance to consider as near-duplicate
     * @return true if the fingerprints are within the threshold
     */
    public static boolean isNearDuplicate(long fp1, long fp2, int threshold) {
        return hammingDistance(fp1, fp2) <= threshold;
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.simhash_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new SimHash instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static SimHash deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.simhash_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized SimHash data");
        }

        SimHash sh = new SimHash();
        SketchOxideNative.simhash_free(sh.nativePtr);
        sh.nativePtr = ptr;
        return sh;
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
            SketchOxideNative.simhash_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("SimHash(fingerprint=0x%016X)", fingerprint());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "SimHash(closed)";
    }
}
