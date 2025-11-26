using System;
using System.Text;
using SketchOxide.Native;

namespace SketchOxide.Membership;

/// <summary>
/// Ribbon Filter for space-efficient membership testing with two-phase construction.
///
/// A modern filter design that uses Ribbon (Rapid Incremental Boolean Banding ON the fly)
/// construction for excellent space efficiency. Elements are first inserted, then the
/// filter is finalized with a Build() call before queries can be made.
/// </summary>
/// <remarks>
/// <para>
/// The Ribbon filter provides:
/// - Excellent space efficiency (close to theoretical minimum)
/// - Two-phase construction: Insert elements, then Build to finalize
/// - Fast queries after construction
/// - No deletion support
/// - Configurable false positive rate
/// </para>
/// <para>
/// Usage pattern:
/// <code>
/// var filter = new RibbonFilter(1000, 0.01);
/// filter.Insert(data1);
/// filter.Insert(data2);
/// // ... insert all elements
/// bool success = filter.Build(); // Finalize the filter
/// if (success)
/// {
///     bool found = filter.Contains(query);
/// }
/// </code>
/// </para>
/// <para>
/// Ideal for applications where:
/// - Space efficiency is critical
/// - Two-phase construction is acceptable
/// - Filter is built once and queried many times
/// </para>
/// </remarks>
public sealed class RibbonFilter : NativeSketch
{
    private readonly ulong _size;
    private readonly double _fpr;
    private bool _isBuilt;

    /// <summary>
    /// Creates a new Ribbon filter with the specified expected number of elements and false positive rate.
    /// </summary>
    /// <param name="expectedElements">Expected number of elements to be inserted. Must be greater than 0.</param>
    /// <param name="falsePositiveRate">Desired false positive rate (e.g., 0.01 for 1%). Must be in range (0, 1).</param>
    /// <exception cref="ArgumentOutOfRangeException">Thrown if expectedElements is 0 or falsePositiveRate is outside (0, 1).</exception>
    /// <exception cref="OutOfMemoryException">Thrown if native memory allocation fails.</exception>
    public RibbonFilter(ulong expectedElements, double falsePositiveRate)
    {
        if (expectedElements == 0)
            throw new ArgumentOutOfRangeException(nameof(expectedElements), expectedElements, "Expected elements must be greater than 0");
        if (falsePositiveRate <= 0 || falsePositiveRate >= 1)
            throw new ArgumentOutOfRangeException(nameof(falsePositiveRate), falsePositiveRate, "False positive rate must be in range (0, 1)");

        _size = expectedElements;
        _fpr = falsePositiveRate;
        _isBuilt = false;
        NativePtr = SketchOxideNative.ribbonfilter_new(expectedElements, falsePositiveRate);

        if (NativePtr == 0)
            throw new OutOfMemoryException("Failed to allocate native RibbonFilter");
    }

    /// <summary>
    /// Private constructor for deserialization.
    /// </summary>
    private RibbonFilter(ulong size, double fpr, bool isBuilt, nuint ptr)
    {
        _size = size;
        _fpr = fpr;
        _isBuilt = isBuilt;
        NativePtr = ptr;
    }

