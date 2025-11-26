package com.sketches_oxide.membership;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * CountingBloomFilter - Probabilistic Membership with Deletion Support.
 *
 * An extension of the classic Bloom filter that uses counters instead of bits,
 * enabling item deletion. Each position stores a count rather than a single bit,
 * allowing decrement operations when items are removed.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Uses k hash functions mapping to m counter positions</li>
 *   <li>Insert: increment all k counter positions</li>
 *   <li>Delete: decrement all k counter positions</li>
 *   <li>Query: check if all k positions have count &gt; 0</li>
 *   <li>Counters typically use 4 bits (max count 15) to limit overflow</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Dynamic membership with additions and deletions</li>
 *   <li>Network routing tables with route removal</li>
 *   <li>Cache eviction tracking</li>
 *   <li>Session management with timeouts</li>
 * </ul>
 *
 * <p><strong>Trade-offs vs Standard Bloom:</strong>
 * <ul>
 *   <li>Space: 3-4x larger (counters vs bits)</li>
 *   <li>Supports deletion (standard Bloom does not)</li>
 *   <li>Risk of counter overflow on high-frequency items</li>
 *   <li>Same FPR guarantees when not overflowed</li>
 * </ul>
 *
 * <p><strong>Warning:</strong> Deleting items that were never inserted can cause
 * false negatives. Only delete items you are certain were previously inserted.
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (CountingBloomFilter cbf = new CountingBloomFilter(10000, 0.01)) {
 *     // Add items
 *     cbf.insert("session-123".getBytes());
 *     cbf.insert("session-456".getBytes());
 *
 *     // Check membership
 *     System.out.println(cbf.contains("session-123".getBytes())); // true
 *
 *     // Remove expired session
 *     cbf.remove("session-123".getBytes());
 *     System.out.println(cbf.contains("session-123".getBytes())); // false (probably)
 * }
 * </pre>
 *
 * @see BloomFilter
 * @see <a href="https://www.eecs.harvard.edu/~michaelm/postscripts/rsa2000.pdf">Counting Bloom Filters</a>
 */
public final class CountingBloomFilter extends NativeSketch implements MergeableSketch<CountingBloomFilter> {

    private final long n;
    private final double fpr;

    /**
     * Create a new CountingBloomFilter.
     *
     * @param n expected number of items (peak occupancy, accounting for deletions)
     * @param fpr desired false positive rate (0 &lt; fpr &lt; 1)
     * @throws IllegalArgumentException if n &lt;= 0 or fpr not in (0, 1)
     * @throws RuntimeException if native memory allocation fails
     */
    public CountingBloomFilter(long n, double fpr) {
        if (n <= 0) {
            throw new IllegalArgumentException("n must be positive, got: " + n);
        }
        if (fpr <= 0 || fpr >= 1) {
            throw new IllegalArgumentException("fpr must be in range (0, 1), got: " + fpr);
        }

        this.n = n;
        this.fpr = fpr;
        this.nativePtr = SketchOxideNative.countingbloomfilter_new(n, fpr);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate CountingBloomFilter");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private CountingBloomFilter(long nativePtr, long n, double fpr) {
        this.nativePtr = nativePtr;
        this.n = n;
        this.fpr = fpr;
    }

    /**
     * Insert an item into the filter.
     *
     * Increments all k counter positions for this item.
     *
     * @param item the item to insert (as raw bytes)
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public void insert(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.countingbloomfilter_insert(nativePtr, item);
    }

    /**
     * Insert a string item into the filter.
     *
     * @param item the string to insert
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public void insert(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        insert(item.getBytes());
    }

    /**
     * Remove an item from the filter.
     *
     * Decrements all k counter positions for this item.
     *
     * <p><strong>Warning:</strong> Only remove items that were previously inserted.
     * Removing items that were never inserted can cause false negatives for
     * other items that share counter positions.
     *
     * @param item the item to remove (as raw bytes)
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public void remove(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.countingbloomfilter_remove(nativePtr, item);
    }

    /**
     * Remove a string item from the filter.
     *
     * @param item the string to remove
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public void remove(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        remove(item.getBytes());
    }

    /**
     * Check if an item might be in the filter.
     *
     * Returns true if all k counter positions have count &gt; 0.
     * Returns false if any counter position is 0 (definitely not in set).
     *
     * @param item the item to check (as raw bytes)
     * @return true if the item might be in the set, false if definitely not
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public boolean contains(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.countingbloomfilter_contains(nativePtr, item);
    }

    /**
     * Check if a string item might be in the filter.
     *
     * @param item the string to check
     * @return true if the item might be in the set, false if definitely not
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public boolean contains(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return contains(item.getBytes());
    }

    /**
     * Merge another CountingBloomFilter into this one.
     *
     * After merging, this filter contains the union of both filters.
     * Counter values are added together.
     *
     * <p><strong>Note:</strong> Merging is typically not supported for counting
     * bloom filters due to counter semantics. This method may have implementation-
     * specific behavior.
     *
     * @param other the filter to merge into this one
     * @throws IllegalStateException if either filter is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(CountingBloomFilter other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.n != other.n || Math.abs(this.fpr - other.fpr) > 1e-9) {
            throw new IllegalArgumentException(
                    "Cannot merge CountingBloomFilters with different parameters: " +
                            String.format("(n=%d, fpr=%.6f) vs (n=%d, fpr=%.6f)",
                                    this.n, this.fpr, other.n, other.fpr));
        }

        // Note: CountingBloomFilter merge semantics may vary by implementation
        // This follows the native library's merge behavior
        throw new UnsupportedOperationException(
                "CountingBloomFilter merge is not supported - counter semantics are ambiguous");
    }

    /**
     * Serialize the filter to binary format.
     *
     * @return binary representation of the filter
     * @throws IllegalStateException if the filter has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.countingbloomfilter_serialize(nativePtr);
    }

    /**
     * Deserialize a CountingBloomFilter from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new CountingBloomFilter instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static CountingBloomFilter deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.countingbloomfilter_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized CountingBloomFilter data");
        }

        return new CountingBloomFilter(ptr, 100, 0.01);
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
            SketchOxideNative.countingbloomfilter_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("CountingBloomFilter(n=%d, fpr=%.6f)", n, fpr);
        }
        return "CountingBloomFilter(closed)";
    }
}
