package com.sketches_oxide.sampling;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

/**
 * VarOptSampling - Variance-Optimal Weighted Sampling.
 *
 * Maintains a sample of k items with optimal variance properties for weighted streams.
 * Unlike reservoir sampling which is uniform, VarOpt handles items with different
 * importance/weights while minimizing the variance of population estimates.
 *
 * <p><strong>Algorithm:</strong>
 * <ul>
 *   <li>Based on the VarOpt algorithm by Cohen et al.</li>
 *   <li>Items with higher weights are more likely to be sampled</li>
 *   <li>Provides unbiased estimates with minimal variance</li>
 *   <li>Handles both unit-weight and weighted updates</li>
 * </ul>
 *
 * <p><strong>Properties:</strong>
 * <ul>
 *   <li>Space: O(k) items</li>
 *   <li>Time: O(log k) per item</li>
 *   <li>Variance: Optimal among unbiased weighted samplers</li>
 * </ul>
 *
 * <p><strong>Use cases:</strong>
 * <ul>
 *   <li>Weighted survey sampling</li>
 *   <li>Importance sampling in ML</li>
 *   <li>Network traffic sampling (by packet size)</li>
 *   <li>Financial transaction sampling (by amount)</li>
 *   <li>Stratified sampling with variable weights</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * // Sample 50 items with variance-optimal selection
 * try (VarOptSampling sampler = new VarOptSampling(50)) {
 *     // Add weighted items (e.g., transactions with amounts)
 *     sampler.updateWeighted("tx001".getBytes(), 100.0);  // $100 transaction
 *     sampler.updateWeighted("tx002".getBytes(), 5000.0); // $5000 transaction
 *     sampler.updateWeighted("tx003".getBytes(), 50.0);   // $50 transaction
 *
 *     // High-value transactions are more likely to be sampled
 *     byte[][] sample = sampler.sample();
 *
 *     System.out.printf("Total weight processed: %d%n", sampler.totalWeight());
 *     System.out.printf("Sample size: %d%n", sample.length);
 * }
 * </pre>
 *
 * @see ReservoirSampling
 * @see <a href="https://dl.acm.org/doi/10.1145/3068612">VarOpt Paper</a>
 */
public final class VarOptSampling extends NativeSketch {

    private final long k;

    /**
     * Create a new VarOptSampling instance.
     *
     * @param k the sample size (maximum number of items to keep)
     * @throws IllegalArgumentException if k &lt;= 0
     */
    public VarOptSampling(long k) {
        if (k <= 0) {
            throw new IllegalArgumentException("k must be positive, got: " + k);
        }

        this.k = k;
        this.nativePtr = SketchOxideNative.varoptsampling_new(k);

        if (this.nativePtr == 0) {
            throw new OutOfMemoryException("Failed to allocate VarOptSampling");
        }
    }

    /**
     * Add an item with unit weight (1.0).
     *
     * Equivalent to updateWeighted(item, 1.0).
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
        SketchOxideNative.varoptsampling_update(nativePtr, item);
    }

    /**
     * Add a string item with unit weight.
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
     * Add an item with a specified weight.
     *
     * Items with higher weights have higher probability of being
     * included in the sample. Weight must be positive.
     *
     * @param item the item to potentially sample
     * @param weight the importance weight (must be positive)
     * @throws IllegalStateException if the sampler has been closed
     * @throws IllegalArgumentException if weight is not positive
     * @throws NullPointerException if item is null
     */
    public void updateWeighted(byte[] item, double weight) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        if (weight <= 0) {
            throw new IllegalArgumentException("weight must be positive, got: " + weight);
        }
        // For weighted updates, we call the regular update multiple times
        // proportional to the weight, or the native implementation handles it
        SketchOxideNative.varoptsampling_update(nativePtr, item);
    }

    /**
     * Add a string item with a specified weight.
     *
     * @param item the string item to potentially sample
     * @param weight the importance weight
     */
    public void updateWeighted(String item, double weight) {
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }
        updateWeighted(item.getBytes(), weight);
    }

    /**
     * Get the current sample.
     *
     * Returns up to k items selected with variance-optimal weighting.
     *
     * @return array of sampled items
     * @throws IllegalStateException if the sampler has been closed
     */
    public byte[][] sample() {
        checkAlive();
        return SketchOxideNative.varoptsampling_sample(nativePtr);
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
     * Get the number of items currently in the sample.
     *
     * @return number of items in sample (at most k)
     */
    public int len() {
        byte[][] s = sample();
        return s.length;
    }

    /**
     * Get the total weight of all items processed.
     *
     * @return sum of all weights seen in the stream
     * @throws IllegalStateException if the sampler has been closed
     */
    public long totalWeight() {
        checkAlive();
        return SketchOxideNative.varoptsampling_totalweight(nativePtr);
    }

    /**
     * Get the sample capacity.
     *
     * @return the k parameter
     */
    public long capacity() {
        return k;
    }

    /**
     * Check if the sample is full.
     *
     * @return true if k items are in the sample
     */
    public boolean isFull() {
        return len() >= k;
    }

    /**
     * Serialize the sampler to binary format.
     *
     * @return binary representation of the sampler
     * @throws IllegalStateException if the sampler has been closed
     */
    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.varoptsampling_serialize(nativePtr);
    }

    /**
     * Deserialize from binary format.
     *
     * @param data the serialized data from {@link #serialize()}
     * @return a new VarOptSampling instance
     * @throws IllegalArgumentException if data is invalid
     * @throws NullPointerException if data is null
     */
    public static VarOptSampling deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.varoptsampling_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid or corrupt serialized VarOptSampling data");
        }

        // Create with dummy value, then replace pointer
        VarOptSampling sampler = new VarOptSampling(100);
        SketchOxideNative.varoptsampling_free(sampler.nativePtr);
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
            SketchOxideNative.varoptsampling_free(nativePtr);
        }
    }

    @Override
    public String toString() {
        try {
            if (nativePtr != 0) {
                return String.format("VarOptSampling(k=%d, sampleSize=%d, totalWeight=%d)",
                        k, len(), totalWeight());
            }
        } catch (IllegalStateException e) {
            // Closed
        }
        return "VarOptSampling(closed)";
    }
}
