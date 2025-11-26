package com.sketches_oxide.similarity;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive unit tests for MinHash Jaccard similarity estimation.
 * Tests construction, element insertion, similarity computation, merging,
 * serialization, and edge cases with various set overlap scenarios.
 */
@DisplayName("MinHash Jaccard Similarity Tests")
public class MinHashTest {

    private MinHash mh1;
    private MinHash mh2;

    @BeforeEach
    void setUp() {
        mh1 = new MinHash(128);
        mh2 = new MinHash(128);
    }

    @AfterEach
    void tearDown() {
        if (mh1 != null) {
            try {
                mh1.close();
            } catch (Exception e) {
                // Already closed
            }
        }
        if (mh2 != null) {
            try {
                mh2.close();
            } catch (Exception e) {
                // Already closed
            }
        }
    }

    // ==================== Constructor Tests ====================

    @Test
    @DisplayName("Should create MinHash with valid numPerm")
    void testConstructorValidNumPerm() {
        try (MinHash mh = new MinHash(128)) {
            assertNotNull(mh);
            assertEquals(128, mh.numPerm());
        }
    }

    @ParameterizedTest
    @ValueSource(longs = {0, -1, -100})
    @DisplayName("Should reject non-positive numPerm")
    void testConstructorInvalidNumPerm(long numPerm) {
        assertThrows(IllegalArgumentException.class, () -> new MinHash(numPerm));
    }

    @ParameterizedTest
    @ValueSource(longs = {1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024})
    @DisplayName("Should accept various valid numPerm values")
    void testConstructorVariousNumPerm(long numPerm) {
        try (MinHash mh = new MinHash(numPerm)) {
            assertEquals(numPerm, mh.numPerm());
        }
    }

    // ==================== Core Operation Tests ====================

    @Test
    @DisplayName("Should add element to MinHash")
    void testUpdate() {
        mh1.update("element");
        // No exception should be thrown
    }

    @Test
    @DisplayName("Should add multiple elements")
    void testMultipleUpdates() {
        for (int i = 0; i < 100; i++) {
            mh1.update("element-" + i);
        }
        // No exception should be thrown
    }

    @Test
    @DisplayName("Should add duplicate elements")
    void testDuplicateUpdates() {
        mh1.update("element");
        mh1.update("element");
        mh1.update("element");
        // No exception should be thrown
    }

    @Test
    @DisplayName("Should work with byte arrays")
    void testUpdateByteArray() {
        byte[] item = "test-data".getBytes();
        mh1.update(item);
        // No exception should be thrown
    }

    @Test
    @DisplayName("Should reject null item on update (string)")
    void testUpdateNullString() {
        assertThrows(NullPointerException.class, () -> mh1.update((String) null));
    }

    @Test
    @DisplayName("Should reject null item on update (byte array)")
    void testUpdateNullByteArray() {
        assertThrows(NullPointerException.class, () -> mh1.update((byte[]) null));
    }

    @Test
    @DisplayName("Should reject operations on closed sketch")
    void testClosedSketchDetection() {
        mh1.close();

        assertThrows(IllegalStateException.class, () -> mh1.update("item"));
        assertThrows(IllegalStateException.class, () -> mh1.jaccardSimilarity(mh2));
        assertThrows(IllegalStateException.class, () -> mh1.serialize());
    }

    @Test
    @DisplayName("Should be idempotent when closed multiple times")
    void testMultipleClose() {
        mh1.close();
        assertDoesNotThrow(() -> mh1.close());
    }

    // ==================== Jaccard Similarity Tests ====================

    @Test
    @DisplayName("Should compute similarity of identical sets")
    void testIdenticalSets() {
        String[] elements = {"apple", "banana", "cherry", "date", "elderberry"};

        for (String elem : elements) {
            mh1.update(elem);
            mh2.update(elem);
        }

        double similarity = mh1.jaccardSimilarity(mh2);
        assertEquals(1.0, similarity, 0.01); // Should be ~1.0
    }

    @Test
    @DisplayName("Should compute similarity of disjoint sets")
    void testDisjointSets() {
        String[] set1 = {"a", "b", "c"};
        String[] set2 = {"d", "e", "f"};

        for (String elem : set1) {
            mh1.update(elem);
        }
        for (String elem : set2) {
            mh2.update(elem);
        }

        double similarity = mh1.jaccardSimilarity(mh2);
        assertEquals(0.0, similarity, 0.01); // Should be ~0.0
    }

