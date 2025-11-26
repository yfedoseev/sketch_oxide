package com.sketches_oxide.membership;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * StableBloomFilter - Bounded FPR for Unbounded Streams.
 *
 * A self-stabilizing Bloom filter variant designed for streaming data with
 * unbounded cardinality. Unlike standard Bloom filters that degrade as more
 * items are added, Stable Bloom Filters maintain a bounded false positive rate
 * by randomly evicting old information.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Uses counters instead of bits (like Counting Bloom)</li>
 *   <li>On each insert, randomly decrements P counters (eviction)</li>
 *   <li>Inserts by setting counters to maximum value</li>
 *   <li>This creates a "sliding window" effect on membership</li>
 *   <li>Old items naturally age out, preventing filter saturation</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Network duplicate detection in continuous streams</li>
 *   <li>Web crawler URL deduplication</li>
 *   <li>Real-time event deduplication</li>
 *   <li>Any scenario with unbounded, continuous data</li>
 * </ul>
 *
 * <p><strong>Properties:</strong>
 * <ul>
 *   <li>Bounded FPR: stabilizes to target rate regardless of input size</li>
 *   <li>Bounded FNR: recent items are reliably detected</li>
 *   <li>Memory-bounded: fixed space for infinite streams</li>
 *   <li>No explicit deletion needed: old items age out</li>
 * </ul>
 *
 * <p><strong>Trade-offs:</strong>
 * <ul>
 *   <li>Higher FPR than standard Bloom for small sets</li>
 *   <li>Cannot query "historical" items (they may have aged out)</li>
 *   <li>False negatives possible for old items</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * // Create filter for continuous stream processing
 * try (StableBloomFilter sbf = new StableBloomFilter(1024 * 1024, 10, 0.01)) {
 *     // Process infinite stream
 *     while (stream.hasNext()) {
 *         byte[] item = stream.next();
 *
 *         if (!sbf.contains(item)) {
 *             // First time seeing this item (probably)
 *             processNewItem(item);
 *         }
 *
 *         sbf.insert(item);
 *     }
 * }
 * </pre>
 *
 * @see BloomFilter
 * @see <a href="https://webdocs.cs.ualberta.ca/~drafMDtail.pdf">Stable Bloom Filters</a>
 */
public final class StableBloomFilter extends NativeSketch implements MergeableSketch<StableBloomFilter> {

    private final long maxBytes;
    private final long logSizeOfArray;
    private final double fpr;

    /**
     * Create a new StableBloomFilter.
     *
     * @param maxBytes maximum memory to use in bytes
     * @param logSizeOfArray log2 of the number of cells (e.g., 10 = 1024 cells)
     *                       This controls the granularity of the filter
     * @param fpr target false positive rate (0 &lt; fpr &lt; 1)
     * @throws IllegalArgumentException if parameters are invalid
     * @throws RuntimeException if native memory allocation fails
     */
    public StableBloomFilter(long maxBytes, long logSizeOfArray, double fpr) {
        if (maxBytes <= 0) {
            throw new IllegalArgumentException("maxBytes must be positive, got: " + maxBytes);
        }
        if (logSizeOfArray <= 0 || logSizeOfArray > 30) {
            throw new IllegalArgumentException(
                    "logSizeOfArray must be in range (0, 30], got: " + logSizeOfArray);
        }
        if (fpr <= 0 || fpr >= 1) {
            throw new IllegalArgumentException("fpr must be in range (0, 1), got: " + fpr);
        }

        this.maxBytes = maxBytes;
        this.logSizeOfArray = logSizeOfArray;
        this.fpr = fpr;
        this.nativePtr = SketchOxideNative.stablebloomfilter_new(maxBytes, logSizeOfArray, fpr);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate StableBloomFilter");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private StableBloomFilter(long nativePtr, long maxBytes, long logSizeOfArray, double fpr) {
        this.nativePtr = nativePtr;
        this.maxBytes = maxBytes;
        this.logSizeOfArray = logSizeOfArray;
        this.fpr = fpr;
    }

    /**
     * Insert an item into the filter.
     *
     * The item is added and some random counters are decremented (eviction).
     * This maintains the stable FPR property.
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
        SketchOxideNative.stablebloomfilter_insert(nativePtr, item);
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
     * Check if an item might be in the filter.
     *
     * Returns true if the item might be in the set. For recently inserted items,
     * this is reliable. For older items, they may have "aged out" and return false.
     *
     * @param item the item to check (as raw bytes)
     * @return true if the item might be in the set, false if not (or aged out)
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public boolean contains(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.stablebloomfilter_contains(nativePtr, item);
    }

    /**
     * Check if a string item might be in the filter.
     *
     * @param item the string to check
     * @return true if the item might be in the set, false if not (or aged out)
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
     * Merge another StableBloomFilter into this one.
     *
     * After merging, this filter contains items from both filters.
     * Both filters must have compatible parameters.
     *
     * @param other the filter to merge into this one
     * @throws IllegalStateException if either filter is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(StableBloomFilter other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.maxBytes != other.maxBytes ||
                this.logSizeOfArray != other.logSizeOfArray ||
                Math.abs(this.fpr - other.fpr) > 1e-9) {
            throw new IllegalArgumentException(
                    "Cannot merge StableBloomFilters with different parameters");
        }

        SketchOxideNative.stablebloomfilter_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Serialize the filter to binary format.
     *
     * <p><strong>Note:</strong> StableBloomFilter may not support serialization
     * in all implementations due to its dynamic nature.
     *
     * @return binary representation of the filter
     * @throws IllegalStateException if the filter has been closed
     * @throws UnsupportedOperationException if serialization is not supported
     */
    public byte[] serialize() {
        checkAlive();
        throw new UnsupportedOperationException(
                "StableBloomFilter serialization is not supported");
    }

    /**
     * Deserialize a StableBloomFilter from binary format.
     *
     * @param data the serialized data
     * @return a new StableBloomFilter instance
     * @throws UnsupportedOperationException always
     */
    public static StableBloomFilter deserialize(byte[] data) {
        throw new UnsupportedOperationException(
                "StableBloomFilter deserialization is not supported");
    }

    /**
     * Get the maximum memory setting.
     *
     * @return the maxBytes parameter used at creation
     */
    public long getMaxBytes() {
        return maxBytes;
    }

    /**
     * Get the log size of array setting.
     *
     * @return the logSizeOfArray parameter used at creation
     */
    public long getLogSizeOfArray() {
        return logSizeOfArray;
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
            SketchOxideNative.stablebloomfilter_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("StableBloomFilter(maxBytes=%d, logSize=%d, fpr=%.6f)",
                    maxBytes, logSizeOfArray, fpr);
        }
        return "StableBloomFilter(closed)";
    }
}
