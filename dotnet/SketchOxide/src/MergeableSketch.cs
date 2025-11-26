namespace SketchOxide;

/// <summary>
/// Interface for sketch data structures that support merging.
/// Allows combining multiple sketches into a single sketch for distributed aggregation.
/// </summary>
/// <typeparam name="T">The sketch type that can be merged.</typeparam>
public interface IMergeableSketch<in T> where T : class
{
    /// <summary>
    /// Merges another sketch of the same type into this sketch.
    /// </summary>
    /// <param name="other">The sketch to merge. Must be the same type and compatible configuration.</param>
    /// <exception cref="ArgumentNullException">Thrown if other is null.</exception>
    /// <exception cref="ArgumentException">Thrown if sketches are incompatible (e.g., different precision).</exception>
    /// <exception cref="ObjectDisposedException">Thrown if either sketch has been disposed.</exception>
    void Merge(T other);
}
