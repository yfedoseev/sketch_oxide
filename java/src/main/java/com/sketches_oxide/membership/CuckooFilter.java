package com.sketches_oxide.membership;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * CuckooFilter - Space-Efficient Probabilistic Membership with Deletion.
 *
 * A modern alternative to Bloom filters that stores fingerprints in a cuckoo hash table.
 * Supports deletion while using less space than counting Bloom filters, and provides
 * better lookup performance in many scenarios.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Stores fingerprints (partial hashes) in buckets</li>
 *   <li>Each item can reside in one of two candidate buckets</li>
 *   <li>Insert: place fingerprint in either bucket, or relocate existing items</li>
 *   <li>Delete: remove fingerprint from its bucket</li>
 *   <li>Query: check if fingerprint exists in either candidate bucket</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Network deduplication with removal support</li>
 *   <li>Database query optimization</li>
 *   <li>Distributed caching with eviction</li>
 *   <li>Real-time spam filtering</li>
 * </ul>
 *
 * <p><strong>Advantages over Bloom Filters:</strong>
 * <ul>
 *   <li>Supports deletion (standard Bloom does not)</li>
 *   <li>More space-efficient at low FPR (&lt; 3%)</li>
 *   <li>Faster lookups (better cache locality)</li>
 *   <li>Can report when filter is full</li>
 * </ul>
 *
 * <p><strong>Limitations:</strong>
 * <ul>
 *   <li>Has a maximum capacity (insertion can fail)</li>
 *   <li>Duplicate insertions can cause issues</li>
 *   <li>FPR depends on fingerprint size</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (CuckooFilter cf = new CuckooFilter(100000)) {
 *     // Insert items
 *     cf.insert("user@example.com".getBytes());
 *
 *     // Check membership
 *     if (cf.contains("user@example.com".getBytes())) {
 *         System.out.println("Email might be registered");
 *     }
 *
 *     // Remove item (e.g., user unsubscribed)
 *     cf.remove("user@example.com".getBytes());
 * }
 * </pre>
 *
 * @see BloomFilter
 * @see CountingBloomFilter
 * @see <a href="https://www.cs.cmu.edu/~dga/papers/cuckoo-conext2014.pdf">Cuckoo Filter Paper</a>
 */
public final class CuckooFilter extends NativeSketch implements MergeableSketch<CuckooFilter> {

    private final long capacity;

    /**
     * Create a new CuckooFilter.
     *
     * @param capacity maximum number of items the filter can hold
     *                 The actual capacity may be slightly higher due to bucket sizing
     * @throws IllegalArgumentException if capacity &lt;= 0
     * @throws RuntimeException if native memory allocation fails
     */
    public CuckooFilter(long capacity) {
        if (capacity <= 0) {
            throw new IllegalArgumentException("capacity must be positive, got: " + capacity);
        }

        this.capacity = capacity;
        this.nativePtr = SketchOxideNative.cuckoofilter_new(capacity);

        if (this.nativePtr == 0) {
            throw new RuntimeException("Failed to allocate CuckooFilter");
        }
    }

    /**
     * Private constructor for deserialization.
     */
    private CuckooFilter(long nativePtr, long capacity) {
        this.nativePtr = nativePtr;
        this.capacity = capacity;
    }

    /**
     * Insert an item into the filter.
     *
     * Computes a fingerprint and places it in one of two candidate buckets.
     * If both buckets are full, existing fingerprints may be relocated
     * (cuckoo hashing).
     *
     * <p><strong>Note:</strong> Insertion can fail if the filter is too full.
     * The filter operates best below 95% capacity.
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
        SketchOxideNative.cuckoofilter_insert(nativePtr, item);
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
     * Removes the fingerprint from its bucket. Unlike CountingBloomFilter,
     * this is a true removal - the item will definitely not be found
     * after removal (unless it was inserted multiple times).
     *
     * <p><strong>Warning:</strong> Removing items that were never inserted
     * can cause false negatives. Only remove items you are certain were inserted.
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
        SketchOxideNative.cuckoofilter_remove(nativePtr, item);
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
     * Returns true if a matching fingerprint is found in either candidate bucket.
     * Returns false if no matching fingerprint exists (definitely not in set).
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
        return SketchOxideNative.cuckoofilter_contains(nativePtr, item);
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
     * Merge another CuckooFilter into this one.
     *
     * <p><strong>Note:</strong> CuckooFilter merge is not typically supported
     * due to the nature of cuckoo hashing. This operation may fail or have
     * undefined behavior.
     *
     * @param other the filter to merge into this one
     * @throws UnsupportedOperationException always, as merge is not supported
     */
    @Override
    public void merge(CuckooFilter other) {
        throw new UnsupportedOperationException(
                "CuckooFilter does not support merge operations");
    }

    /**
     * Serialize the filter to binary format.
     *
     * @return binary representation of the filter
     * @throws IllegalStateException if the filter has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.cuckoofilter_serialize(nativePtr);
    }

    /**
     * Deserialize a CuckooFilter from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new CuckooFilter instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static CuckooFilter deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.cuckoofilter_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized CuckooFilter data");
        }

        return new CuckooFilter(ptr, 100);
    }

    /**
     * Get the capacity of the filter.
     *
     * @return the maximum number of items this filter can hold
     */
    public long getCapacity() {
        return capacity;
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
            SketchOxideNative.cuckoofilter_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        if (nativePtr != 0) {
            return String.format("CuckooFilter(capacity=%d)", capacity);
        }
        return "CuckooFilter(closed)";
    }
}