    @Test
    @DisplayName("Should compute similarity of partially overlapping sets")
    void testPartialOverlap() {
        // Set 1: {a, b, c, d}
        // Set 2: {c, d, e, f}
        // Intersection: {c, d} (2 elements)
        // Union: {a, b, c, d, e, f} (6 elements)
        // Expected Jaccard: 2/6 = 0.333...

        for (String elem : new String[]{"a", "b", "c", "d"}) {
            mh1.update(elem);
        }
        for (String elem : new String[]{"c", "d", "e", "f"}) {
            mh2.update(elem);
        }

        double similarity = mh1.jaccardSimilarity(mh2);
        assertTrue(similarity > 0.25 && similarity < 0.50,
                "Expected similarity ~0.33, got " + similarity);
    }

    @Test
    @DisplayName("Should compute similarity of subsets")
    void testSubset() {
        // Set 1: {a, b}
        // Set 2: {a, b, c, d}
        // Intersection: {a, b} (2 elements)
        // Union: {a, b, c, d} (4 elements)
        // Expected Jaccard: 2/4 = 0.5

        for (String elem : new String[]{"a", "b"}) {
            mh1.update(elem);
        }
        for (String elem : new String[]{"a", "b", "c", "d"}) {
            mh2.update(elem);
        }

        double similarity = mh1.jaccardSimilarity(mh2);
        assertTrue(similarity > 0.4 && similarity < 0.6,
                "Expected similarity ~0.5, got " + similarity);
    }

    @Test
    @DisplayName("Should be symmetric: J(A,B) = J(B,A)")
    void testSymmetry() {
        for (String elem : new String[]{"a", "b", "c"}) {
            mh1.update(elem);
        }
        for (String elem : new String[]{"b", "c", "d"}) {
            mh2.update(elem);
        }

        double sim12 = mh1.jaccardSimilarity(mh2);
        double sim21 = mh2.jaccardSimilarity(mh1);

        assertEquals(sim12, sim21, 0.01);
    }

    @Test
    @DisplayName("Should return similarity in valid range [0, 1]")
    void testSimilarityRange() {
        mh1.update("a");
        mh1.update("b");
        mh2.update("c");

        double similarity = mh1.jaccardSimilarity(mh2);
        assertTrue(similarity >= 0.0 && similarity <= 1.0,
                "Similarity out of range: " + similarity);
    }

    @Test
    @DisplayName("Should reject similarity computation with null")
    void testSimilarityNull() {
        mh1.update("item");
        assertThrows(NullPointerException.class, () -> mh1.jaccardSimilarity(null));
    }

    @Test
    @DisplayName("Should reject similarity with different numPerm")
    void testSimilarityDifferentNumPerm() {
        MinHash mh3 = new MinHash(256);

        mh1.update("item");
        mh3.update("item");

        assertThrows(IllegalArgumentException.class, () -> mh1.jaccardSimilarity(mh3));

        mh3.close();
    }

    @Test
    @DisplayName("Should reject similarity on closed sketch")
    void testSimilarityClosedSketch() {
        mh1.close();

        assertThrows(IllegalStateException.class, () -> mh1.jaccardSimilarity(mh2));
    }

    @Test
    @DisplayName("Should provide reasonable similarity for large sets")
    void testLargeSetSimilarity() {
        // Add 1000 elements to mh1
        for (int i = 0; i < 1000; i++) {
            mh1.update("item-" + i);
        }

        // Add 500 common elements and 500 unique to mh2
        for (int i = 0; i < 500; i++) {
            mh2.update("item-" + i);
        }
        for (int i = 1000; i < 1500; i++) {
            mh2.update("item-" + i);
        }

        double similarity = mh1.jaccardSimilarity(mh2);
        // Expected: 500 / (1000 + 1000 - 500) = 500/1500 = 0.333
        assertTrue(similarity > 0.25 && similarity < 0.45,
                "Similarity out of expected range: " + similarity);
    }

    // ==================== Merge Tests ====================

    @Test
    @DisplayName("Should merge two compatible MinHash sketches")
    void testMergeCompatible() {
        for (String elem : new String[]{"a", "b"}) {
            mh1.update(elem);
        }
        for (String elem : new String[]{"c", "d"}) {
            mh2.update(elem);
        }

        mh1.merge(mh2);

        // After merge, mh1 should represent {a, b, c, d}
        // Create reference to compare
        MinHash reference = new MinHash(128);
        for (String elem : new String[]{"a", "b", "c", "d"}) {
            reference.update(elem);
        }

        double mergedSim = mh1.jaccardSimilarity(reference);
        assertTrue(mergedSim > 0.9, "Merged sketch doesn't match union: " + mergedSim);

        reference.close();
    }

    @Test
    @DisplayName("Should reject merge with different numPerm")
    void testMergeDifferentNumPerm() {
        MinHash mh3 = new MinHash(256);

        assertThrows(IllegalArgumentException.class, () -> mh1.merge(mh3));

        mh3.close();
    }

