using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Membership;

/// <summary>
/// LearnedBloomFilter: ML-Enhanced Membership Testing.
///
/// **EXPERIMENTAL FEATURE** - Use with caution in production systems.
///
/// A Learned Bloom Filter uses machine learning to predict set membership,
/// achieving 70-80% memory reduction compared to standard Bloom filters.
/// </summary>
/// <remarks>
/// How It Works:
/// 1. Feature Extraction: Extract features from keys (hash patterns, bit distributions)
/// 2. ML Model: Simple linear model (logistic regression) learns key patterns
/// 3. Backup Filter: Small Bloom filter guarantees zero false negatives
/// 4. Query: Model predicts membership; backup filter ensures correctness
///
/// Memory Savings:
/// - Traditional Bloom: ~10 bits/element at 1% FPR
/// - Learned Bloom: ~3-4 bits/element (70-80% reduction)
/// - Model is tiny (few KB), backup filter is small
///
/// SECURITY WARNING:
/// ML models can be adversarially attacked. Do NOT use in security-critical
/// applications where an attacker could craft keys to fool the model.
///
/// Reproducibility:
/// Model training is deterministic given the same training data and FPR.
/// </remarks>
public sealed class LearnedBloomFilter : NativeSketch
{
    private readonly double _fpr;
    private readonly int _numTrainingKeys;

    /// <summary>
    /// Creates a new Learned Bloom Filter from training data.
    /// </summary>
    /// <param name="trainingKeys">Keys to train the model on (must be members).</param>
    /// <param name="fpr">Target false positive rate (e.g., 0.01 for 1%).</param>
    /// <exception cref="ArgumentNullException">Thrown if trainingKeys is null.</exception>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if parameters are invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public unsafe LearnedBloomFilter(IEnumerable<byte[]> trainingKeys, double fpr)
    {
        if (trainingKeys == null)
            throw new ArgumentNullException(nameof(trainingKeys));

        var keysList = trainingKeys.ToList();
        if (keysList.Count == 0)
            throw new ArgumentOutOfRangeException(nameof(trainingKeys), "Training keys cannot be empty");
        if (keysList.Count < 10)
            throw new ArgumentOutOfRangeException(nameof(trainingKeys), keysList.Count,
                "Must have at least 10 samples for stable model");
        if (fpr <= 0 || fpr >= 1)
            throw new ArgumentOutOfRangeException(nameof(fpr), fpr, "FPR must be in (0, 1)");

        _fpr = fpr;
        _numTrainingKeys = keysList.Count;

        // Prepare pointers for FFI call
        byte*[] keyPointers = new byte*[keysList.Count];
        ulong[] keyLengths = new ulong[keysList.Count];

        // Pin all training keys and collect their pointers
        var handles = new System.Runtime.InteropServices.GCHandle[keysList.Count];
        try
        {
            for (int i = 0; i < keysList.Count; i++)
            {
                handles[i] = System.Runtime.InteropServices.GCHandle.Alloc(
                    keysList[i], System.Runtime.InteropServices.GCHandleType.Pinned);
                keyPointers[i] = (byte*)handles[i].AddrOfPinnedObject();
                keyLengths[i] = (ulong)keysList[i].Length;
            }

            // Call FFI
            fixed (byte** keysPtr = keyPointers)
            fixed (ulong* lengthsPtr = keyLengths)
            {
                NativePtr = SketchOxideNative.learned_bloom_new(
                    keysPtr, lengthsPtr, (ulong)keysList.Count, fpr);
            }
        }
        finally
        {
            // Unpin all handles
            foreach (var handle in handles)
            {
                if (handle.IsAllocated)
                    handle.Free();
            }
        }

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native LearnedBloomFilter");
    }

    /// <summary>
    /// Creates a new Learned Bloom Filter from string training data.
    /// </summary>
    /// <param name="trainingKeys">String keys to train on.</param>
    /// <param name="fpr">Target false positive rate.</param>
    public LearnedBloomFilter(IEnumerable<string> trainingKeys, double fpr)
        : this(trainingKeys.Select(k => Encoding.UTF8.GetBytes(k)), fpr)
    {
    }

    /// <summary>
    /// Gets the target false positive rate.
    /// </summary>
    public double ExpectedFpr
    {
        get
        {
            CheckAlive();
            return SketchOxideNative.learned_bloom_fpr(NativePtr);
        }
    }

    /// <summary>
    /// Gets the number of training keys used.
    /// </summary>
    public int NumTrainingKeys
    {
        get
        {
            CheckAlive();
            return _numTrainingKeys;
        }
    }

    /// <summary>
    /// Checks if an item may be present in the filter.
    /// </summary>
    /// <param name="data">The item bytes to check.</param>
    /// <returns>True if the item may be present (with FPR probability), false if definitely not present.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public unsafe bool Contains(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            return SketchOxideNative.learned_bloom_contains(NativePtr, ptr, (ulong)data.Length);
        }
    }

    /// <summary>
    /// Checks if a string item may be present in the filter.
    /// </summary>
    /// <param name="value">The string to check.</param>
    /// <returns>True if the item may be present, false if definitely not present.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool Contains(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Contains(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Gets the memory usage in bytes.
    /// </summary>
    /// <returns>Total memory usage including model and backup filter.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public ulong MemoryUsage()
    {
        CheckAlive();
        return SketchOxideNative.learned_bloom_memory_usage(NativePtr);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "LearnedBloomFilter(disposed)";
        return $"LearnedBloomFilter(trainingKeys={_numTrainingKeys}, fpr={_fpr:F4}, EXPERIMENTAL)";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.learned_bloom_free(NativePtr);
            NativePtr = 0;
        }
    }
}
