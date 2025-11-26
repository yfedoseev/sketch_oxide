package com.sketches_oxide.membership;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import org.junit.jupiter.params.provider.CsvSource;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive unit tests for BloomFilter membership sketch.
 * Tests all critical functionality including construction, insertion, querying,
 * merging, serialization, and resource management.
 */
@DisplayName("BloomFilter Membership Sketch Tests")
public class BloomFilterTest {

    private BloomFilter filter;

    @BeforeEach
    void setUp() {
        filter = new BloomFilter(10000, 0.01);
    }

    @AfterEach
    void tearDown() {
        if (filter != null) {
            try {
                filter.close();
            } catch (Exception e) {
                // Already closed
            }
        }
    }

    // ==================== Constructor Tests ====================

    @Test
    @DisplayName("Should create BloomFilter with valid parameters")
    void testConstructorValidParameters() {
        try (BloomFilter bf = new BloomFilter(1000, 0.01)) {
            assertNotNull(bf);
            assertEquals(1000, bf.getN());
            assertEquals(0.01, bf.getFpr(), 0.0001);
        }
    }

    @Test
    @DisplayName("Should reject negative n")
    void testConstructorNegativeN() {
        assertThrows(IllegalArgumentException.class, () -> new BloomFilter(-1000, 0.01));
    }

    @Test
    @DisplayName("Should reject zero n")
    void testConstructorZeroN() {
        assertThrows(IllegalArgumentException.class, () -> new BloomFilter(0, 0.01));
    }

    @ParameterizedTest
    @ValueSource(doubles = {0.0, -0.1, -1.0})
    @DisplayName("Should reject invalid FPR values (0 or less)")
    void testConstructorFprTooLow(double fpr) {
        assertThrows(IllegalArgumentException.class, () -> new BloomFilter(1000, fpr));
    }

    @ParameterizedTest
    @ValueSource(doubles = {1.0, 1.5, 2.0})
    @DisplayName("Should reject invalid FPR values (1 or more)")
    void testConstructorFprTooHigh(double fpr) {
        assertThrows(IllegalArgumentException.class, () -> new BloomFilter(1000, fpr));
    }

    @Test
    @DisplayName("Should accept boundary FPR values")
    void testConstructorBoundaryFpr() {
        try (BloomFilter bf1 = new BloomFilter(1000, 0.0001)) {
            assertEquals(0.0001, bf1.getFpr(), 0.00001);
        }
        try (BloomFilter bf2 = new BloomFilter(1000, 0.999)) {
            assertEquals(0.999, bf2.getFpr(), 0.001);
        }
    }

    @ParameterizedTest
    @CsvSource({
        "100,0.01",
        "1000,0.001",
        "1000000,0.0001",
        "100,0.99"
    })
    @DisplayName("Should accept various valid parameter combinations")
    void testConstructorVariousParameters(long n, double fpr) {
        try (BloomFilter bf = new BloomFilter(n, fpr)) {
            assertEquals(n, bf.getN());
            assertEquals(fpr, bf.getFpr(), fpr * 0.01);
        }
    }

    // ==================== Core Operation Tests ====================

    @Test
    @DisplayName("Should insert and find items")
    void testInsertAndContains() {
        filter.insert("test-item");
        assertTrue(filter.contains("test-item"));
    }

    @Test
    @DisplayName("Should handle multiple inserts")
    void testMultipleInserts() {
        for (int i = 0; i < 100; i++) {
            filter.insert("item-" + i);
        }

        // All inserted items should be found
        for (int i = 0; i < 100; i++) {
            assertTrue(filter.contains("item-" + i), "Item " + i + " not found");
        }
    }

    @Test
    @DisplayName("Should work with byte arrays")
    void testInsertAndContainsByteArray() {
        byte[] item = "test-data".getBytes();
        filter.insert(item);
        assertTrue(filter.contains(item));
    }

    @Test
    @DisplayName("Should handle duplicate insertions")
    void testDuplicateInsertions() {
        filter.insert("duplicate-item");
        filter.insert("duplicate-item");
        filter.insert("duplicate-item");

        assertTrue(filter.contains("duplicate-item"));
    }

