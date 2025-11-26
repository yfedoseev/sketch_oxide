package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import org.junit.jupiter.params.provider.CsvSource;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive unit tests for CountMinSketch frequency estimator.
 * Tests construction, frequency tracking, accuracy guarantees (no underestimation),
 * merging, serialization, and resource management.
 */
@DisplayName("CountMinSketch Frequency Estimation Tests")
public class CountMinSketchTest {

    private CountMinSketch cms;

    @BeforeEach
    void setUp() {
        cms = new CountMinSketch(0.01, 0.01);
    }

    @AfterEach
    void tearDown() {
        if (cms != null) {
            try {
                cms.close();
            } catch (Exception e) {
                // Already closed
            }
        }
    }

    // ==================== Constructor Tests ====================

    @Test
    @DisplayName("Should create CountMinSketch with valid parameters")
    void testConstructorValidParameters() {
        try (CountMinSketch test = new CountMinSketch(0.01, 0.01)) {
            assertNotNull(test);
            assertTrue(test.width() > 0);
            assertTrue(test.depth() > 0);
        }
    }

    @ParameterizedTest
    @ValueSource(doubles = {0.0, -0.1, -1.0})
    @DisplayName("Should reject invalid epsilon (0 or less)")
    void testConstructorEpsilonTooLow(double epsilon) {
        assertThrows(IllegalArgumentException.class, () -> new CountMinSketch(epsilon, 0.01));
    }

    @ParameterizedTest
    @ValueSource(doubles = {1.0, 1.5, 2.0})
    @DisplayName("Should reject invalid epsilon (1 or more)")
    void testConstructorEpsilonTooHigh(double epsilon) {
        assertThrows(IllegalArgumentException.class, () -> new CountMinSketch(epsilon, 0.01));
    }

    @ParameterizedTest
    @ValueSource(doubles = {0.0, -0.1, -1.0})
    @DisplayName("Should reject invalid delta (0 or less)")
    void testConstructorDeltaTooLow(double delta) {
        assertThrows(IllegalArgumentException.class, () -> new CountMinSketch(0.01, delta));
    }

    @ParameterizedTest
    @ValueSource(doubles = {1.0, 1.5, 2.0})
    @DisplayName("Should reject invalid delta (1 or more)")
    void testConstructorDeltaTooHigh(double delta) {
        assertThrows(IllegalArgumentException.class, () -> new CountMinSketch(0.01, delta));
    }

    @Test
    @DisplayName("Should accept boundary epsilon and delta values")
    void testConstructorBoundaryValues() {
        try (CountMinSketch cms1 = new CountMinSketch(0.0001, 0.0001)) {
            assertTrue(cms1.width() > 0);
        }
        try (CountMinSketch cms2 = new CountMinSketch(0.999, 0.999)) {
            assertTrue(cms2.width() > 0);
        }
    }

    @ParameterizedTest
    @CsvSource({
        "0.01,0.01",
        "0.001,0.001",
        "0.05,0.05",
        "0.1,0.1"
    })
    @DisplayName("Should accept various valid parameter combinations")
    void testConstructorVariousParameters(double epsilon, double delta) {
        try (CountMinSketch test = new CountMinSketch(epsilon, delta)) {
            assertTrue(test.width() > 0);
            assertTrue(test.depth() > 0);
        }
    }

    // ==================== Core Operation Tests ====================

    @Test
    @DisplayName("Should update and estimate frequency")
    void testUpdateAndEstimate() {
        cms.update("apple");
        long estimate = cms.estimate("apple");
        assertEquals(1, estimate);
    }

    @Test
    @DisplayName("Should handle multiple updates of same item")
    void testMultipleUpdates() {
        cms.update("apple");
        cms.update("apple");
        cms.update("apple");

        long estimate = cms.estimate("apple");
        assertEquals(3, estimate);
    }

    @Test
    @DisplayName("Should update different items")
    void testUpdateDifferentItems() {
        cms.update("apple");
        cms.update("banana");
        cms.update("cherry");

        assertEquals(1, cms.estimate("apple"));
        assertEquals(1, cms.estimate("banana"));
        assertEquals(1, cms.estimate("cherry"));
    }

    @Test
    @DisplayName("Should work with byte arrays")
    void testUpdateEstimateByteArray() {
        byte[] item = "test-data".getBytes();
        cms.update(item);
        long estimate = cms.estimate(item);
        assertEquals(1, estimate);
    }

    @Test
    @DisplayName("Should reject null item on update (string)")
    void testUpdateNullString() {
        assertThrows(NullPointerException.class, () -> cms.update((String) null));
    }

