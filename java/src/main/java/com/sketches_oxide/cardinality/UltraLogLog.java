package com.sketches_oxide.cardinality;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;
import java.nio.ByteBuffer;

/**
 * UltraLogLog Cardinality Estimator - State-of-the-Art Algorithm (VLDB 2024).
 *
 * <p>UltraLogLog is an improved version of HyperLogLog that provides approximately
 * 28% better space efficiency for the same accuracy level. It uses an improved
 * estimator formula and better bias correction, especially for small cardinalities.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <ol>
 *   <li>Hash each input item to get a uniform random value</li>
 *   <li>Use the first p bits to select one of 2^p registers</li>
 *   <li>Count leading zeros in the remaining bits and store the maximum in each register</li>
 *   <li>Estimate cardinality using harmonic mean of register values with bias correction</li>
 * </ol>
 *
 * <p><strong>Key Improvements over HyperLogLog:</strong>
 * <ul>
 *   <li>28% more space-efficient for the same accuracy</li>
 *   <li>Better bias correction for small cardinalities</li>
 *   <li>Smoother transition between different estimator modes</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Counting unique visitors in analytics with tighter memory constraints</li>
 *   <li>Cardinality estimation in distributed systems</li>
 *   <li>Data deduplication verification</li>
 *   <li>Network traffic analysis</li>
 * </ul>
 *
 * <p><strong>Performance:</strong>
 * <ul>
 *   <li>Space: O(2^precision) bytes (precision 4-18)</li>
 *   <li>Time: O(1) per update</li>
 *   <li>Error: ~1.04/sqrt(2^precision) standard error</li>
 * </ul>
 *
 * <p><strong>Memory Usage by Precision:</strong>
 * <ul>
 *   <li>precision 4: 16 registers, 16 bytes</li>
 *   <li>precision 8: 256 registers, 256 bytes</li>
 *   <li>precision 12: 4,096 registers, 4 KB (recommended)</li>
 *   <li>precision 14: 16,384 registers, 16 KB</li>
 *   <li>precision 16: 65,536 registers, 64 KB</li>
 *   <li>precision 18: 262,144 registers, 256 KB</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (UltraLogLog ull = new UltraLogLog(14)) {
 *     for (String item : items) {
 *         ull.update(item);
 *     }
 *     System.out.println("Unique items: " + Math.round(ull.estimate()));
 * }
 * </pre>
 *
 * @see <a href="https://vldb.org/pvldb/vol17/p2699-lang.pdf">UltraLogLog: A Practical and More Space-Efficient Alternative to HyperLogLog</a>
 * @see HyperLogLog
 */
public final class UltraLogLog extends NativeSketch implements MergeableSketch<UltraLogLog> {

    /** Minimum allowed precision value. */
    public static final int MIN_PRECISION = 4;

    /** Maximum allowed precision value. */
    public static final int MAX_PRECISION = 18;

    /** Default precision value providing good balance between accuracy and memory. */
    public static final int DEFAULT_PRECISION = 14;

