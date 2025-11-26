using System;
using System.Text;

namespace SketchOxide.Frequency;

/// <summary>
/// Represents an item with its estimated frequency from the Space Saving algorithm.
///
/// Used by <see cref="SpaceSaving.TopK()"/> to return the most frequent items
/// and their estimated counts.
/// </summary>
public readonly struct FrequentItem
{
    /// <summary>
    /// The item bytes.
    /// </summary>
    public byte[] Item { get; }

    /// <summary>
    /// The estimated frequency count of the item.
    /// </summary>
    public ulong Count { get; }

    /// <summary>
    /// Creates a new frequent item entry.
    /// </summary>
    /// <param name="item">The item bytes.</param>
    /// <param name="count">The estimated frequency count.</param>
    /// <exception cref="ArgumentNullException">Thrown if item is null.</exception>
    public FrequentItem(byte[] item, ulong count)
    {
        Item = item ?? throw new ArgumentNullException(nameof(item));
        Count = count;
    }

    /// <summary>
    /// Gets the item as a UTF-8 string.
    /// </summary>
    /// <returns>The item decoded as a UTF-8 string.</returns>
    public string GetItemAsString() => Encoding.UTF8.GetString(Item);

    /// <summary>
    /// Returns a string representation of this frequent item.
    /// </summary>
    public override string ToString()
    {
        try
        {
            return $"FrequentItem(item=\"{GetItemAsString()}\", count={Count})";
        }
        catch
        {
            return $"FrequentItem(item=[{Item.Length} bytes], count={Count})";
        }
    }

    /// <summary>
    /// Determines whether this instance equals another FrequentItem.
    /// </summary>
    public bool Equals(FrequentItem other)
    {
        if (Count != other.Count) return false;
        if (Item == null && other.Item == null) return true;
        if (Item == null || other.Item == null) return false;
        if (Item.Length != other.Item.Length) return false;
        for (int i = 0; i < Item.Length; i++)
        {
            if (Item[i] != other.Item[i]) return false;
        }
        return true;
    }

    /// <summary>
    /// Determines whether this instance equals another object.
    /// </summary>
    public override bool Equals(object? obj)
    {
        return obj is FrequentItem other && Equals(other);
    }

    /// <summary>
    /// Returns a hash code for this instance.
    /// </summary>
    public override int GetHashCode()
    {
        unchecked
        {
            int hash = (int)Count;
            if (Item != null)
            {
                foreach (byte b in Item)
                {
                    hash = (hash * 31) ^ b;
                }
            }
            return hash;
        }
    }

    /// <summary>
    /// Equality operator.
    /// </summary>
    public static bool operator ==(FrequentItem left, FrequentItem right) => left.Equals(right);

    /// <summary>
    /// Inequality operator.
    /// </summary>
    public static bool operator !=(FrequentItem left, FrequentItem right) => !left.Equals(right);
}