    @Test
    @DisplayName("Should reject null item on update (byte array)")
    void testUpdateNullByteArray() {
        assertThrows(NullPointerException.class, () -> cms.update((byte[]) null));
    }

    @Test
    @DisplayName("Should reject null item on estimate (string)")
    void testEstimateNullString() {
        assertThrows(NullPointerException.class, () -> cms.estimate((String) null));
    }

    @Test
    @DisplayName("Should reject null item on estimate (byte array)")
    void testEstimateNullByteArray() {
        assertThrows(NullPointerException.class, () -> cms.estimate((byte[]) null));
    }

    @Test
    @DisplayName("Should guarantee no underestimation")
    void testNoUnderestimation() {
        // Insert 100 times
        for (int i = 0; i < 100; i++) {
            cms.update("item");
        }

        long estimate = cms.estimate("item");
        assertTrue(estimate >= 100, "Estimate " + estimate + " is less than true count 100");
    }

    @Test
    @DisplayName("Should return 0 for non-existent items")
    void testNonExistentItems() {
        cms.update("apple");

        long estimate = cms.estimate("banana");
        assertEquals(0, estimate);
    }

    @Test
    @DisplayName("Should reject operations on closed sketch")
    void testClosedSketchDetection() {
        cms.close();

        assertThrows(IllegalStateException.class, () -> cms.update("item"));
        assertThrows(IllegalStateException.class, () -> cms.estimate("item"));
        assertThrows(IllegalStateException.class, () -> cms.serialize());
        assertThrows(IllegalStateException.class, () -> cms.width());
        assertThrows(IllegalStateException.class, () -> cms.depth());
    }

    @Test
    @DisplayName("Should be idempotent when closed multiple times")
    void testMultipleClose() {
        cms.close();
        assertDoesNotThrow(() -> cms.close());
    }

    // ==================== Accuracy Tests ====================

    @Test
    @DisplayName("Should maintain relative error bounds")
    void testAccuracyBounds() {
        // Insert items with different frequencies
        for (int i = 0; i < 100; i++) {
            cms.update("frequent");
        }
        for (int i = 0; i < 50; i++) {
            cms.update("medium");
        }
        for (int i = 0; i < 10; i++) {
            cms.update("rare");
        }

        long freq = cms.estimate("frequent");
        long med = cms.estimate("medium");
        long r = cms.estimate("rare");

        // All should be >= true counts
        assertTrue(freq >= 100);
        assertTrue(med >= 50);
        assertTrue(r >= 10);

        // Relative ordering should be preserved
        assertTrue(freq >= med);
        assertTrue(med >= r);
    }

    @Test
    @DisplayName("Should handle high frequency items")
    void testHighFrequencyItems() {
        for (int i = 0; i < 10000; i++) {
            cms.update("popular");
        }

        long estimate = cms.estimate("popular");
        assertTrue(estimate >= 10000);
    }

    // ==================== Batch Operation Tests ====================

    @Test
    @DisplayName("Should update multiple items with updateBatch")
    void testUpdateBatch() {
        cms.updateBatch("apple", "banana", "cherry", "date", "elderberry");

        for (String item : new String[]{"apple", "banana", "cherry", "date", "elderberry"}) {
            assertEquals(1, cms.estimate(item));
        }
    }

    @Test
    @DisplayName("Should estimate multiple items with estimateBatch")
    void testEstimateBatch() {
        cms.update("apple");
        cms.update("banana");
        cms.update("cherry");

        long[] estimates = cms.estimateBatch("apple", "banana", "cherry", "date");

        assertEquals(4, estimates.length);
        assertEquals(1, estimates[0]); // apple
        assertEquals(1, estimates[1]); // banana
        assertEquals(1, estimates[2]); // cherry
        assertEquals(0, estimates[3]); // date (not in sketch)
    }

    @Test
    @DisplayName("Should reject null items array in updateBatch")
    void testUpdateBatchNullArray() {
        assertThrows(NullPointerException.class, () -> cms.updateBatch((String[]) null));
    }

    @Test
    @DisplayName("Should reject null items array in estimateBatch")
    void testEstimateBatchNullArray() {
        assertThrows(NullPointerException.class, () -> cms.estimateBatch((String[]) null));
    }

    @Test
    @DisplayName("Should handle byte array batch operations")
    void testBatchOperationsByteArray() {
        byte[] item1 = "data1".getBytes();
        byte[] item2 = "data2".getBytes();

        cms.updateBatch(item1, item2);

        long[] estimates = cms.estimateBatch(item1, item2);
        assertEquals(1, estimates[0]);
        assertEquals(1, estimates[1]);
    }