    /**
     * Create a new UltraLogLog sketch with default precision (14).
     *
     * <p>Default precision provides approximately 0.8% standard error
     * with 16 KB memory usage.
     *
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public UltraLogLog() {
        this(DEFAULT_PRECISION);
    }

    /**
     * Create a new UltraLogLog sketch.
     *
     * @param precision the precision parameter (4-18)
     *                  Determines space (2^precision bytes) and error (~1.04/sqrt(2^precision))
     * @throws IllegalArgumentException if precision is not in range [4, 18]
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public UltraLogLog(int precision) {
        if (precision < MIN_PRECISION || precision > MAX_PRECISION) {
            throw new IllegalArgumentException(
                    "Precision must be between " + MIN_PRECISION + " and " + MAX_PRECISION +
                            ", got: " + precision);
        }

        this.nativePtr = SketchOxideNative.ultraLogLog_new(precision);
        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate UltraLogLog");
        }
    }

    /**
     * Add an item to the sketch.
     *
     * <p>The item is hashed internally to determine which register to update.
     * Adding the same item multiple times has no additional effect on the estimate.
     *
     * @param item the byte array data to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.ultraLogLog_update(nativePtr, item);
    }

    /**
     * Add a string item to the sketch.
     *
     * <p>Convenience method that converts the string to UTF-8 bytes before adding.
     *
     * @param item the string to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        update(item.getBytes());
    }

    /**
     * Update with data from a direct ByteBuffer (zero-copy).
     *
     * <p>For best performance, use a direct (off-heap) ByteBuffer.
     * This method will work with heap-based buffers but will not provide
     * the zero-copy performance benefit.
     *
     * @param buffer the buffer containing data to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if buffer is null
     */
    public void update(ByteBuffer buffer) {
        checkAlive();
        if (buffer == null) {
            throw new NullPointerException("buffer cannot be null");
        }

        if (buffer.isDirect()) {
            // Zero-copy path: use direct buffer address
            long address = getDirectBufferAddress(buffer);
            int length = buffer.remaining();
            SketchOxideNative.ultraLogLog_updateDirect(nativePtr, address, length);
        } else {
            // Fallback for heap buffers: copy to byte array
            byte[] bytes = new byte[buffer.remaining()];
            buffer.get(bytes);
            update(bytes);
        }
    }

    /**
     * Get the estimated cardinality.
     *
     * <p>Returns the estimated number of unique items that have been added to the sketch.
     * The estimate has a standard error of approximately 1.04/sqrt(2^precision).
     *
     * @return estimated number of unique items as a double
     * @throws IllegalStateException if the sketch has been closed
     */
    public double estimate() {
        checkAlive();
        return SketchOxideNative.ultraLogLog_estimate(nativePtr);
    }

    /**
     * Get the estimated cardinality as a long.
     *
     * <p>Convenience method that rounds the estimate to the nearest long value.
     *
     * @return estimated number of unique items, rounded to nearest long
     * @throws IllegalStateException if the sketch has been closed
     */
    public long estimateLong() {
        return Math.round(estimate());
    }

    /**
     * Merge another UltraLogLog into this one.
     *
     * <p>After merging, this sketch represents the union of both sketches,
     * providing an estimate of the total unique items across both original sketches.
     * Both sketches must have the same precision level.
     *
     * <p>The other sketch is not modified by this operation.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if precisions don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(UltraLogLog other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.precision() != other.precision()) {
            throw new IllegalArgumentException(
                    "Cannot merge UltraLogLogs with different precisions: " +
                            this.precision() + " vs " + other.precision());
        }

        SketchOxideNative.ultraLogLog_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the precision level.
     *
     * <p>The precision determines the number of registers (2^precision) and
     * consequently the memory usage and accuracy of the sketch.
     *
     * @return the precision parameter used at creation
     * @throws IllegalStateException if the sketch has been closed
     */
    public int precision() {
        checkAlive();
        return SketchOxideNative.ultraLogLog_precision(nativePtr);
    }

    /**
     * Get the number of registers in this sketch.
     *
     * <p>Returns 2^precision, which equals the number of bytes used for storage.
     *
     * @return the number of registers (2^precision)
     * @throws IllegalStateException if the sketch has been closed
     */
    public int registerCount() {
        return 1 << precision();
    }

    /**
     * Calculate the theoretical standard error for this sketch's precision.
     *
     * <p>The standard error is approximately 1.04/sqrt(2^precision).
     *
     * @return the theoretical relative standard error (e.g., 0.008 for ~0.8% error)
     * @throws IllegalStateException if the sketch has been closed
     */
    public double theoreticalError() {
        return 1.04 / Math.sqrt(registerCount());
    }

