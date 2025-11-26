package com.sketches.oxide;

/**
 * VacuumFilter: Best-in-class dynamic membership filter (VLDB 2020)
 *
 * <p>Vacuum filters combine the space efficiency of static filters with the flexibility
 * of dynamic operations (insertions AND deletions). The key innovation is a semi-sorted
 * bucket layout with adaptive fingerprint sizing, achieving superior memory efficiency
 * compared to Cuckoo, Bloom, and Quotient filters.</p>
 *
 * <h2>Key Advantages</h2>
 * <ul>
 *   <li><b>Space efficiency</b>: 12-14 bits/item at 1% FPR (best among dynamic filters)</li>
 *   <li><b>Cache-optimized</b>: Semi-sorting improves query performance</li>
 *   <li><b>Predictable performance</b>: No cuckoo evictions, just linear probing</li>
 *   <li><b>True deletions</b>: Unlike Bloom variants, actual removal without false negatives</li>
 *   <li><b>Configurable</b>: Tune fingerprint bits vs FPR tradeoff</li>
 * </ul>
 *
 * <h2>Performance Characteristics</h2>
 * <ul>
 *   <li>Insert: O(1) amortized (with rehashing)</li>
 *   <li>Query: O(1) expected</li>
 *   <li>Delete: O(1) expected</li>
 *   <li>Space: O(n) where n is capacity</li>
 * </ul>
 *
 * <h2>Example Usage</h2>
 * <pre>
 * // Create filter for 1000 items with 1% FPR
 * try (VacuumFilter filter = new VacuumFilter(1000, 0.01)) {
 *     // Insert items
 *     filter.insert("key1".getBytes());
 *     filter.insert("key2".getBytes());
 *
 *     // Query membership (no false negatives)
 *     assert filter.contains("key1".getBytes());
 *     assert filter.contains("key2".getBytes());
 *     assert !filter.contains("key3".getBytes());
 *
 *     // Delete items
 *     filter.delete("key1".getBytes());
 *     assert !filter.contains("key1".getBytes());
 *
 *     // Check statistics
 *     System.out.println("Load factor: " + filter.loadFactor());
 *     System.out.println("Memory: " + filter.memoryUsage() + " bytes");
 * }
 * </pre>
 *
 * <p><b>Thread Safety:</b> This class is NOT thread-safe. External synchronization
 * is required for concurrent access.</p>
 *
 * @see <a href="https://dl.acm.org/doi/10.14778/3389133.3389135">
 *      Vacuum Filters: More Space-Efficient and Faster Replacement for Bloom and Cuckoo Filters (VLDB 2020)</a>
 */
public class VacuumFilter implements AutoCloseable {

    static {
        System.loadLibrary("sketch_oxide_jni");
    }

    private long nativeHandle;
    private boolean closed = false;

    /**
     * Creates a new Vacuum Filter.
     *
     * @param capacity Expected number of elements
     * @param fpr Target false positive rate (e.g., 0.01 for 1%)
     * @throws IllegalArgumentException if capacity is 0 or fpr is not in range (0.0, 1.0)
     */
    public VacuumFilter(long capacity, double fpr) {
        if (capacity <= 0) {
            throw new IllegalArgumentException("Capacity must be > 0");
        }
        if (fpr <= 0.0 || fpr >= 1.0) {
            throw new IllegalArgumentException("FPR must be in range (0.0, 1.0)");
        }

        this.nativeHandle = nativeNew(capacity, fpr);
        if (this.nativeHandle == 0) {
            throw new RuntimeException("Failed to create VacuumFilter");
        }
    }

    /**
     * Inserts an element into the filter.
     *
     * @param key The key to insert
     * @throws IllegalStateException if the filter is closed
     * @throws NullPointerException if key is null
     */
    public void insert(byte[] key) {
        checkNotClosed();
        if (key == null) {
            throw new NullPointerException("Key cannot be null");
        }
        nativeInsert(nativeHandle, key);
    }

    /**
     * Checks if an element might be in the filter.
     *
     * @param key The key to check
     * @return true if the element might be present (with FPR probability of false positive),
     *         false if definitely not present (no false negatives)
     * @throws IllegalStateException if the filter is closed
     * @throws NullPointerException if key is null
     */
    public boolean contains(byte[] key) {
        checkNotClosed();
        if (key == null) {
            throw new NullPointerException("Key cannot be null");
        }
        return nativeContains(nativeHandle, key);
    }

    /**
     * Deletes an element from the filter.
     *
     * @param key The key to delete
     * @return true if the element was found and removed, false otherwise
     * @throws IllegalStateException if the filter is closed
     * @throws NullPointerException if key is null
     */
    public boolean delete(byte[] key) {
        checkNotClosed();
        if (key == null) {
            throw new NullPointerException("Key cannot be null");
        }
        return nativeDelete(nativeHandle, key);
    }

    /**
     * Returns the current load factor (0.0 to 1.0).
     *
     * @return Load factor as a fraction of capacity
     * @throws IllegalStateException if the filter is closed
     */
    public double loadFactor() {
        checkNotClosed();
        return nativeLoadFactor(nativeHandle);
    }

    /**
     * Returns the total capacity (maximum items before rehashing).
     *
     * @return Capacity in number of items
     * @throws IllegalStateException if the filter is closed
     */
    public long capacity() {
        checkNotClosed();
        return nativeCapacity(nativeHandle);
    }

    /**
     * Returns the number of items currently stored.
     *
     * @return Number of items
     * @throws IllegalStateException if the filter is closed
     */
    public long size() {
        checkNotClosed();
        return nativeLen(nativeHandle);
    }

    /**
     * Returns true if the filter is empty.
     *
     * @return true if empty, false otherwise
     * @throws IllegalStateException if the filter is closed
     */
    public boolean isEmpty() {
        checkNotClosed();
        return nativeIsEmpty(nativeHandle);
    }

    /**
     * Returns the memory usage in bytes.
     *
     * @return Memory usage in bytes
     * @throws IllegalStateException if the filter is closed
     */
    public long memoryUsage() {
        checkNotClosed();
        return nativeMemoryUsage(nativeHandle);
    }

    /**
     * Clears all items from the filter.
     *
     * @throws IllegalStateException if the filter is closed
     */
    public void clear() {
        checkNotClosed();
        nativeClear(nativeHandle);
    }

    /**
     * Closes the filter and releases native resources.
     *
     * <p>After calling this method, any further operations will throw IllegalStateException.</p>
     */
    @Override
    public void close() {
        if (!closed && nativeHandle != 0) {
            nativeFree(nativeHandle);
            nativeHandle = 0;
            closed = true;
        }
    }

    private void checkNotClosed() {
        if (closed) {
            throw new IllegalStateException("VacuumFilter has been closed");
        }
    }

    @Override
    protected void finalize() throws Throwable {
        try {
            close();
        } finally {
            super.finalize();
        }
    }

    // Native methods
    private static native long nativeNew(long capacity, double fpr);
    private static native void nativeInsert(long handle, byte[] key);
    private static native boolean nativeContains(long handle, byte[] key);
    private static native boolean nativeDelete(long handle, byte[] key);
    private static native double nativeLoadFactor(long handle);
    private static native long nativeCapacity(long handle);
    private static native long nativeLen(long handle);
    private static native boolean nativeIsEmpty(long handle);
    private static native long nativeMemoryUsage(long handle);
    private static native void nativeClear(long handle);
    private static native void nativeFree(long handle);
}
