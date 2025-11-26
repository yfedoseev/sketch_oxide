using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Membership;

/// <summary>
/// VacuumFilter: Best-in-class dynamic membership filter (VLDB 2020).
///
/// Vacuum filters combine the space efficiency of static filters with the flexibility
/// of dynamic operations (insertions AND deletions). The key innovation is a semi-sorted
/// bucket layout with adaptive fingerprint sizing, achieving superior memory efficiency
/// compared to Cuckoo, Bloom, and Quotient filters.
/// </summary>
/// <remarks>
/// Key Advantages:
/// - Space efficiency: Less than 15 bits/item at 1% FPR (best among dynamic filters)
/// - Cache-optimized: Semi-sorting improves query performance
/// - Predictable performance: No cuckoo evictions, just linear probing
/// - True deletions: Unlike Bloom variants, actual removal without false negatives
/// - Configurable: Tune fingerprint bits vs FPR tradeoff
///
/// Time Complexity:
/// - Insert: O(1) amortized (with rehashing)
/// - Query: O(1) expected
/// - Delete: O(1) expected
/// - Space: O(n) where n is capacity
///
/// References:
/// - Wang et al. "Vacuum Filters: More Space-Efficient and Faster Replacement for Bloom and Cuckoo Filters" (VLDB 2020)
/// </remarks>
public sealed class VacuumFilter : NativeSketch
{
    private readonly ulong _capacity;
    private readonly double _fpr;

    /// <summary>
    /// Creates a new VacuumFilter for membership testing with deletions.
    /// </summary>
    /// <param name="capacity">Expected number of items to store.</param>
    /// <param name="fpr">Target false positive rate (e.g., 0.01 for 1%).</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if parameters are invalid.</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native allocation fails.</exception>
    public VacuumFilter(ulong capacity, double fpr)
    {
        if (capacity == 0)
            throw new ArgumentOutOfRangeException(nameof(capacity), capacity, "Capacity must be greater than 0");
        if (fpr <= 0 || fpr >= 1)
            throw new ArgumentOutOfRangeException(nameof(fpr), fpr, "FPR must be in (0, 1)");

        _capacity = capacity;
        _fpr = fpr;
        NativePtr = SketchOxideNative.vacuum_filter_new(capacity, fpr);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native VacuumFilter");
    }

    /// <summary>
    /// Gets the capacity of the filter (maximum items before rehashing).
    /// </summary>
    public ulong Capacity
    {
        get
        {
            CheckAlive();
            return _capacity;
        }
    }

    /// <summary>
    /// Gets the target false positive rate.
    /// </summary>
    public double FalsePositiveRate
    {
        get
        {
            CheckAlive();
            return _fpr;
        }
    }

    /// <summary>
    /// Inserts an item into the filter.
    /// </summary>
    /// <param name="data">The bytes to insert.</param>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    /// <exception cref="InvalidOperationException">Thrown if insertion fails.</exception>
    public unsafe void Insert(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            int result = SketchOxideNative.vacuum_filter_insert(NativePtr, ptr, (ulong)data.Length);
            if (result != 0)
                throw new InvalidOperationException("Failed to insert item into VacuumFilter");
        }
    }

    /// <summary>
    /// Inserts a string item into the filter.
    /// </summary>
    /// <param name="value">The string to insert.</param>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public void Insert(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        Insert(Encoding.UTF8.GetBytes(value));
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
            return SketchOxideNative.vacuum_filter_contains(NativePtr, ptr, (ulong)data.Length);
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
    /// Deletes an item from the filter.
    /// </summary>
    /// <param name="data">The item bytes to delete.</param>
    /// <returns>True if the item was found and deleted, false if not found.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public unsafe bool Delete(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));

        fixed (byte* ptr = data)
        {
            return SketchOxideNative.vacuum_filter_delete(NativePtr, ptr, (ulong)data.Length);
        }
    }

    /// <summary>
    /// Deletes a string item from the filter.
    /// </summary>
    /// <param name="value">The string to delete.</param>
    /// <returns>True if the item was found and deleted, false if not found.</returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public bool Delete(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Delete(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Gets statistics about the filter.
    /// </summary>
    /// <returns>A tuple containing (capacity, num_items, load_factor, memory_bits).</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public unsafe (ulong Capacity, ulong NumItems, double LoadFactor, ulong MemoryBits) GetStats()
    {
        CheckAlive();

        ulong capacity = 0, numItems = 0, memoryBits = 0;
        double loadFactor = 0.0;

        SketchOxideNative.vacuum_filter_stats(
            NativePtr, &capacity, &numItems, &loadFactor, &memoryBits);

        return (capacity, numItems, loadFactor, memoryBits);
    }

    /// <summary>
    /// Clears all items from the filter.
    /// </summary>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public void Clear()
    {
        CheckAlive();
        SketchOxideNative.vacuum_filter_clear(NativePtr);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "VacuumFilter(disposed)";
        return $"VacuumFilter(capacity={_capacity}, fpr={_fpr:F4})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.vacuum_filter_free(NativePtr);
            NativePtr = 0;
        }
    }
}