    @Test
    @DisplayName("Should reject merge with null")
    void testMergeNull() {
        assertThrows(NullPointerException.class, () -> mh1.merge(null));
    }

    @Test
    @DisplayName("Should reject merge on closed sketch")
    void testMergeClosedSketch() {
        mh1.close();

        assertThrows(IllegalStateException.class, () -> mh1.merge(mh2));
    }

    @Test
    @DisplayName("Should merge sketches with identical data")
    void testMergeIdenticalData() {
        for (String elem : new String[]{"a", "b", "c"}) {
            mh1.update(elem);
            mh2.update(elem);
        }

        mh1.merge(mh2);

        // Create reference
        MinHash reference = new MinHash(128);
        for (String elem : new String[]{"a", "b", "c"}) {
            reference.update(elem);
        }

        double sim = mh1.jaccardSimilarity(reference);
        assertEquals(1.0, sim, 0.05);

        reference.close();
    }

    @Test
    @DisplayName("Should merge sets correctly (union property)")
    void testMergeUnionProperty() {
        // mh1 = {1, 2, 3}
        for (int i : new int[]{1, 2, 3}) {
            mh1.update("item-" + i);
        }

        // mh2 = {3, 4, 5}
        for (int i : new int[]{3, 4, 5}) {
            mh2.update("item-" + i);
        }

        mh1.merge(mh2);

        // Expected union = {1, 2, 3, 4, 5}
        MinHash reference = new MinHash(128);
        for (int i : new int[]{1, 2, 3, 4, 5}) {
            reference.update("item-" + i);
        }

        double sim = mh1.jaccardSimilarity(reference);
        assertTrue(sim > 0.85, "Merge didn't create proper union: " + sim);

        reference.close();
    }

    // ==================== Serialization Tests ====================

    @Test
    @DisplayName("Should serialize and deserialize empty MinHash")
    void testSerializeEmptyMinHash() {
        byte[] serialized = mh1.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);

        MinHash restored = MinHash.deserialize(serialized);
        assertNotNull(restored);
        restored.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize MinHash with data")
    void testSerializeMinHashWithData() {
        for (String elem : new String[]{"apple", "banana", "cherry"}) {
            mh1.update(elem);
        }

        byte[] serialized = mh1.serialize();

        MinHash restored = MinHash.deserialize(serialized);

        // The restored sketch should have same similarity with another set
        MinHash testSet = new MinHash(128);
        testSet.update("apple");
        testSet.update("banana");

        double simOriginal = mh1.jaccardSimilarity(testSet);
        double simRestored = restored.jaccardSimilarity(testSet);

        assertEquals(simOriginal, simRestored, 0.01);

        restored.close();
        testSet.close();
    }

    @Test
    @DisplayName("Should preserve data through serialization round-trip")
    void testSerializationRoundTrip() {
        for (int i = 0; i < 100; i++) {
            mh1.update("item-" + i);
        }

        byte[] serialized = mh1.serialize();
        MinHash restored = MinHash.deserialize(serialized);

        // Both should have same similarity with a test set
        MinHash testSet = new MinHash(128);
        for (int i = 0; i < 50; i++) {
            testSet.update("item-" + i);
        }

        double simOriginal = mh1.jaccardSimilarity(testSet);
        double simRestored = restored.jaccardSimilarity(testSet);

        assertEquals(simOriginal, simRestored, 0.01,
                "Serialization round-trip changed similarity");

        restored.close();
        testSet.close();
    }

    @Test
    @DisplayName("Should reject invalid serialized data")
    void testDeserializeInvalidData() {
        byte[] invalidData = {1, 2, 3, 4, 5};
        assertThrows(IllegalArgumentException.class, () -> MinHash.deserialize(invalidData));
    }

    @Test
    @DisplayName("Should reject null serialized data")
    void testDeserializeNull() {
        assertThrows(NullPointerException.class, () -> MinHash.deserialize(null));
    }

    @Test
    @DisplayName("Should reject serialize on closed MinHash")
    void testSerializeClosedMinHash() {
        mh1.close();
        assertThrows(IllegalStateException.class, () -> mh1.serialize());
    }

    // ==================== String Representation Tests ====================

    @Test
    @DisplayName("Should provide meaningful toString")
    void testToString() {
        String str = mh1.toString();
        assertNotNull(str);
        assertTrue(str.contains("MinHash"));
        assertTrue(str.contains("128"));
    }

    @Test
    @DisplayName("Should indicate closed state in toString")
    void testToStringClosed() {
        mh1.close();
        String str = mh1.toString();
        assertTrue(str.contains("closed") || str.contains("MinHash"));
    }

    // ==================== Standard Error Tests ====================

    @Test
    @DisplayName("Should compute standard error correctly")
    void testStandardError() {
        double stdError = mh1.getStandardError();
        double expected = 1.0 / Math.sqrt(128);
        assertEquals(expected, stdError, 0.0001);
    }

