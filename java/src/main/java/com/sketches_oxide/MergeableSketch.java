package com.sketches_oxide;

/**
 * Interface for sketches that support merging.
 *
 * Defines the merge contract for combining multiple sketch instances.
 * This is useful for distributed computing where sketches are computed
 * on different nodes and need to be aggregated.
 *
 * @param <T> the concrete sketch type
 */
public interface MergeableSketch<T> {
    /**
     * Merge another sketch into this one.
     *
     * After merging, this sketch represents the combined state of both sketches.
     * The other sketch is not modified.
     *
     * @param other the sketch to merge
     * @throws IllegalStateException if either sketch is closed
     * @throws IllegalArgumentException if the sketches are incompatible
     *                                  (e.g., different precision levels)
     * @throws NullPointerException if other is null
     */
    void merge(T other);
}