    /// <summary>
    /// Gets the expected number of elements this filter was configured for.
    /// </summary>
    public ulong ExpectedElements
    {
        get
        {
            CheckAlive();
            return _size;
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
    /// Gets whether the filter has been built and is ready for queries.
    /// </summary>
    public bool IsBuilt
    {
        get
        {
            CheckAlive();
            return _isBuilt;
        }
    }

    /// <summary>
    /// Inserts an element into the filter during the construction phase.
    /// </summary>
    /// <param name="data">The bytes representing the element to insert.</param>
    /// <returns>
    /// <c>true</c> if the element was successfully inserted;
    /// <c>false</c> if insertion failed (e.g., filter already built or too many elements).
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    /// <exception cref="InvalidOperationException">Thrown if Build() has already been called.</exception>
    public bool Insert(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));
        if (_isBuilt) throw new InvalidOperationException("Cannot insert elements after Build() has been called");

        unsafe
        {
            fixed (byte* ptr = data)
            {
                return SketchOxideNative.ribbonfilter_insert(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Inserts a string element into the filter during the construction phase.
    /// </summary>
    /// <param name="value">The string to insert.</param>
    /// <returns>
    /// <c>true</c> if the element was successfully inserted;
    /// <c>false</c> if insertion failed.
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    /// <exception cref="InvalidOperationException">Thrown if Build() has already been called.</exception>
    public bool Insert(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Insert(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Finalizes the filter construction, making it ready for queries.
    /// </summary>
    /// <returns>
    /// <c>true</c> if the filter was successfully built;
    /// <c>false</c> if construction failed.
    /// </returns>
    /// <remarks>
    /// After calling Build(), no more elements can be inserted.
    /// The filter must be built before Contains() can be called.
    /// </remarks>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    /// <exception cref="InvalidOperationException">Thrown if Build() has already been called.</exception>
    public bool Build()
    {
        CheckAlive();
        if (_isBuilt) throw new InvalidOperationException("Filter has already been built");

        bool success = SketchOxideNative.ribbonfilter_build(NativePtr);
        if (success)
        {
            _isBuilt = true;
        }
        return success;
    }

    /// <summary>
    /// Tests whether an element might be in the set.
    /// </summary>
    /// <param name="data">The bytes representing the element to query.</param>
    /// <returns>
    /// <c>true</c> if the element might be in the set (may be a false positive);
    /// <c>false</c> if the element is definitely not in the set (no false negatives).
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    /// <exception cref="InvalidOperationException">Thrown if Build() has not been called yet.</exception>
    public bool Contains(ReadOnlySpan<byte> data)
    {
        CheckAlive();
        if (data == null) throw new ArgumentNullException(nameof(data));
        if (!_isBuilt) throw new InvalidOperationException("Filter must be built before querying. Call Build() first.");

        unsafe
        {
            fixed (byte* ptr = data)
            {
                return SketchOxideNative.ribbonfilter_contains(NativePtr, new Span<byte>(ptr, data.Length).ToArray(), (ulong)data.Length);
            }
        }
    }

    /// <summary>
    /// Tests whether a string element might be in the set.
    /// </summary>
    /// <param name="value">The string to query.</param>
    /// <returns>
    /// <c>true</c> if the element might be in the set (may be a false positive);
    /// <c>false</c> if the element is definitely not in the set (no false negatives).
    /// </returns>
    /// <exception cref="ArgumentNullException">Thrown if value is null.</exception>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    /// <exception cref="InvalidOperationException">Thrown if Build() has not been called yet.</exception>
    public bool Contains(string value)
    {
        if (value == null) throw new ArgumentNullException(nameof(value));
        return Contains(Encoding.UTF8.GetBytes(value));
    }

    /// <summary>
    /// Serializes the filter to a byte array.
    /// </summary>
    /// <returns>Serialized filter bytes.</returns>
    /// <exception cref="ObjectDisposedException">Thrown if the filter is disposed.</exception>
    public byte[] Serialize()
    {
        CheckAlive();
        return SketchOxideNative.ribbonfilter_serialize(NativePtr, out _);
    }

    /// <summary>
    /// Deserializes a Ribbon filter from a byte array.
    /// </summary>
    /// <param name="data">Serialized filter bytes.</param>
    /// <param name="expectedElements">The expected elements parameter used when creating the original filter.</param>
    /// <param name="falsePositiveRate">The false positive rate parameter used when creating the original filter.</param>
    /// <param name="wasBuilt">Whether the original filter had been built before serialization.</param>
    /// <returns>A new RibbonFilter instance.</returns>
    /// <exception cref="ArgumentNullException">Thrown if data is null.</exception>
    /// <exception cref="ArgumentException">Thrown if data is invalid.</exception>
    public static RibbonFilter Deserialize(byte[] data, ulong expectedElements, double falsePositiveRate, bool wasBuilt = true)
    {
        if (data == null) throw new ArgumentNullException(nameof(data));

        nuint ptr = SketchOxideNative.ribbonfilter_deserialize(data, (ulong)data.Length);
        if (ptr == 0)
            throw new ArgumentException("Failed to deserialize RibbonFilter: invalid data");

        return new RibbonFilter(expectedElements, falsePositiveRate, wasBuilt, ptr);
    }

    /// <summary>
    /// Returns a string representation of the filter.
    /// </summary>
    public override string ToString()
    {
        if (IsDisposed)
            return "RibbonFilter(disposed)";
        return $"RibbonFilter(expectedElements={_size}, fpr={_fpr:P2}, built={_isBuilt})";
    }

    /// <summary>
    /// Frees the native Rust instance.
    /// </summary>
    protected override void FreeNative()
    {
        if (NativePtr != 0)
        {
            SketchOxideNative.ribbonfilter_free(NativePtr);
            NativePtr = 0;
        }
    }
}