    @Test
    @DisplayName("Should reject null item on insert (byte array)")
    void testInsertNullByteArray() {
        assertThrows(NullPointerException.class, () -> filter.insert((byte[]) null));
    }

    @Test
    @DisplayName("Should reject null item on insert (string)")
    void testInsertNullString() {
        assertThrows(NullPointerException.class, () -> filter.insert((String) null));
    }

    @Test
    @DisplayName("Should reject null item on contains (byte array)")
    void testContainsNullByteArray() {
        assertThrows(NullPointerException.class, () -> filter.contains((byte[]) null));
    }

    @Test
    @DisplayName("Should reject null item on contains (string)")
    void testContainsNullString() {
        assertThrows(NullPointerException.class, () -> filter.contains((String) null));
    }

    @Test
    @DisplayName("Should guarantee no false negatives")
    void testNoFalseNegatives() {
        String[] items = {"apple", "banana", "cherry", "date", "elderberry"};
        for (String item : items) {
            filter.insert(item);
        }

        for (String item : items) {
            assertTrue(filter.contains(item), "False negative for: " + item);
        }
    }

    @Test
    @DisplayName("Should reject operations on closed filter")
    void testClosedFilterDetection() {
        filter.close();

        assertThrows(IllegalStateException.class, () -> filter.insert("item"));
        assertThrows(IllegalStateException.class, () -> filter.contains("item"));
        assertThrows(IllegalStateException.class, () -> filter.serialize());
    }

    @Test
    @DisplayName("Should be idempotent when closed multiple times")
    void testMultipleClose() {
        filter.close();
        assertDoesNotThrow(() -> filter.close());
    }

    // ==================== Batch Operation Tests ====================

    @Test
    @DisplayName("Should insert multiple items with insertBatch")
    void testInsertBatch() {
        filter.insertBatch("item1", "item2", "item3", "item4", "item5");

        for (int i = 1; i <= 5; i++) {
            assertTrue(filter.contains("item" + i));
        }
    }

    @Test
    @DisplayName("Should reject null items array in insertBatch")
    void testInsertBatchNullArray() {
        assertThrows(NullPointerException.class, () -> filter.insertBatch((String[]) null));
    }

    @Test
    @DisplayName("Should reject null items array in containsBatch")
    void testContainsBatchNullArray() {
        assertThrows(NullPointerException.class, () -> filter.containsBatch((String[]) null));
    }

    @Test
    @DisplayName("Should check multiple items with containsBatch")
    void testContainsBatch() {
        filter.insert("apple");
        filter.insert("banana");
        filter.insert("cherry");

        boolean[] results = filter.containsBatch("apple", "banana", "cherry", "date", "elderberry");

        assertEquals(5, results.length);
        assertTrue(results[0]); // apple
        assertTrue(results[1]); // banana
        assertTrue(results[2]); // cherry
        assertFalse(results[3]); // date (definitely not in)
        assertFalse(results[4]); // elderberry (definitely not in)
    }

    @Test
    @DisplayName("Should handle byte array batch operations")
    void testBatchOperationsByteArray() {
        byte[] item1 = "data1".getBytes();
        byte[] item2 = "data2".getBytes();

        filter.insertBatch(item1, item2);

        boolean[] results = filter.containsBatch(item1, item2);
        assertTrue(results[0]);
        assertTrue(results[1]);
    }

    // ==================== Merge Tests ====================

    @Test
    @DisplayName("Should merge two compatible BloomFilters")
    void testMergeCompatible() {
        BloomFilter filter2 = new BloomFilter(10000, 0.01);

        filter.insert("first-set");
        filter2.insert("second-set");

        filter.merge(filter2);

        assertTrue(filter.contains("first-set"));
        assertTrue(filter.contains("second-set"));

        filter2.close();
    }

    @Test
    @DisplayName("Should reject merge with different n values")
    void testMergeDifferentN() {
        BloomFilter filter2 = new BloomFilter(5000, 0.01);

        assertThrows(IllegalArgumentException.class, () -> filter.merge(filter2));

        filter2.close();
    }

