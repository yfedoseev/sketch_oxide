using System;

namespace SketchOxide.Frequency;

/// <summary>
/// Represents the frequency bounds for a queried item.
///
/// Provides lower and upper bounds on the true frequency of an item,
/// as returned by <see cref="FrequentItems.Query(ReadOnlySpan{byte})"/>.
/// The true frequency is guaranteed to be within [LowerBound, UpperBound].
/// </summary>
public readonly struct FrequencyBounds : IEquatable<FrequencyBounds>
{
    /// <summary>
    /// The lower bound on the item's true frequency.
    /// The true count is guaranteed to be at least this value.
    /// </summary>
    public ulong LowerBound { get; }

    /// <summary>
    /// The upper bound on the item's true frequency.
    /// The true count is guaranteed to be at most this value.
    /// </summary>
    public ulong UpperBound { get; }

    /// <summary>
    /// Creates new frequency bounds.
    /// </summary>
    /// <param name="lowerBound">The lower bound estimate.</param>
    /// <param name="upperBound">The upper bound estimate.</param>
    /// <exception cref="ArgumentException">Thrown if lowerBound > upperBound.</exception>
    public FrequencyBounds(ulong lowerBound, ulong upperBound)
    {
        if (lowerBound > upperBound)
            throw new ArgumentException($"Lower bound ({lowerBound}) cannot exceed upper bound ({upperBound})");

        LowerBound = lowerBound;
        UpperBound = upperBound;
    }

    /// <summary>
    /// Gets the midpoint estimate between lower and upper bounds.
    /// This is a reasonable point estimate when the exact value is unknown.
    /// </summary>
    public ulong MidpointEstimate => (LowerBound + UpperBound) / 2;

    /// <summary>
    /// Gets the range (width) of the bounds.
    /// Smaller ranges indicate more precise estimates.
    /// </summary>
    public ulong Range => UpperBound - LowerBound;

    /// <summary>
    /// Gets the relative error as a fraction of the midpoint estimate.
    /// Returns 0 if the midpoint is 0.
    /// </summary>
    public double RelativeError
    {
        get
        {
            ulong mid = MidpointEstimate;
            return mid > 0 ? (double)Range / (2 * mid) : 0;
        }
    }

    /// <summary>
    /// Checks if the bounds contain the exact value (i.e., LowerBound == UpperBound).
    /// </summary>
    public bool IsExact => LowerBound == UpperBound;

    /// <summary>
    /// Returns a string representation of the bounds.
    /// </summary>
    public override string ToString() => $"FrequencyBounds[{LowerBound}, {UpperBound}]";

    /// <summary>
    /// Determines whether this instance equals another FrequencyBounds.
    /// </summary>
    public bool Equals(FrequencyBounds other)
    {
        return LowerBound == other.LowerBound && UpperBound == other.UpperBound;
    }

    /// <summary>
    /// Determines whether this instance equals another object.
    /// </summary>
    public override bool Equals(object? obj)
    {
        return obj is FrequencyBounds other && Equals(other);
    }

    /// <summary>
    /// Returns a hash code for this instance.
    /// </summary>
    public override int GetHashCode()
    {
        return HashCode.Combine(LowerBound, UpperBound);
    }

    /// <summary>
    /// Equality operator.
    /// </summary>
    public static bool operator ==(FrequencyBounds left, FrequencyBounds right) => left.Equals(right);

    /// <summary>
    /// Inequality operator.
    /// </summary>
    public static bool operator !=(FrequencyBounds left, FrequencyBounds right) => !left.Equals(right);
}