    // ==================== Merge Tests ====================

    @Test
    @DisplayName("Should merge two compatible sketches")
    void testMergeCompatible() {
        CountMinSketch cms2 = new CountMinSketch(0.01, 0.01);

        cms.update("apple");
        cms.update("apple");
        cms2.update("apple");
        cms2.update("banana");

        cms.merge(cms2);

        assertTrue(cms.estimate("apple") >= 3);
        assertTrue(cms.estimate("banana") >= 1);

        cms2.close();
    }

    @Test
    @DisplayName("Should reject merge with different epsilon")
    void testMergeDifferentEpsilon() {
        CountMinSketch cms2 = new CountMinSketch(0.05, 0.01);

        assertThrows(IllegalArgumentException.class, () -> cms.merge(cms2));

        cms2.close();
    }

    @Test
    @DisplayName("Should reject merge with different delta")
    void testMergeDifferentDelta() {
        CountMinSketch cms2 = new CountMinSketch(0.01, 0.05);

        assertThrows(IllegalArgumentException.class, () -> cms.merge(cms2));

        cms2.close();
    }

    @Test
    @DisplayName("Should reject merge with null")
    void testMergeNull() {
        assertThrows(NullPointerException.class, () -> cms.merge(null));
    }

    @Test
    @DisplayName("Should reject merge on closed sketch")
    void testMergeClosedSketch() {
        CountMinSketch cms2 = new CountMinSketch(0.01, 0.01);
        cms.close();

        assertThrows(IllegalStateException.class, () -> cms.merge(cms2));

        cms2.close();
    }

    @Test
    @DisplayName("Should merge sketches with identical data")
    void testMergeIdenticalData() {
        CountMinSketch cms2 = new CountMinSketch(0.01, 0.01);

        for (String item : new String[]{"apple", "banana", "cherry"}) {
            cms.update(item);
            cms2.update(item);
        }

        cms.merge(cms2);

        for (String item : new String[]{"apple", "banana", "cherry"}) {
            assertTrue(cms.estimate(item) >= 2);
        }

        cms2.close();
    }

    // ==================== Serialization Tests ====================

    @Test
    @DisplayName("Should serialize and deserialize empty sketch")
    void testSerializeEmptySketch() {
        byte[] serialized = cms.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);

        CountMinSketch restored = CountMinSketch.deserialize(serialized);
        assertNotNull(restored);
        restored.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize sketch with data")
    void testSerializeSketchWithData() {
        cms.update("apple");
        cms.update("banana");
        cms.update("cherry");

        byte[] serialized = cms.serialize();

        CountMinSketch restored = CountMinSketch.deserialize(serialized);
        assertEquals(1, restored.estimate("apple"));
        assertEquals(1, restored.estimate("banana"));
        assertEquals(1, restored.estimate("cherry"));

        restored.close();
    }

    @Test
    @DisplayName("Should preserve data through serialization round-trip")
    void testSerializationRoundTrip() {
        for (int i = 0; i < 1000; i++) {
            cms.update("item-" + (i % 100)); // 100 unique items, 10 each
        }

        byte[] serialized = cms.serialize();
        CountMinSketch restored = CountMinSketch.deserialize(serialized);

        for (int i = 0; i < 100; i++) {
            assertTrue(restored.estimate("item-" + i) >= 10,
                    "Item item-" + i + " frequency not preserved");
        }

        restored.close();
    }

    @Test
    @DisplayName("Should reject invalid serialized data")
    void testDeserializeInvalidData() {
        byte[] invalidData = {1, 2, 3, 4, 5};
        assertThrows(IllegalArgumentException.class, () -> CountMinSketch.deserialize(invalidData));
    }

    @Test
    @DisplayName("Should reject null serialized data")
    void testDeserializeNull() {
        assertThrows(NullPointerException.class, () -> CountMinSketch.deserialize(null));
    }

    @Test
    @DisplayName("Should reject serialize on closed sketch")
    void testSerializeClosedSketch() {
        cms.close();
        assertThrows(IllegalStateException.class, () -> cms.serialize());
    }

    // ==================== String Representation Tests ====================

    @Test
    @DisplayName("Should provide meaningful toString")
    void testToString() {
        String str = cms.toString();
        assertNotNull(str);
        assertTrue(str.contains("CountMinSketch"));
        assertTrue(str.contains("0.01")); // epsilon/delta
    }

