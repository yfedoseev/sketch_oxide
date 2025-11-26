package com.sketches_oxide.sampling;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * ReservoirSampling - Uniform Random Sampling from Streams.
 *
 * Maintains a uniform random sample of k items from a stream of unknown length.
 * Each item in the stream has an equal probability of being in the final sample.
 * Classic algorithm by Vitter (1985) for one-pass sampling.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>First k items go directly into the reservoir</li>
 *   <li>Item i (i &gt; k) replaces a random item with probability k/i</li>
 *   <li>Guarantees uniform sampling without knowing stream size</li>
 *   <li>Single pass, O(k) memory</li>
 * </ul>
 *
 * <p><strong>Properties:</strong>
 * <ul>
 *   <li>Space: O(k) items</li>
 *   <li>Time: O(1) per item</li>
 *   <li>Uniformity: Each item has exactly k/n probability of being sampled</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Sampling from large data streams</li>
 *   <li>Random subset selection for analysis</li>
 *   <li>A/B testing cohort selection</li>
 *   <li>Statistical sampling for surveys</li>
 *   <li>Downsampling for visualization</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * // Keep a random sample of 100 items
 * try (ReservoirSampling sampler = new ReservoirSampling(100)) {
 *     // Process a stream of items
 *     for (String item : largeStream) {
 *         sampler.update(item.getBytes());
 *     }
 *
 *     // Get the uniform random sample
 *     byte[][] sample = sampler.sample();
 *     System.out.printf("Sampled %d items from %d total%n",
 *         sample.length, sampler.count());
 *
 *     // Process sampled items
 *     for (byte[] item : sample) {
 *         String str = new String(item);
 *         System.out.println("Sampled: " + str);
 *     }
 * }
 * </pre>
 *
 * @see VarOptSampling
 * @see <a href="https://en.wikipedia.org/wiki/Reservoir_sampling">Reservoir Sampling</a>
 */
public final class ReservoirSampling extends NativeSketch {

    private final long k;

    /**
     * Create a new ReservoirSampling instance.
     *
     * @param k the reservoir size (maximum number of samples to keep)
     * @throws IllegalArgumentException if k &lt;= 0
     */
    public ReservoirSampling(long k) {
        if (k <= 0) {
            throw new IllegalArgumentException("k must be positive, got: " + k);
        }

        this.k = k;
        this.nativePtr = SketchOxideNative.reservoirsampling_new(k);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate ReservoirSampling");
        }
    }

    /**
     * Add an item to the stream.
     *
     * The item may or may not be kept in the reservoir based on
     * the random selection algorithm.
     *
     * @param item the item to potentially sample
     * @throws IllegalStateException if the sampler has been closed
     * @throws NullPointerException if item is null
     */
    public void update(byte[] item) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        SketchOxideNative.reservoirsampling_update(nativePtr, item);
    }

    /**
     * Add a string item to the stream.
     *
     * @param item the string item to potentially sample
     */
    public void update(String item) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        update(item.getBytes());
    }

    /**
     * Get the current sample.
     *
     * Returns up to k items uniformly sampled from the stream.
     * If fewer than k items have been seen, returns all of them.
     *
     * @return array of sampled items
     * @throws IllegalStateException if the sampler has been closed
     */
    public byte[][] sample() {
        checkAlive();
        return SketchOxideNative.reservoirsampling_sample(nativePtr);
    }

    /**
     * Get the current sample as strings.
     *
     * Convenience method that converts sampled bytes to strings.
     *
     * @return array of sampled items as strings
     */
    public String[] sampleAsStrings() {
        byte[][] bytes = sample();
        String[] result = new String[bytes.length];
        for (int i = 0; i < bytes.length; i++) {
            result[i] = new String(bytes[i]);
        }
        return result;
    }

    /**
     * Get the number of items currently in the reservoir.
     *
     * @return number of items in sample (at most k)
     */
    public int len() {
        byte[][] s = sample();
        return s.length;
    }

    /**
     * Get the reservoir capacity.
     *
     * @return the k parameter
     */
    public long capacity() {
        return k;
    }

    /**
     * Get the total count of items seen.
     *
     * @return total number of items added to the stream
     * @throws IllegalStateException if the sampler has been closed
     */
    public long count() {
        checkAlive();
        return SketchOxideNative.reservoirsampling_count(nativePtr);
    }

    /**
     * Check if the reservoir is full.
     *
     * @return true if at least k items have been seen
     */
    public boolean isFull() {
        return count() >= k;
    }

    /**
     * Get the probability that any given item is in the sample.
     *
     * @return k/n where n is the number of items seen
     */
    public double getSamplingProbability() {
        long n = count();
        if (n == 0) return 0.0;
        return Math.min(1.0, (double) k / n);
    }

    /**
     * Serialize the sampler to binary format.
     *
     * @return binary representation of the sampler
     * @throws IllegalStateException if the sampler has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.reservoirsampling_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new ReservoirSampling instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static ReservoirSampling deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.reservoirsampling_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized ReservoirSampling data");
        }

        // Create with dummy value, then replace pointer
        ReservoirSampling sampler = new ReservoirSampling(100);
        SketchOxideNative.reservoirsampling_free(sampler.nativePtr);
        sampler.nativePtr = ptr;
        return sampler;
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
            SketchOxideNative.reservoirsampling_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("ReservoirSampling(k=%d, count=%d, sampleSize=%d)",
                        k, count(), len());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "ReservoirSampling(closed)";
    }
}
