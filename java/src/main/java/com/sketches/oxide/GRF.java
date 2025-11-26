package com.sketches.oxide;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;

/**
 * GRF (Gorilla Range Filter): Shape-Based Range Filter for LSM-Trees
 *
 * <p>GRF is an advanced range filter optimized for LSM-tree workloads. Unlike traditional
 * range filters, GRF uses shape encoding to capture the distribution of keys, enabling
 * more efficient range queries for skewed data.</p>
 *
 * <h2>Algorithm Overview</h2>
 * <ol>
 *   <li><b>Key Sorting</b>: Input keys are sorted and deduplicated</li>
 *   <li><b>Shape Encoding</b>: Keys are segmented based on distribution patterns</li>
 *   <li><b>Fingerprinting</b>: Each segment gets a compact fingerprint</li>
 *   <li><b>Range Queries</b>: Efficiently check which segments overlap query range</li>
 * </ol>
 *
 * <h2>Key Innovation</h2>
 * <p>Traditional range filters treat all ranges equally. GRF's shape encoding
 * adapts to key distribution, providing:</p>
 * <ul>
 *   <li><b>Better FPR</b> for skewed distributions (Zipf, power-law)</li>
 *   <li><b>Adaptive segments</b> that match data patterns</li>
 *   <li><b>LSM-tree optimization</b> for compaction and merge operations</li>
 * </ul>
 *
 * <h2>Performance Characteristics</h2>
 * <ul>
 *   <li>Build: O(n log n) for sorting + O(n) for segmentation</li>
 *   <li>Query: O(log n) binary search + O(k) segment checks</li>
 *   <li>Space: B bits per key (comparable to Grafite)</li>
 *   <li>FPR: Better than Grafite for skewed distributions</li>
 * </ul>
 *
 * <h2>Production Use Cases</h2>
 * <ul>
 *   <li>RocksDB/LevelDB SSTable filters</li>
 *   <li>Time-series databases (InfluxDB, TimescaleDB)</li>
 *   <li>Log aggregation systems (Elasticsearch, Loki)</li>
 *   <li>Columnar databases (Parquet, ORC)</li>
 *   <li>Financial time-series data</li>
 * </ul>
 *
 * <h2>Example Usage</h2>
 * <pre>
 * // Build GRF from keys with skewed distribution
 * long[] keys = {1, 2, 3, 5, 8, 13, 21, 34, 55, 89}; // Fibonacci
 * try (GRF grf = GRF.build(keys, 6)) {
 *     // Query ranges
 *     assert grf.mayContainRange(10, 25); // Contains 13, 21
 *     assert grf.mayContain(13); // Point query
 *
 *     // Get statistics
 *     System.out.println("Segments: " + grf.segmentCount());
 *     System.out.println("Keys: " + grf.keyCount());
 *
 *     // Expected FPR for range width
 *     double fpr = grf.expectedFpr(10);
 *     System.out.println("Expected FPR: " + fpr);
 * }
 * </pre>
 *
 * <p><b>Thread Safety:</b> This class is thread-safe for read operations after construction.
 * The filter is immutable once built.</p>
 *
 * @see <a href="https://dl.acm.org/doi/10.1145/3588908">
 *      Gorilla Range Filter: Shape-Based Range Filtering for LSM-Trees (SIGMOD 2024)</a>
 */
public class GRF implements AutoCloseable {

    static {
        System.loadLibrary("sketch_oxide_jni");
    }

    private long nativeHandle;
    private boolean closed = false;

    private GRF(long nativeHandle) {
        this.nativeHandle = nativeHandle;
    }

    /**
     * Builds a GRF filter from a set of keys.
     *
     * @param keys Array of keys to build the filter from
     * @param bitsPerKey Number of bits per key (typically 4-8)
     * @return A new GRF instance
     * @throws IllegalArgumentException if keys is empty or bitsPerKey is invalid
     * @throws RuntimeException if building the filter fails
     */
    public static GRF build(long[] keys, int bitsPerKey) {
        if (keys == null || keys.length == 0) {
            throw new IllegalArgumentException("Keys cannot be null or empty");
        }
        if (bitsPerKey < 2 || bitsPerKey > 16) {
            throw new IllegalArgumentException("bitsPerKey must be between 2 and 16");
        }

        // Convert long[] to byte array (8 bytes per long)
        ByteBuffer buffer = ByteBuffer.allocate(keys.length * 8);
        buffer.order(ByteOrder.LITTLE_ENDIAN);
        for (long key : keys) {
            buffer.putLong(key);
        }

        long handle = nativeBuild(buffer.array(), bitsPerKey);
        if (handle == 0) {
            throw new RuntimeException("Failed to build GRF");
        }

        return new GRF(handle);
    }

    /**
     * Checks if a range of values might be in the filter.
     *
     * @param low Lower bound of range (inclusive)
     * @param high Upper bound of range (inclusive)
     * @return true if range might contain keys, false if definitely does not
     * @throws IllegalStateException if the filter is closed
     */
    public boolean mayContainRange(long low, long high) {
        checkNotClosed();
        return nativeMayContainRange(nativeHandle, low, high);
    }

    /**
     * Checks if a single key may be present (equivalent to mayContainRange(key, key)).
     *
     * @param key The key to check
     * @return true if the key might be present, false if definitely not present
     * @throws IllegalStateException if the filter is closed
     */
    public boolean mayContain(long key) {
        checkNotClosed();
        return nativeMayContain(nativeHandle, key);
    }

    /**
     * Returns the number of keys in the filter.
     *
     * @return Number of unique keys
     * @throws IllegalStateException if the filter is closed
     */
    public long keyCount() {
        checkNotClosed();
        return nativeKeyCount(nativeHandle);
    }

    /**
     * Returns the number of segments.
     *
     * @return Number of segments created
     * @throws IllegalStateException if the filter is closed
     */
    public long segmentCount() {
        checkNotClosed();
        return nativeSegmentCount(nativeHandle);
    }

    /**
     * Returns the bits per key configuration.
     *
     * @return Bits per key
     * @throws IllegalStateException if the filter is closed
     */
    public int bitsPerKey() {
        checkNotClosed();
        return nativeBitsPerKey(nativeHandle);
    }

    /**
     * Calculates expected FPR for a given range width.
     *
     * <p>GRF's FPR adapts to the distribution. For skewed data, it's typically
     * better than the theoretical Grafite bound of L / 2^(B-2).</p>
     *
     * @param rangeWidth Width of the query range
     * @return Expected false positive rate (0.0 to 1.0)
     * @throws IllegalStateException if the filter is closed
     */
    public double expectedFpr(long rangeWidth) {
        checkNotClosed();
        return nativeExpectedFpr(nativeHandle, rangeWidth);
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
            throw new IllegalStateException("GRF has been closed");
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
    private static native long nativeBuild(byte[] keys, int bitsPerKey);
    private static native boolean nativeMayContainRange(long handle, long low, long high);
    private static native boolean nativeMayContain(long handle, long key);
    private static native long nativeKeyCount(long handle);
    private static native long nativeSegmentCount(long handle);
    private static native int nativeBitsPerKey(long handle);
    private static native double nativeExpectedFpr(long handle, long rangeWidth);
    private static native void nativeFree(long handle);
}
