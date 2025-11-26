package com.sketches_oxide.membership;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * BinaryFuseFilter - Ultra-Efficient Static Membership Testing.
 *
 * A state-of-the-art static filter that achieves approximately 75% better space
 * efficiency than Bloom filters while maintaining fast query times. Binary fuse
 * filters are constructed from a known set of items and provide the most compact
 * representation for static membership testing.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Based on XOR filters with fuse graph construction</li>
 *   <li>Uses three hash functions mapping to segments</li>
 *   <li>Fingerprints are XORed at construction time</li>
 *   <li>Query: XOR fingerprints from three positions, compare to stored value</li>
 *   <li>Achieves near-optimal space: ~9 bits per key at 1% FPR</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Static dictionaries (spell checkers, blocklists)</li>
 *   <li>Database key existence checks</li>
 *   <li>Network routing tables</li>
 *   <li>Any scenario with a known, fixed set of items</li>
 * </ul>
 *
 * <p><strong>Advantages:</strong>
 * <ul>
 *   <li>~75% more space-efficient than Bloom filters</li>
 *   <li>Faster queries than most alternatives</li>
 *   <li>Perfect for read-heavy workloads</li>
 *   <li>Predictable query time (no hash collisions to resolve)</li>
 * </ul>
 *
 * <p><strong>Limitations:</strong>
 * <ul>
 *   <li>Static: cannot add or remove items after construction</li>
 *   <li>Construction requires all items upfront</li>
 *   <li>Construction can fail on adversarial inputs (rare)</li>
 *   <li>Works with integer items (hash your data first)</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * // Create filter from known set of item IDs
 * long[] itemIds = {12345L, 67890L, 11111L, 22222L};
 * try (BinaryFuseFilter filter = new BinaryFuseFilter(itemIds, 8)) {
 *     // Fast membership queries
 *     System.out.println(filter.contains(12345L)); // true
 *     System.out.println(filter.contains(99999L)); // false (probably)
 * }
 * </pre>
 *
 * @see BloomFilter
 * @see <a href="https://arxiv.org/abs/2201.01174">Binary Fuse Filters Paper</a>
 */
public final class BinaryFuseFilter extends NativeSketch implements MergeableSketch<BinaryFuseFilter> {

    private final int bitsPerEntry;
    private final long itemCount;

    /**
     * Create a new BinaryFuseFilter from a set of items.
     *
     * The filter is constructed immediately from the provided items.
     * After construction, no items can be added or removed.
     *
     * @param items array of long values representing the items to store
     *              (typically hashes of your actual data)
     * @param bitsPerEntry number of bits per fingerprint (8, 16, or 32 typical)
     *                     Higher values = lower FPR but more space
     *                     8 bits: ~0.4% FPR, 16 bits: ~0.0015% FPR
     * @throws IllegalArgumentException if items is empty or bitsPerEntry invalid
     * @throws NullPointerException if items is null
     * @throws RuntimeException if native memory allocation or construction fails
     */
    public BinaryFuseFilter(long[] items, int bitsPerEntry) {
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        if (items.length == 0) {
            throw new IllegalArgumentException("items array cannot be empty");
        }
        if (bitsPerEntry <= 0 || bitsPerEntry > 32) {
            throw new IllegalArgumentException(
                    "bitsPerEntry must be in range [1, 32], got: " + bitsPerEntry);
        }

        this.bitsPerEntry = bitsPerEntry;
        this.itemCount = items.length;
        this.nativePtr = SketchOxideNative.binaryfusefilter_new(items, bitsPerEntry);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to construct BinaryFuseFilter - construction may have failed");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private BinaryFuseFilter(long nativePtr, int bitsPerEntry, long itemCount) {
        this.nativePtr = nativePtr;
        this.bitsPerEntry = bitsPerEntry;
        this.itemCount = itemCount;
    }

    /**
     * Check if an item might be in the filter.
     *
     * Returns true if the item might be in the set (with FPR based on bitsPerEntry).
     * Returns false if the item is definitely not in the set.
     *
     * @param item the item to check (as a long value, typically a hash)
     * @return true if the item might be in the set, false if definitely not
     * @throws IllegalStateException if the filter has been closed
     */
    public boolean contains(long item) {
        checkAlive();
        return SketchOxideNative.binaryfusefilter_contains(nativePtr, item);
    }

    /**
     * Merge another BinaryFuseFilter into this one.
     *
     * <p><strong>Note:</strong> BinaryFuseFilter does not support merge operations
     * as it is a static data structure. To combine sets, reconstruct a new filter
     * with the combined item set.
     *
     * @param other the filter to merge
     * @throws UnsupportedOperationException always
     */
    @Override
    public void merge(BinaryFuseFilter other) {
        throw new UnsupportedOperationException(
                "BinaryFuseFilter is static and does not support merge operations. " +
                        "Reconstruct with combined item set instead.");
    }

    /**
     * Serialize the filter to binary format.
     *
     * @return binary representation of the filter
     * @throws IllegalStateException if the filter has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.binaryfusefilter_serialize(nativePtr);
    }

    /**
     * Deserialize a BinaryFuseFilter from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new BinaryFuseFilter instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static BinaryFuseFilter deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.binaryfusefilter_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized BinaryFuseFilter data");
        }

        return new BinaryFuseFilter(ptr, 8, 0);
    }

    /**
     * Get the bits per entry setting.
     *
     * @return the number of bits used per fingerprint
     */
    public int getBitsPerEntry() {
        return bitsPerEntry;
    }

    /**
     * Get the number of items in the filter.
     *
     * @return the number of items used to construct this filter
     */
    public long getItemCount() {
        return itemCount;
    }

    /**
     * Get the approximate false positive rate.
     *
     * @return estimated FPR based on bits per entry
     */
    public double getApproximateFpr() {
        // FPR is approximately 2^(-bitsPerEntry)
        return Math.pow(2, -bitsPerEntry);
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
            SketchOxideNative.binaryfusefilter_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("BinaryFuseFilter(items=%d, bitsPerEntry=%d, approxFpr=%.6f)",
                    itemCount, bitsPerEntry, getApproximateFpr());
        }
        return "BinaryFuseFilter(closed)";
    }
}
