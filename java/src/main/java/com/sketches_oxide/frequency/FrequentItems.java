package com.sketches_oxide.frequency;

import com.sketches_oxide.MergeableSketch;
import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * FrequentItems - Top-K Frequent Items Detection with Error Bounds.
 *
 * <p>FrequentItems maintains a summary of the most frequent items in a data stream
 * with configurable maximum size. It provides frequency estimates along with
 * guaranteed upper and lower bounds on the true frequency.
 *
 * <p><strong>Algorithm Overview:</strong>
 * <p>FrequentItems uses a modified Misra-Gries or Space-Saving algorithm variant
 * that maintains:
 * <ul>
 *   <li>A fixed-size map of item -> count pairs</li>
 *   <li>Error tracking for accurate bound computation</li>
 *   <li>Efficient mergeability for distributed scenarios</li>
 * </ul>
 *
 * <p><strong>Error Bounds:</strong>
 * <ul>
 *   <li><strong>Lower bound</strong>: Guaranteed minimum true frequency</li>
 *   <li><strong>Upper bound</strong>: Guaranteed maximum true frequency</li>
 *   <li><strong>Point estimate</strong>: Best guess (typically upper bound)</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Finding most popular products in e-commerce</li>
 *   <li>Identifying trending topics</li>
 *   <li>Top-K query result caching decisions</li>
 *   <li>Hot spot detection in distributed systems</li>
 * </ul>
 *
 * <p><strong>Space Complexity:</strong> O(maxSize)
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (FrequentItems fi = new FrequentItems(1000)) {
 *     for (String item : stream) {
 *         fi.update(item);
 *     }
 *
 *     // Get items with estimated frequency > 100
 *     byte[][] frequentItems = fi.getFrequentItems(100);
 *     for (byte[] item : frequentItems) {
 *         String key = new String(item);
 *         System.out.println(key + ": " + fi.lowerBound(key) +
 *                           " to " + fi.upperBound(key));
 *     }
 * }
 * </pre>
 *
 * @see SpaceSaving
 * @see CountMinSketch
 */
public final class FrequentItems extends NativeSketch implements MergeableSketch<FrequentItems> {

    /** Minimum allowed maxSize. */
    public static final long MIN_MAX_SIZE = 1;

    /** Maximum allowed maxSize. */
    public static final long MAX_MAX_SIZE = 1_000_000_000L;

    /** Default maxSize providing good balance. */
    public static final long DEFAULT_MAX_SIZE = 1000;

    private final long maxSize;

