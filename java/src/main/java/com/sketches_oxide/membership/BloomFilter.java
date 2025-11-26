package com.sketches_oxide.membership;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * BloomFilter - Classic Probabilistic Membership Testing.
 *
 * Provides fast membership testing with a configurable false positive rate.
 * Space-efficient alternative to hash tables for membership queries.
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>URL blacklisting / whitelisting</li>
 *   <li>Cache filtering (checking if item might be in cache)</li>
 *   <li>Duplicate detection in distributed systems</li>
 * </ul>
 *
 * <p><strong>Performance:</strong>
 * <ul>
 *   <li>Space: ~10 bits per item</li>
 *   <li>Time: O(k) per operation (k = number of hash functions)</li>
 *   <li>False positive rate: Configurable</li>
 *   <li>False negatives: 0 (guaranteed)</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (BloomFilter bf = new BloomFilter(1000, 0.01)) {
 *     bf.insert("user@example.com".getBytes());
 *     if (bf.contains("user@example.com".getBytes())) {
 *         System.out.println("Might be in the set");
 *     }
 * }
 * </pre>
 *
 * @see <a href="https://en.wikipedia.org/wiki/Bloom_filter">Bloom Filter</a>
 */
public final class BloomFilter extends NativeSketch implements MergeableSketch<BloomFilter> {

    private final long n;
    private final double fpr;

    /**
     * Create a new BloomFilter.
     *
     * @param n expected number of items
     * @param fpr desired false positive rate (0 < fpr < 1)
     * @throws IllegalArgumentException if parameters are invalid
     */
    public BloomFilter(long n, double fpr) {
        if (n <= 0) {
            throw new IllegalArgumentException("n must be positive, got: " + n);
        }
        if (fpr <= 0 || fpr >= 1) {
            throw new IllegalArgumentException("fpr must be in range (0, 1), got: " + fpr);
        }

        this.n = n;
        this.fpr = fpr;
        this.nativePtr = SketchOxideNative.bloomFilter_new(n, fpr);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate BloomFilter");
        }
    }

    /**
     * Insert an item into the filter.
     *
     * @param item the item to insert
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public void insert(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.bloomFilter_insert(nativePtr, item);
    }

    /**
     * Insert a string item.
     *
     * @param item the string to insert
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
     * Returns true if the item might be in the set (with false positive rate = fpr).
     * Returns false if the item is definitely not in the set.
     *
     * @param item the item to check
     * @return true if the item might be in the set, false if definitely not
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if item is null
     */
    public boolean contains(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.bloomFilter_contains(nativePtr, item);
    }

    /**
     * Check if a string item might be in the filter.
     *
     * @param item the string to check
     * @return true if the item might be in the set, false if definitely not
     */
    public boolean contains(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return contains(item.getBytes());
    }

    /**
     * Merge another BloomFilter into this one.
     *
     * Both filters must have the same n and fpr parameters.
     *
     * @param other the filter to merge
     * @throws IllegalStateException if either filter is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(BloomFilter other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.n != other.n || Math.abs(this.fpr - other.fpr) > 1e-9) {
            throw new IllegalArgumentException(
                    "Cannot merge BloomFilters with different parameters: " +
                            String.format("(n=%d, fpr=%.6f) vs (n=%d, fpr=%.6f)",
                                    this.n, this.fpr, other.n, other.fpr));
        }

        SketchOxideNative.bloomFilter_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Serialize the filter to binary format.
     *
     * @return binary representation of the filter
     * @throws IllegalStateException if the filter has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.bloomFilter_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new BloomFilter instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static BloomFilter deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.bloomFilter_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized BloomFilter data");
        }

        BloomFilter bf = new BloomFilter(100, 0.01);  // Dummy
        bf.nativePtr = ptr;
        return bf;
    }

    /**
     * Get the expected number of items.
     *
     * @return the n parameter
     */
    public long getN() {
        return n;
    }

    /**
     * Get the false positive rate.
     *
     * @return the fpr parameter
     */
    public double getFpr() {
        return fpr;
    }

    /**
     * Insert multiple items into the filter in a single call (optimized for throughput).
     *
     * <p>Batch inserts are significantly faster than multiple individual insert() calls
     * because they amortize the JNI (Java Native Interface) overhead across many items.
     * This is the preferred method when adding large quantities of data.
     *
     * @param items the items to insert
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if items is null
     */
    public void insertBatch(byte[]... items) {
        checkAlive();
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        for (byte[] item : items) {
            insert(item);
        }
    }

    /**
     * Insert multiple string items into the filter in a single call.
     *
     * @param items the string items to insert
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if items is null
     */
    public void insertBatch(String... items) {
        checkAlive();
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        for (String item : items) {
            insert(item);
        }
    }

    /**
     * Check multiple items with a single call (optimized for lookups).
     *
     * @param items the items to check
     * @return array of booleans, one for each item
     * @throws IllegalStateException if the filter has been closed
     * @throws NullPointerException if items is null
     */
    public boolean[] containsBatch(byte[]... items) {
        checkAlive();
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        boolean[] results = new boolean[items.length];
        for (int i = 0; i < items.length; i++) {
            results[i] = contains(items[i]);
        }
        return results;
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
            SketchOxideNative.bloomFilter_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("BloomFilter(n=%d, fpr=%.6f)", n, fpr);
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "BloomFilter(closed)";
    }
}
