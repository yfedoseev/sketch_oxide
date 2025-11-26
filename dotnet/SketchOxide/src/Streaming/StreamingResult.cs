using System;

namespace SketchOxide.Streaming;

/// <summary>
/// Represents a streaming count result with estimate and error bounds.
///
/// Provides the estimated count along with upper and lower bounds based on
/// the epsilon error parameter of the streaming sketch.
/// </summary>
public readonly struct StreamingResult
{
    /// <summary>
    /// The estimated count value.
    /// </summary>
    public ulong Estimate { get; }

    /// <summary>
    /// The lower bound on the true count based on epsilon error.
    /// </summary>
    public ulong LowerBound { get; }

    /// <summary>
    /// The upper bound on the true count based on epsilon error.
    /// </summary>
    public ulong UpperBound { get; }

    /// <summary>
    /// The epsilon (error) parameter used for computing bounds.
    /// </summary>
    public double Epsilon { get; }

    /// <summary>
    /// Creates a new streaming result with bounds.
    /// </summary>
    /// <param name="estimate">The estimated count.</param>
    /// <param name="epsilon">The epsilon error parameter.</param>
    public StreamingResult(ulong estimate, double epsilon)
    {
        Estimate = estimate;
        Epsilon = epsilon;

        // Compute bounds: estimate * (1 +/- epsilon)
        double margin = estimate * epsilon;
        LowerBound = (ulong)Math.Max(0, estimate - margin);
        UpperBound = (ulong)(estimate + margin);
    }

    /// <summary>
    /// Creates a new streaming result with explicit bounds.
    /// </summary>
    /// <param name="estimate">The estimated count.</param>
    /// <param name="lowerBound">The lower bound.</param>
    /// <param name="upperBound">The upper bound.</param>
    /// <param name="epsilon">The epsilon error parameter.</param>
    public StreamingResult(ulong estimate, ulong lowerBound, ulong upperBound, double epsilon)
    {
        Estimate = estimate;
        LowerBound = lowerBound;
        UpperBound = upperBound;
        Epsilon = epsilon;
    }

    /// <summary>
    /// Gets the relative error as a fraction of the estimate.
    /// </summary>
    /// <returns>The relative error, or 0 if estimate is 0.</returns>
    public double RelativeError => Estimate > 0 ? (double)(UpperBound - LowerBound) / (2 * Estimate) : 0;

    /// <summary>
    /// Returns a string representation of the result.
    /// </summary>
    public override string ToString() => $"StreamingResult(estimate={Estimate}, bounds=[{LowerBound}, {UpperBound}], epsilon={Epsilon})";
}
