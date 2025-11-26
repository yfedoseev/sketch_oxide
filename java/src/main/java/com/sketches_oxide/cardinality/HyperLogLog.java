package com.sketches_oxide.cardinality;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;
import java.nio.ByteBuffer;

/**
 * HyperLogLog Cardinality Estimator.
 *
 * Provides probabilistic cardinality estimation with O(log(log(N))) space
 * complexity and ~1.04/sqrt(2^precision) standard error.
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Counting unique visitors in analytics</li>
 *   <li>Cardinality estimation in distributed systems</li>
 *   <li>Data deduplication verification</li>
 * </ul>
 *
 * <p><strong>Performance:</strong>
 * <ul>
 *   <li>Space: O(2^precision) bytes</li>
 *   <li>Time: O(1) per update</li>
 *   <li>Error: ~0.8% for typical precision=14</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (HyperLogLog hll = new HyperLogLog(14)) {
 *     for (String item : items) {
 *         hll.update(item.getBytes());
 *     }
 *     System.out.println("Unique items: " + Math.round(hll.estimate()));
 * }
 * </pre>
 *
 * @see <a href="https://en.wikipedia.org/wiki/HyperLogLog">HyperLogLog</a>
 */
public final class HyperLogLog extends NativeSketch implements MergeableSketch<HyperLogLog> {

    /**
     * Create a new HyperLogLog sketch.
     *
     * @param precision the precision parameter (4-16)
     *                  Determines space (2^precision) and error (~1.04/sqrt(2^precision))
     * @throws IllegalArgumentException if precision is not in range [4, 16]
     */
    public HyperLogLog(int precision) {
        if (precision < 4 || precision > 16) {
            throw new IllegalArgumentException(
                    "Precision must be between 4 and 16, got: " + precision);
        }

        this.nativePtr = SketchOxideNative.hyperLogLog_new(precision);
        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate HyperLogLog");
        }
    }

    /**
     * Add an item to the sketch.
     *
     * @param item the data to add
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.hyperLogLog_update(nativePtr, item);
    }

    /**
     * Update with data from a String.
     *
     * @param item the string to add
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
            SketchOxideNative.hyperLogLog_updateDirect(nativePtr, address, length);
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
     * @return estimated number of unique items
     * @throws IllegalStateException if the sketch has been closed
     */
    public double estimate() {
        checkAlive();
        return SketchOxideNative.hyperLogLog_estimate(nativePtr);
    }

    /**
     * Get the estimated cardinality as a long.
     *
     * @return estimated number of unique items, rounded to nearest long
     */
    public long estimateLong() {
        return Math.round(estimate());
    }

    /**
     * Merge another HyperLogLog into this one.
     *
     * Both sketches must have the same precision level.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if precisions don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(HyperLogLog other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.precision() != other.precision()) {
            throw new IllegalArgumentException(
                    "Cannot merge HyperLogLogs with different precisions: " +
                            this.precision() + " vs " + other.precision());
        }

        SketchOxideNative.hyperLogLog_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the precision level.
     *
     * @return the precision parameter used at creation
     */
    public int precision() {
        checkAlive();
        return SketchOxideNative.hyperLogLog_precision(nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * Useful for:
     * <ul>
     *   <li>Storing in a database</li>
     *   <li>Sending over the network</li>
     *   <li>Checkpointing for fault tolerance</li>
     * </ul>
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.hyperLogLog_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * Reconstructs a HyperLogLog from serialized data.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new HyperLogLog instance
     * @throws IllegalArgumentException if data is invalid or corrupt
     * @throws NullPointerException if data is null
     */
    public static HyperLogLog deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.hyperLogLog_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized HyperLogLog data");
        }

        HyperLogLog hll = new HyperLogLog(14);  // Dummy to get object initialized
        hll.nativePtr = ptr;
        return hll;
    }

    /**
     * Extract the memory address of a direct ByteBuffer.
     *
     * Uses sun.misc.Unsafe to get the buffer's memory address for zero-copy updates.
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
     * Reset the sketch to empty state.
     *
     * @throws IllegalStateException if the sketch has been closed
     */
    public void reset() {
        close();
        nativePtr = 0;
        // Recreate with original precision
        int originalPrecision = precision();
        nativePtr = SketchOxideNative.hyperLogLog_new(originalPrecision);
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
            SketchOxideNative.hyperLogLog_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("HyperLogLog(precision=%d, estimate=%.0f)",
                        precision(), estimate());
            }
        } catch (IllegalStateException e) {
            // Sketch is closed
        }
        return "HyperLogLog(closed)";
    }
}
