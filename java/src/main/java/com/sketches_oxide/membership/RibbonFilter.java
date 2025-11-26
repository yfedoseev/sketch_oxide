package com.sketches_oxide.membership;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * RibbonFilter - Space-Optimal Membership Filter with Build Phase.
 *
 * A highly space-efficient filter based on ribbon (Rapid Incremental Boolean
 * Banded matrix AND target) construction. Achieves approximately 7 bits per key
 * at 1% FPR, making it one of the most space-efficient practical filters.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Uses Gaussian elimination on a banded matrix</li>
 *   <li>Items are inserted, then filter is "built" (finalized)</li>
 *   <li>After building, queries are very fast</li>
 *   <li>Achieves near-information-theoretic space lower bound</li>
 * </ul>
 *
 * <p><strong>Two-Phase Usage:</strong>
 * <ol>
 *   <li><strong>Insert Phase:</strong> Add all items using {@link #insert(byte[])}</li>
 *   <li><strong>Build Phase:</strong> Call {@link #build()} to finalize the filter</li>
 *   <li><strong>Query Phase:</strong> Use {@link #contains(byte[])} for lookups</li>
 * </ol>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Memory-constrained environments</li>
 *   <li>Large-scale key-value stores</li>
 *   <li>Database index structures (LSM trees)</li>
 *   <li>Any application prioritizing space efficiency</li>
 * </ul>
 *
 * <p><strong>Space Efficiency:</strong>
 * <ul>
 *   <li>~7 bits/key at 1% FPR (vs ~10 for Bloom)</li>
 *   <li>~10 bits/key at 0.1% FPR (vs ~14 for Bloom)</li>
 *   <li>~14 bits/key at 0.01% FPR (vs ~20 for Bloom)</li>
 * </ul>
 *
 * <p><strong>Important:</strong> You must call {@link #build()} after inserting all
 * items and before querying. Queries before build() will return incorrect results.
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (RibbonFilter rf = new RibbonFilter(100000, 0.01)) {
 *     // Phase 1: Insert all items
 *     for (String key : allKeys) {
 *         rf.insert(key.getBytes());
 *     }
 *
 *     // Phase 2: Build the filter (REQUIRED)
 *     rf.build();
 *
 *     // Phase 3: Query
 *     if (rf.contains("some-key".getBytes())) {
 *         System.out.println("Key might exist");
 *     }
 * }
 * </pre>
 *
 * @see BloomFilter
 * @see BinaryFuseFilter
 * @see <a href="https://arxiv.org/abs/2103.02515">Ribbon Filter Paper</a>
 */
public final class RibbonFilter extends NativeSketch implements MergeableSketch<RibbonFilter> {

    private final long n;
    private final double fpr;
    private boolean built = false;

    /**
     * Create a new RibbonFilter.
     *
     * @param n expected number of items to insert
     * @param fpr desired false positive rate (0 &lt; fpr &lt; 1)
     * @throws IllegalArgumentException if n &lt;= 0 or fpr not in (0, 1)
     * @throws RuntimeException if native memory allocation fails
     */
    public RibbonFilter(long n, double fpr) {
        if (n <= 0) {
            throw new IllegalArgumentException("n must be positive, got: " + n);
        }
        if (fpr <= 0 || fpr >= 1) {
            throw new IllegalArgumentException("fpr must be in range (0, 1), got: " + fpr);
        }

        this.n = n;
        this.fpr = fpr;
        this.nativePtr = SketchOxideNative.ribbonfilter_new(n, fpr);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate RibbonFilter");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private RibbonFilter(long nativePtr, long n, double fpr, boolean built) {
        this.nativePtr = nativePtr;
        this.n = n;
        this.fpr = fpr;
        this.built = built;
    }

    /**
     * Insert an item into the filter.
     *
     * Items can only be inserted before {@link #build()} is called.
     *
     * @param item the item to insert (as raw bytes)
     * @throws IllegalStateException if the filter has been closed or already built
     * @throws NullPointerException if item is null
     */
    public void insert(byte[] item) {
        checkAlive();
        if (built) {
            throw new IllegalStateException("Cannot insert after build() has been called");
        }
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.ribbonfilter_insert(nativePtr, item);
    }

    /**
     * Insert a string item into the filter.
     *
     * @param item the string to insert
     * @throws IllegalStateException if the filter has been closed or already built
     * @throws NullPointerException if item is null
     */
    public void insert(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        insert(item.getBytes());
    }

    /**
     * Build (finalize) the filter.
     *
     * This method must be called after inserting all items and before querying.
     * After calling build(), no more items can be inserted, but the filter
     * becomes queryable.
     *
     * <p><strong>Note:</strong> Building is a computationally intensive operation
     * that performs Gaussian elimination. For very large filters, this may take
     * noticeable time.
     *
     * @throws IllegalStateException if the filter has been closed or already built
     */
    public void build() {
        checkAlive();
        if (built) {
            throw new IllegalStateException("Filter has already been built");
        }
        SketchOxideNative.ribbonfilter_build(nativePtr);
        built = true;
    }

    /**
     * Check if an item might be in the filter.
     *
     * Returns true if the item might be in the set (with false positive rate = fpr).
     * Returns false if the item is definitely not in the set.
     *
     * @param item the item to check (as raw bytes)
     * @return true if the item might be in the set, false if definitely not
     * @throws IllegalStateException if the filter has been closed or not yet built
     * @throws NullPointerException if item is null
     */
    public boolean contains(byte[] item) {
        checkAlive();
        if (!built) {
            throw new IllegalStateException("Must call build() before querying");
        }
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.ribbonfilter_contains(nativePtr, item);
    }

    /**
     * Check if a string item might be in the filter.
     *
     * @param item the string to check
     * @return true if the item might be in the set, false if definitely not
     * @throws IllegalStateException if the filter has been closed or not yet built
     * @throws NullPointerException if item is null
     */
    public boolean contains(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return contains(item.getBytes());
    }

    /**
     * Check if the filter has been built.
     *
     * @return true if build() has been called, false otherwise
     */
    public boolean isBuilt() {
        return built;
    }

    /**
     * Merge another RibbonFilter into this one.
     *
     * <p><strong>Note:</strong> RibbonFilter does not support merge operations
     * due to its construction algorithm. To combine sets, create a new filter
     * with all items.
     *
     * @param other the filter to merge
     * @throws UnsupportedOperationException always
     */
    @Override
    public void merge(RibbonFilter other) {
        throw new UnsupportedOperationException(
                "RibbonFilter does not support merge operations. " +
                        "Create a new filter with combined items instead.");
    }

    /**
     * Serialize the filter to binary format.
     *
     * @return binary representation of the filter
     * @throws IllegalStateException if the filter has been closed or not yet built
     */
    public byte[] serialize() {
        checkAlive();
        if (!built) {
            throw new IllegalStateException("Must call build() before serializing");
        }
        return SketchOxideNative.ribbonfilter_serialize(nativePtr);
    }

    /**
     * Deserialize a RibbonFilter from binary format.
     *
     * The deserialized filter is already in built state and ready for queries.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new RibbonFilter instance (already built)
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static RibbonFilter deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.ribbonfilter_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized RibbonFilter data");
        }

        // Deserialized filters are already built
        return new RibbonFilter(ptr, 100, 0.01, true);
    }

    /**
     * Get the expected number of items.
     *
     * @return the n parameter used at creation
     */
    public long getN() {
        return n;
    }

    /**
     * Get the target false positive rate.
     *
     * @return the fpr parameter used at creation
     */
    public double getFpr() {
        return fpr;
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
            SketchOxideNative.ribbonfilter_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("RibbonFilter(n=%d, fpr=%.6f, built=%s)",
                    n, fpr, built);
        }
        return "RibbonFilter(closed)";
    }
}