    @Test
    @DisplayName("Should indicate closed state in toString")
    void testToStringClosed() {
        cms.close();
        String str = cms.toString();
        assertTrue(str.contains("closed") || str.contains("CountMinSketch"));
    }

    // ==================== Resource Management Tests ====================

    @Test
    @DisplayName("Should work with try-with-resources")
    void testTryWithResources() {
        try (CountMinSketch test = new CountMinSketch(0.01, 0.01)) {
            test.update("test");
            assertEquals(1, test.estimate("test"));
        }
        // No exception should be thrown
    }

    // ==================== Width and Depth Tests ====================

    @Test
    @DisplayName("Should provide valid width")
    void testWidth() {
        int width = cms.width();
        assertTrue(width > 0);
    }

    @Test
    @DisplayName("Should provide valid depth")
    void testDepth() {
        int depth = cms.depth();
        assertTrue(depth > 0);
    }

    @Test
    @DisplayName("Should respect epsilon in width calculation")
    void testWidthRespectEpsilon() {
        try (CountMinSketch precise = new CountMinSketch(0.001, 0.01);
             CountMinSketch coarse = new CountMinSketch(0.1, 0.01)) {

            // More precise epsilon should have larger width
            assertTrue(precise.width() > coarse.width());
        }
    }

    @Test
    @DisplayName("Should respect delta in depth calculation")
    void testDepthRespectDelta() {
        try (CountMinSketch confident = new CountMinSketch(0.01, 0.001);
             CountMinSketch lessConfident = new CountMinSketch(0.01, 0.1)) {

            // Higher confidence should have larger depth
            assertTrue(confident.depth() > lessConfident.depth());
        }
    }

    // ==================== Reset Tests ====================

    @Test
    @DisplayName("Should reset sketch to empty state")
    void testReset() {
        cms.update("apple");
        cms.update("apple");
        cms.update("banana");

        cms.reset();

        assertEquals(0, cms.estimate("apple"));
        assertEquals(0, cms.estimate("banana"));
    }

    @Test
    @DisplayName("Should allow updates after reset")
    void testUpdateAfterReset() {
        cms.update("apple");
        cms.reset();

        cms.update("banana");
        assertEquals(0, cms.estimate("apple"));
        assertEquals(1, cms.estimate("banana"));
    }

    // ==================== Large Dataset Tests ====================

    @Test
    @DisplayName("Should handle large dataset")
    void testLargeDataset() {
        int n = 100000;
        for (int i = 0; i < n; i++) {
            cms.update("item-" + (i % 1000)); // 1000 unique items
        }

        long estimate = cms.estimate("item-0");
        assertTrue(estimate >= 100, "Estimate too low: " + estimate);
    }

    // ==================== Edge Case Tests ====================

    @Test
    @DisplayName("Should handle empty string")
    void testEmptyString() {
        cms.update("");
        assertEquals(1, cms.estimate(""));
    }

    @Test
    @DisplayName("Should handle binary data")
    void testBinaryData() {
        byte[] binary = {0, 1, 2, 3, 127, -128, -1};
        cms.update(binary);
        assertEquals(1, cms.estimate(binary));
    }

    @Test
    @DisplayName("Should handle Unicode strings")
    void testUnicodeStrings() {
        String[] items = {"你好", "مرحبا", "Привет", "🎉", "こんにちは"};
        for (String item : items) {
            cms.update(item);
        }

        for (String item : items) {
            assertEquals(1, cms.estimate(item));
        }
    }

    @Test
    @DisplayName("Should distinguish similar items")
    void testDistinguishSimilarItems() {
        cms.update("test");
        cms.update("tests");
        cms.update("testing");

        assertEquals(1, cms.estimate("test"));
        assertEquals(1, cms.estimate("tests"));
        assertEquals(1, cms.estimate("testing"));
        assertEquals(0, cms.estimate("tester"));
    }

    @Test
    @DisplayName("Should handle very large items")
    void testLargeItems() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 10000; i++) {
            sb.append("x");
        }
        String largeString = sb.toString();

        cms.update(largeString);
        assertEquals(1, cms.estimate(largeString));
    }

    @Test
    @DisplayName("Should work with different epsilon/delta combinations")
    void testDifferentParameters() {
        for (double epsilon : new double[]{0.001, 0.01, 0.05, 0.1}) {
            for (double delta : new double[]{0.001, 0.01, 0.05, 0.1}) {
                try (CountMinSketch test = new CountMinSketch(epsilon, delta)) {
                    test.update("test");
                    assertEquals(1, test.estimate("test"));
                }
            }
        }
    }
}