    /**
     * Create a new FrequentItems sketch with default maxSize (1000).
     *
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public FrequentItems() {
        this(DEFAULT_MAX_SIZE);
    }

    /**
     * Create a new FrequentItems sketch.
     *
     * @param maxSize maximum number of items to track (1 to 1,000,000,000)
     *                Higher values track more items but use more memory.
     * @throws IllegalArgumentException if maxSize is invalid
     * @throws OutOfMemoryException if native memory allocation fails
     */
    public FrequentItems(long maxSize) {
        if (maxSize < MIN_MAX_SIZE || maxSize > MAX_MAX_SIZE) {
            throw new IllegalArgumentException(
                    "maxSize must be between " + MIN_MAX_SIZE + " and " + MAX_MAX_SIZE +
                            ", got: " + maxSize);
        }

        this.maxSize = maxSize;
        this.nativePtr = SketchOxideNative.frequentitems_new(maxSize);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate FrequentItems");
        }
    }

    /**
     * Update the frequency of an item (increment by 1).
     *
     * @param item the item to count
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.frequentitems_update(nativePtr, item);
    }

    /**
     * Update with a string item.
     *
     * @param item the string to count
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
     * Estimate the frequency of an item.
     *
     * <p>Returns the point estimate, which is typically the upper bound.
     *
     * @param item the item to query
     * @return estimated frequency
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long estimate(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.frequentitems_estimate(nativePtr, item);
    }

    /**
     * Estimate the frequency of a string item.
     *
     * @param item the string to query
     * @return estimated frequency
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long estimate(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return estimate(item.getBytes());
    }

    /**
     * Get the guaranteed lower bound on an item's true frequency.
     *
     * <p>The true frequency is guaranteed to be >= this value.
     *
     * @param item the item to query
     * @return lower bound on true frequency
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long lowerBound(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.frequentitems_lowerBound(nativePtr, item);
    }

    /**
     * Get the guaranteed lower bound on a string item's frequency.
     *
     * @param item the string to query
     * @return lower bound on true frequency
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long lowerBound(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return lowerBound(item.getBytes());
    }

    /**
     * Get the guaranteed upper bound on an item's true frequency.
     *
     * <p>The true frequency is guaranteed to be <= this value.
     *
     * @param item the item to query
     * @return upper bound on true frequency
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long upperBound(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.frequentitems_upperBound(nativePtr, item);
    }

    /**
     * Get the guaranteed upper bound on a string item's frequency.
     *
     * @param item the string to query
     * @return upper bound on true frequency
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public long upperBound(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return upperBound(item.getBytes());
    }

    /**
     * Get items with estimated frequency above a threshold.
     *
     * <p>Returns all monitored items whose estimated frequency exceeds
     * the given threshold.
     *
     * @param threshold minimum frequency threshold
     * @return array of items with frequency > threshold
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if threshold is negative
     */
    public byte[][] getFrequentItems(long threshold) {
        checkAlive();
        if (threshold < 0) {
            throw new IllegalArgumentException("threshold must be non-negative, got: " + threshold);
        }
        return SketchOxideNative.frequentitems_getFrequentItems(nativePtr, threshold);
    }

    /**
     * Get items with estimated frequency above a threshold as strings.
     *
     * @param threshold minimum frequency threshold
     * @return array of items with frequency > threshold as strings
     * @throws IllegalStateException if the sketch has been closed
     * @throws IllegalArgumentException if threshold is negative
     */
    public String[] getFrequentItemsAsStrings(long threshold) {
        byte[][] items = getFrequentItems(threshold);
        String[] result = new String[items.length];
        for (int i = 0; i < items.length; i++) {
            result[i] = new String(items[i]);
        }
        return result;
    }

    /**
     * Merge another FrequentItems into this one.
     *
     * <p>Both sketches must have the same maxSize parameter.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if maxSize values don't match
     * @throws NullPointerException if other is null
     */
    @Override
    public void merge(FrequentItems other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (this.maxSize != other.maxSize) {
            throw new IllegalArgumentException(
                    "Cannot merge FrequentItems with different maxSize: " +
                            this.maxSize + " vs " + other.maxSize);
        }

        SketchOxideNative.frequentitems_merge(this.nativePtr, other.nativePtr);
    }

    /**
     * Get the maximum number of items this sketch can track.
     *
     * @return the maxSize parameter
     */
    public long maxSize() {
        return maxSize;
    }

    /**
     * Get the current number of tracked items.
     *
     * @return the number of items currently being tracked
     * @throws IllegalStateException if the sketch has been closed
     */
    public long currentSize() {
        checkAlive();
        return SketchOxideNative.frequentitems_currentSize(nativePtr);
    }

    /**
     * Serialize the sketch to binary format.
     *
     * @return binary representation of the sketch
     * @throws IllegalStateException if the sketch has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.frequentitems_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new FrequentItems instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static FrequentItems deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.frequentitems_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized FrequentItems data");
        }

        FrequentItems fi = new FrequentItems(DEFAULT_MAX_SIZE);
        SketchOxideNative.frequentitems_free(fi.nativePtr);
        fi.nativePtr = ptr;
        return fi;
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
            SketchOxideNative.frequentitems_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("FrequentItems(maxSize=%d, currentSize=%d)",
                        maxSize, currentSize());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "FrequentItems(closed)";
    }
}