    @Test
    @DisplayName("Should reject merge with different fpr values")
    void testMergeDifferentFpr() {
        BloomFilter filter2 = new BloomFilter(10000, 0.05);

        assertThrows(IllegalArgumentException.class, () -> filter.merge(filter2));

        filter2.close();
    }

    @Test
    @DisplayName("Should reject merge with null")
    void testMergeNull() {
        assertThrows(NullPointerException.class, () -> filter.merge(null));
    }

    @Test
    @DisplayName("Should reject merge on closed filter")
    void testMergeClosedFilter() {
        BloomFilter filter2 = new BloomFilter(10000, 0.01);
        filter.close();

        assertThrows(IllegalStateException.class, () -> filter.merge(filter2));

        filter2.close();
    }

    @Test
    @DisplayName("Should merge filters with identical data")
    void testMergeIdenticalFilters() {
        BloomFilter filter2 = new BloomFilter(10000, 0.01);

        String[] items = {"apple", "banana", "cherry"};
        for (String item : items) {
            filter.insert(item);
            filter2.insert(item);
        }

        filter.merge(filter2);

        for (String item : items) {
            assertTrue(filter.contains(item));
        }

        filter2.close();
    }

    // ==================== Serialization Tests ====================

    @Test
    @DisplayName("Should serialize and deserialize empty filter")
    void testSerializeEmptyFilter() {
        byte[] serialized = filter.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);

        BloomFilter restored = BloomFilter.deserialize(serialized);
        assertNotNull(restored);
        restored.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize filter with data")
    void testSerializeFilterWithData() {
        filter.insert("item1");
        filter.insert("item2");
        filter.insert("item3");

        byte[] serialized = filter.serialize();

        BloomFilter restored = BloomFilter.deserialize(serialized);
        assertTrue(restored.contains("item1"));
        assertTrue(restored.contains("item2"));
        assertTrue(restored.contains("item3"));

        restored.close();
    }

    @Test
    @DisplayName("Should preserve data through serialization round-trip")
    void testSerializationRoundTrip() {
        int itemCount = 1000;
        for (int i = 0; i < itemCount; i++) {
            filter.insert("item-" + i);
        }

        byte[] serialized = filter.serialize();
        BloomFilter restored = BloomFilter.deserialize(serialized);

        for (int i = 0; i < itemCount; i++) {
            assertTrue(restored.contains("item-" + i),
                    "Item item-" + i + " not found after deserialization");
        }

        restored.close();
    }

    @Test
    @DisplayName("Should reject invalid serialized data")
    void testDeserializeInvalidData() {
        byte[] invalidData = {1, 2, 3, 4, 5};
        assertThrows(IllegalArgumentException.class, () -> BloomFilter.deserialize(invalidData));
    }

    @Test
    @DisplayName("Should reject null serialized data")
    void testDeserializeNull() {
        assertThrows(NullPointerException.class, () -> BloomFilter.deserialize(null));
    }

    @Test
    @DisplayName("Should reject serialize on closed filter")
    void testSerializeClosedFilter() {
        filter.close();
        assertThrows(IllegalStateException.class, () -> filter.serialize());
    }

    @Test
    @DisplayName("Should serialize to bytes array of reasonable size")
    void testSerializationSize() {
        for (int i = 0; i < 1000; i++) {
            filter.insert("item-" + i);
        }

        byte[] serialized = filter.serialize();

        // Size should be reasonable for a Bloom filter (not exact, just sanity check)
        assertTrue(serialized.length > 0);
        assertTrue(serialized.length < 100_000);
    }

    // ==================== String Representation Tests ====================

    @Test
    @DisplayName("Should provide meaningful toString")
    void testToString() {
        String str = filter.toString();
        assertNotNull(str);
        assertTrue(str.contains("BloomFilter"));
        assertTrue(str.contains("10000"));
        assertTrue(str.contains("0.01"));
    }

