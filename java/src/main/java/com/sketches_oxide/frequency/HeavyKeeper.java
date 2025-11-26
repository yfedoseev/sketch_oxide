package com.sketches_oxide.frequency;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;

/**
 * HeavyKeeper: High-Precision Top-K Heavy Hitter Detection
 *
 * <p>HeavyKeeper is a probabilistic data structure for identifying heavy hitters (top-k items)
 * in data streams with high precision. It uses an innovative exponential decay strategy that
 * actively removes small flows while protecting large flows.
 *
 * <p><strong>Use Cases:</strong>
 * <ul>
 *   <li>Top-K most frequent items in streaming data</li>
 *   <li>Network traffic analysis (elephant flow detection)</li>
 *   <li>Real-time analytics dashboards</li>
 *   <li>DDoS detection and mitigation</li>
 * </ul>
 *
 * <p><strong>Performance:</strong>
 * <ul>
 *   <li>Update: O(d) where d is depth (typically 4-6)</li>
 *   <li>Query: O(d)</li>
 *   <li>Space: O(d × w × 32 bits + k × 96 bits)</li>
 *   <li>High accuracy for heavy hitters with low memory</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (HeavyKeeper hk = new HeavyKeeper(100, 0.001, 0.01)) {
 *     // Track top-100 items
 *     for (int i = 0; i &lt; 10000; i++) {
 *         hk.update(("item_" + (i % 500)).getBytes());
 *     }
 *
 *     // Get top heavy hitters
 *     List&lt;TopKEntry&gt; topK = hk.topK();
 *     for (int i = 0; i &lt; Math.min(10, topK.size()); i++) {
 *         TopKEntry entry = topK.get(i);
 *         System.out.println("Hash: " + entry.getItemHash() + ", Count: " + entry.getCount());
 *     }
 *
 *     // Query specific item
 *     int count = hk.estimate("item_42".getBytes());
 *     System.out.println("Estimated count: " + count);
 * }
 * </pre>
 *
 * @see <a href="https://www.usenix.org/conference/atc18/presentation/gong">HeavyKeeper Paper (USENIX ATC 2018)</a>
 */
public final class HeavyKeeper extends NativeSketch {

    /**
     * Creates a new HeavyKeeper sketch.
     *
     * @param k Number of top items to track (must be &gt; 0)
     * @param epsilon Error bound in (0, 1). Smaller = higher accuracy but more space.
     * @param delta Failure probability in (0, 1). Smaller = higher confidence but more space.
     * @throws IllegalArgumentException if parameters are invalid
     * @throws OutOfMemoryError if native allocation fails
     */
    public HeavyKeeper(int k, double epsilon, double delta) {
        if (k <= 0) {
            throw new IllegalArgumentException("k must be > 0, got: " + k);
        }
        if (epsilon <= 0.0 || epsilon >= 1.0) {
            throw new IllegalArgumentException("epsilon must be in (0, 1), got: " + epsilon);
        }
        if (delta <= 0.0 || delta >= 1.0) {
            throw new IllegalArgumentException("delta must be in (0, 1), got: " + delta);
        }

        this.nativePtr = SketchOxideNative.heavyKeeper_new(k, epsilon, delta);
        if (this.nativePtr == 0) {
            throw new OutOfMemoryError("Failed to allocate HeavyKeeper");
        }
    }

    /**
     * Updates the sketch with an item.
     *
     * @param item The item to add (as byte array)
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.heavyKeeper_update(nativePtr, item);
    }

    /**
     * Updates the sketch with a String item.
     *
     * @param item The string item to add
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
     * Estimates the count of a specific item.
     *
     * <p>Returns the estimated frequency of the item. May overestimate due to hash collisions
     * but provides good accuracy for heavy hitters.
     *
     * @param item The item to query (as byte array)
     * @return The estimated count
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public int estimate(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return SketchOxideNative.heavyKeeper_estimate(nativePtr, item);
    }

    /**
     * Estimates the count of a specific String item.
     *
     * @param item The string item to query
     * @return The estimated count
     * @throws IllegalStateException if the sketch has been closed
     * @throws NullPointerException if item is null
     */
    public int estimate(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        return estimate(item.getBytes());
    }

    /**
     * Returns the top-k heavy hitters.
     *
     * <p>Returns a list of entries sorted by count in descending order. Each entry contains
     * the item hash and its estimated count.
     *
     * @return List of top-k entries, sorted by count descending
     * @throws IllegalStateException if the sketch has been closed
     */
    public List<TopKEntry> topK() {
        checkAlive();
        byte[] data = SketchOxideNative.heavyKeeper_topK(nativePtr);

        if (data == null || data.length == 0) {
            return new ArrayList<>();
        }

        // Deserialize: [count: 4 bytes][item_hash: 8 bytes] repeating
        List<TopKEntry> result = new ArrayList<>();
        ByteBuffer buffer = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);

        while (buffer.remaining() >= 12) {
            int count = buffer.getInt();
            long itemHash = buffer.getLong();
            result.add(new TopKEntry(itemHash, count));
        }

        return result;
    }

    /**
     * Applies exponential decay to all counters.
     *
     * <p>Divides all counts by the decay factor (default 1.08), which represents approximately
     * 8% decay. This ages old items and makes room for new heavy hitters.
     *
     * @throws IllegalStateException if the sketch has been closed
     */
    public void decay() {
        checkAlive();
        SketchOxideNative.heavyKeeper_decay(nativePtr);
    }

    /**
     * Merges another HeavyKeeper into this one.
     *
     * <p>Both sketches must have the same parameters (k, epsilon, delta).
     *
     * @param other The sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if parameters don't match
     * @throws NullPointerException if other is null
     */
    public void merge(HeavyKeeper other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        boolean success = SketchOxideNative.heavyKeeper_merge(this.nativePtr, other.nativePtr);
        if (!success) {
            throw new IllegalArgumentException(
                "Cannot merge HeavyKeepers with different parameters");
        }
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
            SketchOxideNative.heavyKeeper_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("HeavyKeeper(ptr=%d)", nativePtr);
            }
        } catch (IllegalStateException e) {
            // Sketch is closed
        }
        return "HeavyKeeper(closed)";
    }

    /**
     * Entry in the top-k list.
     *
     * <p>Contains the item hash and its estimated count.
     */
    public static final class TopKEntry {
        private final long itemHash;
        private final int count;

        TopKEntry(long itemHash, int count) {
            this.itemHash = itemHash;
            this.count = count;
        }

        /**
         * Gets the item hash.
         *
         * @return The 64-bit hash of the item
         */
        public long getItemHash() {
            return itemHash;
        }

        /**
         * Gets the estimated count.
         *
         * @return The estimated frequency
         */
        public int getCount() {
            return count;
        }

        @Override
        public String toString() {
            return String.format("TopKEntry(hash=%d, count=%d)", itemHash, count);
        }
    }
}