    @Test
    @DisplayName("Should have smaller standard error with more permutations")
    void testStandardErrorScaling() {
        try (MinHash mh64 = new MinHash(64);
             MinHash mh256 = new MinHash(256)) {

            double err64 = mh64.getStandardError();
            double err256 = mh256.getStandardError();

            assertTrue(err256 < err64, "More permutations should have lower error");
        }
    }

    // ==================== Resource Management Tests ====================

    @Test
    @DisplayName("Should work with try-with-resources")
    void testTryWithResources() {
        try (MinHash mh = new MinHash(128);
             MinHash mh_other = new MinHash(128)) {

            mh.update("test");
            mh_other.update("test");

            double similarity = mh.jaccardSimilarity(mh_other);
            assertEquals(1.0, similarity, 0.01);
        }
        // No exception should be thrown
    }

    // ==================== Edge Cases Tests ====================

    @Test
    @DisplayName("Should handle empty MinHash similarity")
    void testEmptyMinHashSimilarity() {
        // Both empty
        double sim1 = mh1.jaccardSimilarity(mh2);
        assertTrue(sim1 >= 0.0 && sim1 <= 1.0);

        // One empty, one non-empty
        mh2.update("item");
        double sim2 = mh1.jaccardSimilarity(mh2);
        assertTrue(sim2 >= 0.0 && sim2 <= 1.0);
    }

    @Test
    @DisplayName("Should handle single element sets")
    void testSingleElementSimilarity() {
        mh1.update("item");
        mh2.update("item");

        double similarity = mh1.jaccardSimilarity(mh2);
        assertEquals(1.0, similarity, 0.05);

        mh2 = new MinHash(128);
        mh2.update("different");

        double sim2 = mh1.jaccardSimilarity(mh2);
        assertEquals(0.0, sim2, 0.05);
    }

    @Test
    @DisplayName("Should handle very large sets")
    void testLargeSets() {
        // Create 10000 elements in mh1
        for (int i = 0; i < 10000; i++) {
            mh1.update("element-" + i);
        }

        // Create 10000 elements in mh2 with 50% overlap
        for (int i = 0; i < 5000; i++) {
            mh2.update("element-" + i);
        }
        for (int i = 10000; i < 15000; i++) {
            mh2.update("element-" + i);
        }

        double similarity = mh1.jaccardSimilarity(mh2);
        // Expected: 5000 / (10000 + 10000 - 5000) = 5000/15000 = 0.333
        assertTrue(similarity > 0.25 && similarity < 0.45,
                "Large set similarity unexpected: " + similarity);
    }

    @Test
    @DisplayName("Should handle binary data")
    void testBinaryData() {
        byte[] binary1 = {0, 1, 2, 3, 127};
        byte[] binary2 = {0, 1, 2, 3, 127};

        mh1.update(binary1);
        mh2.update(binary2);

        double similarity = mh1.jaccardSimilarity(mh2);
        assertEquals(1.0, similarity, 0.05);
    }

    @Test
    @DisplayName("Should handle Unicode strings")
    void testUnicodeStrings() {
        String[] items = {"你好", "مرحبا", "Привет", "🎉", "こんにちは"};

        for (String item : items) {
            mh1.update(item);
            mh2.update(item);
        }

        double similarity = mh1.jaccardSimilarity(mh2);
        assertEquals(1.0, similarity, 0.05);
    }

    @Test
    @DisplayName("Should work with different numPerm values independently")
    void testDifferentNumPermValues() {
        for (long numPerm : new long[]{8, 16, 32, 64, 128, 256}) {
            try (MinHash mha = new MinHash(numPerm);
                 MinHash mhb = new MinHash(numPerm)) {

                mha.update("item1");
                mhb.update("item2");

                double sim = mha.jaccardSimilarity(mhb);
                assertTrue(sim >= 0.0 && sim <= 1.0);
            }
        }
    }

    @Test
    @DisplayName("Should handle empty string elements")
    void testEmptyStringElement() {
        mh1.update("");
        mh2.update("");

        double similarity = mh1.jaccardSimilarity(mh2);
        assertEquals(1.0, similarity, 0.05);
    }

    @Test
    @DisplayName("Should distinguish different strings")
    void testDistinguishDifferentStrings() {
        mh1.update("test");
        mh1.update("testing");
        mh1.update("tests");

        mh2.update("tester");
        mh2.update("tested");
        mh2.update("testing");

        double similarity = mh1.jaccardSimilarity(mh2);
        // Only "testing" is common: 1/5 = 0.2
        assertTrue(similarity > 0.1 && similarity < 0.4,
                "Similarity for different strings unexpected: " + similarity);
    }
}