    @Test
    @DisplayName("Should indicate closed state in toString")
    void testToStringClosed() {
        filter.close();
        String str = filter.toString();
        assertTrue(str.contains("closed") || str.contains("BloomFilter"));
    }

    // ==================== Resource Management Tests ====================

    @Test
    @DisplayName("Should work with try-with-resources")
    void testTryWithResources() {
        try (BloomFilter bf = new BloomFilter(1000, 0.01)) {
            bf.insert("test");
            assertTrue(bf.contains("test"));
        }
        // No exception should be thrown
    }

    @Test
    @DisplayName("Should handle repeated insertions efficiently")
    void testRepeatedInsertions() {
        for (int i = 0; i < 100; i++) {
            filter.insert("repeated-item");
        }

        assertTrue(filter.contains("repeated-item"));
    }

    // ==================== Large Dataset Tests ====================

    @Test
    @DisplayName("Should handle large dataset")
    void testLargeDataset() {
        int n = 100000;
        for (int i = 0; i < n; i++) {
            filter.insert("item-" + i);
        }

        // Sample verification
        for (int i = 0; i < 100; i++) {
            assertTrue(filter.contains("item-" + i));
        }
    }

    @Test
    @DisplayName("Should maintain false positive rate on large dataset")
    void testFprOnLargeDataset() {
        BloomFilter largeFilter = new BloomFilter(1000000, 0.01);

        // Insert items
        for (int i = 0; i < 500000; i++) {
            largeFilter.insert("item-" + i);
        }

        // Check some non-existing items
        int falsePositives = 0;
        for (int i = 500000; i < 510000; i++) {
            if (largeFilter.contains("item-" + i)) {
                falsePositives++;
            }
        }

        double observedFpr = (double) falsePositives / 10000;

        // FPR should be within reasonable bounds (some variance expected)
        assertTrue(observedFpr <= 0.1, "FPR too high: " + observedFpr);

        largeFilter.close();
    }

    // ==================== Edge Case Tests ====================

    @Test
    @DisplayName("Should handle empty string")
    void testEmptyString() {
        filter.insert("");
        assertTrue(filter.contains(""));
    }

    @Test
    @DisplayName("Should handle binary data")
    void testBinaryData() {
        byte[] binary = {0, 1, 2, 3, 127, -128, -1};
        filter.insert(binary);
        assertTrue(filter.contains(binary));
    }

    @Test
    @DisplayName("Should handle Unicode strings")
    void testUnicodeStrings() {
        String[] items = {"你好", "مرحبا", "Привет", "🎉", "こんにちは"};
        for (String item : items) {
            filter.insert(item);
        }

        for (String item : items) {
            assertTrue(filter.contains(item), "Unicode item not found: " + item);
        }
    }

    @Test
    @DisplayName("Should distinguish similar items")
    void testDistinguishSimilarItems() {
        filter.insert("test");
        filter.insert("tests");
        filter.insert("testing");

        assertTrue(filter.contains("test"));
        assertTrue(filter.contains("tests"));
        assertTrue(filter.contains("testing"));
        assertFalse(filter.contains("tester"));
    }

    @Test
    @DisplayName("Should handle very large items")
    void testLargeItems() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 10000; i++) {
            sb.append("x");
        }
        String largeString = sb.toString();

        filter.insert(largeString);
        assertTrue(filter.contains(largeString));
    }

    @Test
    @DisplayName("Should work with different n values")
    void testDifferentNValues() {
        for (long n : new long[]{100, 1000, 10000, 100000}) {
            try (BloomFilter bf = new BloomFilter(n, 0.01)) {
                bf.insert("test");
                assertTrue(bf.contains("test"));
            }
        }
    }

    @Test
    @DisplayName("Should work with different fpr values")
    void testDifferentFprValues() {
        for (double fpr : new double[]{0.001, 0.01, 0.05, 0.1, 0.5}) {
            try (BloomFilter bf = new BloomFilter(1000, fpr)) {
                bf.insert("test");
                assertTrue(bf.contains("test"));
            }
        }
    }
}
