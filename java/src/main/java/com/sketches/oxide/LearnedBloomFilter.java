package com.sketches.oxide;

import java.util.List;

/**
 * LearnedBloomFilter: ML-Enhanced Membership Testing
 *
 * <p><b>EXPERIMENTAL FEATURE - Use with caution in production systems.</b></p>
 *
 * <p>A Learned Bloom Filter uses machine learning to predict set membership,
 * achieving 70-80% memory reduction compared to standard Bloom filters.</p>
 *
 * <h2>How It Works</h2>
 * <ol>
 *   <li><b>Feature Extraction</b>: Extract features from keys (hash patterns, bit distributions)</li>
 *   <li><b>ML Model</b>: Simple linear model (logistic regression) learns key patterns</li>
 *   <li><b>Backup Filter</b>: Small Bloom filter guarantees zero false negatives</li>
 *   <li><b>Query</b>: Model predicts membership; backup filter ensures correctness</li>
 * </ol>
 *
 * <h2>Architecture</h2>
 * <pre>
 * Query Key
 *    |
 *    v
 * Feature Extractor -&gt; Features
 *    |
 *    v
 * Linear Model -&gt; Prediction
 *    |
 *    v
 * If Predicted Positive -&gt; Check Backup Filter
 * If Predicted Negative -&gt; Return False
 * </pre>
 *
 * <h2>Memory Savings</h2>
 * <ul>
 *   <li>Traditional Bloom: ~10 bits/element at 1% FPR</li>
 *   <li>Learned Bloom: ~3-4 bits/element (70-80% reduction)</li>
 *   <li>Model is tiny (few KB), backup filter is small</li>
 * </ul>
 *
 * <h2>Security Warning</h2>
 * <p><b>ML models can be adversarially attacked. Do NOT use in security-critical
 * applications where an attacker could craft keys to fool the model.</b></p>
 *
 * <h2>Reproducibility</h2>
 * <p>Model training is deterministic given the same training data and FPR.</p>
 *
 * <h2>Example Usage</h2>
 * <pre>
 * // Train on a dataset
 * List&lt;byte[]&gt; trainingKeys = new ArrayList&lt;&gt;();
 * for (int i = 0; i &lt; 10000; i++) {
 *     trainingKeys.add(("key" + i).getBytes());
 * }
 *
 * try (LearnedBloomFilter filter = new LearnedBloomFilter(trainingKeys, 0.01)) {
 *     // Query
 *     assert filter.contains("key500".getBytes());
 *     assert !filter.contains("nonexistent".getBytes()); // Probably false
 *
 *     // Check memory savings
 *     long mem = filter.memoryUsage();
 *     System.out.println("Memory: " + mem + " bytes");
 * }
 * </pre>
 *
 * <p><b>Thread Safety:</b> This class is thread-safe for read operations after construction.
 * The filter is immutable once built.</p>
 *
 * <h2>When to Use</h2>
 * <ul>
 *   <li>Memory-constrained environments where 70-80% savings matter</li>
 *   <li>Non-adversarial workloads (internal caches, CDN, databases)</li>
 *   <li>Datasets with learnable patterns (not pure random data)</li>
 * </ul>
 *
 * <h2>When NOT to Use</h2>
 * <ul>
 *   <li>Security-critical applications (authentication, access control)</li>
 *   <li>Adversarial environments (web-facing, user-controlled inputs)</li>
 *   <li>Real-time systems requiring guaranteed performance</li>
 * </ul>
 */
public class LearnedBloomFilter implements AutoCloseable {

    static {
        System.loadLibrary("sketch_oxide_jni");
    }

    private long nativeHandle;
    private boolean closed = false;

    /**
     * Creates a new Learned Bloom Filter.
     *
     * <p><b>Important:</b> All keys in trainingKeys must be members of the set.
     * The model trains on these positive examples.</p>
     *
     * @param trainingKeys Keys to train the model on (must be members)
     * @param fpr Target false positive rate (e.g., 0.01 for 1%)
     * @throws IllegalArgumentException if trainingKeys is empty, too small (less than 10), or fpr is invalid
     * @throws RuntimeException if filter creation fails
     */
    public LearnedBloomFilter(List<byte[]> trainingKeys, double fpr) {
        if (trainingKeys == null || trainingKeys.isEmpty()) {
            throw new IllegalArgumentException("Training keys cannot be null or empty");
        }
        if (trainingKeys.size() < 10) {
            throw new IllegalArgumentException("Need at least 10 training samples for stable model");
        }
        if (fpr <= 0.0 || fpr >= 1.0) {
            throw new IllegalArgumentException("FPR must be in range (0.0, 1.0)");
        }

        this.nativeHandle = nativeNew(trainingKeys, fpr);
        if (this.nativeHandle == 0) {
            throw new RuntimeException("Failed to create LearnedBloomFilter");
        }
    }

    /**
     * Checks if a key might be in the set.
     *
     * <p><b>Guarantees:</b></p>
     * <ul>
     *   <li><b>Zero false negatives</b>: All training keys will return true</li>
     *   <li>False positive rate approximately matches target FPR</li>
     * </ul>
     *
     * @param key The key to check
     * @return true if key is definitely in the set (if it was in training data)
     *         or might be in the set (false positive),
     *         false if definitely not in the set
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
     * Returns memory usage in bytes.
     *
     * <p>Includes:</p>
     * <ul>
     *   <li>Linear model weights</li>
     *   <li>Backup Bloom filter</li>
     *   <li>Feature extractor metadata</li>
     * </ul>
     *
     * @return Memory usage in bytes
     * @throws IllegalStateException if the filter is closed
     */
    public long memoryUsage() {
        checkNotClosed();
        return nativeMemoryUsage(nativeHandle);
    }

    /**
     * Returns the target false positive rate.
     *
     * @return Target FPR
     * @throws IllegalStateException if the filter is closed
     */
    public double fpr() {
        checkNotClosed();
        return nativeFpr(nativeHandle);
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
            throw new IllegalStateException("LearnedBloomFilter has been closed");
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
    private static native long nativeNew(List<byte[]> trainingData, double fpr);
    private static native boolean nativeContains(long handle, byte[] key);
    private static native long nativeMemoryUsage(long handle);
    private static native double nativeFpr(long handle);
    private static native void nativeFree(long handle);
}