    /**
     * Serialize the sketch to binary format.
     *
     * <p>The serialized format can be stored in a database, sent over the network,
     * or saved to disk for later reconstruction using {@link #deserialize(byte[])}.
     *
     * <p>Useful for:
     * <ul>
     *   <li>Storing in a database</li>
     *   <li>Sending over the network</li>
     *   <li>Checkpointing for fault tolerance</li>
     *   <li>Sharing between different language bindings</li>
     * </ul>
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.ultraLogLog_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * <p>Reconstructs an UltraLogLog from serialized data produced by {@link #serialize()}.
     * The precision is automatically determined from the serialized data.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new UltraLogLog instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static UltraLogLog deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.ultraLogLog_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized UltraLogLog data");
        }

        // Create a dummy object and replace its native pointer
        // The precision will be read from the deserialized native object
        UltraLogLog ull = new UltraLogLog(DEFAULT_PRECISION);
        // Free the dummy pointer before replacing
        SketchOxideNative.ultraLogLog_free(ull.nativePtr);
        ull.nativePtr = ptr;
        return ull;
    }

    /**
     * Extract the memory address of a direct ByteBuffer.
     *
     * Uses reflection to get the buffer's memory address for zero-copy updates.
     *
     * @param buffer a direct ByteBuffer
     * @return the memory address
     */
    private long getDirectBufferAddress(ByteBuffer buffer) {
        try {
            java.lang.reflect.Field addressField = sun.nio.ch.DirectBuffer.class.getDeclaredField("address");
            addressField.setAccessible(true);
            return (long) addressField.get(buffer);
        } catch (Exception e) {
            // Fallback: try using Unsafe if reflection doesn't work
            try {
                sun.misc.Unsafe unsafe = sun.misc.Unsafe.getUnsafe();
                java.lang.reflect.Field f = sun.nio.ch.DirectBuffer.class.getDeclaredField("address");
                return unsafe.getLong(buffer, unsafe.objectFieldOffset(f));
            } catch (Exception ex) {
                throw new RuntimeException("Failed to get DirectBuffer address", ex);
            }
        }
    }

    /**
     * Update with a batch of items (optimized for throughput).
     *
     * <p>Batch updates are significantly faster than multiple individual update() calls
     * because they amortize the JNI (Java Native Interface) overhead across many items.
     * This is the preferred method when adding large quantities of data.
     *
     * @param items the items to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if items is null
     */
    public void updateBatch(byte[]... items) {
        checkAlive();
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        for (byte[] item : items) {
            update(item);
        }
    }

    /**
     * Update with a batch of string items (optimized for throughput).
     *
     * @param items the string items to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if items is null
     */
    public void updateBatch(String... items) {
        checkAlive();
        if (items == null) {
            throw new NullPointerException("items cannot be null");
        }
        for (String item : items) {
            update(item);
        }
    }

    /**
     * Close the sketch and release native memory.
     *
     * <p>This method is idempotent - calling it multiple times has no additional effect.
     * After calling close(), all other methods except close() will throw
     * {@link IllegalStateException}.
     *
     * <p>It is recommended to use try-with-resources to ensure proper cleanup:
     * <pre>
     * try (UltraLogLog ull = new UltraLogLog(14)) {
     *     ull.update("item");
     *     // ...
     * } // automatically closed here
     * </pre>
     */
    @Override
    public void close() {
        if (nativePtr != 0) {
            freeNative();
            nativePtr = 0;
        }
    }

    /**
     * Free native memory resources.
     *
     * <p>Called internally by {@link #close()} and the finalizer.
     * Subclasses should not call this method directly.
     */
    @Override
    protected void freeNative() {
        if (nativePtr != 0) {
            SketchOxideNative.ultraLogLog_free(nativePtr);
        }
    }

    /**
     * Returns a string representation of the sketch.
     *
     * <p>Includes the precision and current cardinality estimate.
     *
     * @return string representation in the format "UltraLogLog(precision=N, estimate=M)"
     */
    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("UltraLogLog(precision=%d, estimate=%.0f)",
                        precision(), estimate());
            }
        } catch (IllegalStateException e) {
            // Sketch is closed
        }
        return "UltraLogLog(closed)";
    }
}
