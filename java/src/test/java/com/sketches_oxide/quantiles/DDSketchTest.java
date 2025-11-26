package com.sketches_oxide.quantiles;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;
import org.junit.jupiter.params.provider.CsvSource;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive unit tests for DDSketch quantile estimator.
 * Tests construction, value insertion, quantile queries with relative error guarantees,
 * merging, serialization, and resource management.
 */
@DisplayName("DDSketch Quantile Estimation Tests")
public class DDSketchTest {

    private DDSketch sketch;

    @BeforeEach
    void setUp() {
        sketch = new DDSketch(0.01); // 1% relative accuracy
    }

    @AfterEach
    void tearDown() {
        if (sketch != null) {
            try {
                sketch.close();
            } catch (Exception e) {
                // Already closed
            }
        }
    }

    // ==================== Constructor Tests ====================

    @Test
    @DisplayName("Should create DDSketch with valid relative accuracy")
    void testConstructorValidParameters() {
        try (DDSketch test = new DDSketch(0.01)) {
            assertNotNull(test);
            assertEquals(0.01, test.getRelativeAccuracy(), 0.0001);
        }
    }

    @ParameterizedTest
    @ValueSource(doubles = {0.0, -0.1, -1.0})
    @DisplayName("Should reject invalid relative accuracy (0 or less)")
    void testConstructorAlphaTooLow(double alpha) {
        assertThrows(IllegalArgumentException.class, () -> new DDSketch(alpha));
    }

    @ParameterizedTest
    @ValueSource(doubles = {1.0, 1.5, 2.0})
    @DisplayName("Should reject invalid relative accuracy (1 or more)")
    void testConstructorAlphaTooHigh(double alpha) {
        assertThrows(IllegalArgumentException.class, () -> new DDSketch(alpha));
    }

    @Test
    @DisplayName("Should accept boundary relative accuracy values")
    void testConstructorBoundaryValues() {
        try (DDSketch ds1 = new DDSketch(0.0001)) {
            assertEquals(0.0001, ds1.getRelativeAccuracy(), 0.00001);
        }
        try (DDSketch ds2 = new DDSketch(0.999)) {
            assertEquals(0.999, ds2.getRelativeAccuracy(), 0.001);
        }
    }

    @ParameterizedTest
    @ValueSource(doubles = {0.001, 0.01, 0.05, 0.1, 0.5})
    @DisplayName("Should accept various valid relative accuracy values")
    void testConstructorVariousParameters(double alpha) {
        try (DDSketch test = new DDSketch(alpha)) {
            assertEquals(alpha, test.getRelativeAccuracy(), alpha * 0.01);
        }
    }

    // ==================== Core Operation Tests ====================

    @Test
    @DisplayName("Should update with single value")
    void testUpdateSingleValue() {
        sketch.update(100.0);
        double median = sketch.quantile(0.5);
        assertTrue(median > 0, "Median should be positive");
    }

    @Test
    @DisplayName("Should update with multiple values")
    void testUpdateMultipleValues() {
        for (int i = 1; i <= 100; i++) {
            sketch.update(i);
        }

        double median = sketch.quantile(0.5);
        assertTrue(median > 0);
    }

    @Test
    @DisplayName("Should handle duplicate updates")
    void testDuplicateUpdates() {
        for (int i = 0; i < 100; i++) {
            sketch.update(50.0);
        }

        double median = sketch.quantile(0.5);
        assertTrue(Math.abs(median - 50.0) < 50.0 * 0.02); // Within 2%
    }

    @Test
    @DisplayName("Should reject operations on closed sketch")
    void testClosedSketchDetection() {
        sketch.close();

        assertThrows(IllegalStateException.class, () -> sketch.update(100.0));
        assertThrows(IllegalStateException.class, () -> sketch.quantile(0.5));
        assertThrows(IllegalStateException.class, () -> sketch.serialize());
    }

    @Test
    @DisplayName("Should be idempotent when closed multiple times")
    void testMultipleClose() {
        sketch.close();
        assertDoesNotThrow(() -> sketch.close());
    }

    // ==================== Quantile Query Tests ====================

    @Test
    @DisplayName("Should provide quantile at various percentiles")
    void testQuantilesAtDifferentPercentiles() {
        for (int i = 1; i <= 1000; i++) {
            sketch.update(i);
        }

        double p0 = sketch.quantile(0.0);   // min
        double p25 = sketch.quantile(0.25);  // 1st quartile
        double p50 = sketch.quantile(0.5);   // median
        double p75 = sketch.quantile(0.75);  // 3rd quartile
        double p100 = sketch.quantile(1.0);  // max

        assertTrue(p0 <= p25);
        assertTrue(p25 <= p50);
        assertTrue(p50 <= p75);
        assertTrue(p75 <= p100);
    }

    @Test
    @DisplayName("Should reject invalid quantiles")
    void testInvalidQuantiles() {
        sketch.update(100.0);

        assertThrows(IllegalArgumentException.class, () -> sketch.quantile(-0.1));
        assertThrows(IllegalArgumentException.class, () -> sketch.quantile(1.1));
    }

    @Test
    @DisplayName("Should accept boundary quantile values")
    void testBoundaryQuantiles() {
        for (int i = 1; i <= 100; i++) {
            sketch.update(i);
        }

        double min = sketch.quantile(0.0);
        double max = sketch.quantile(1.0);

        assertTrue(min >= 1);
        assertTrue(max <= 100);
    }

    @Test
    @DisplayName("Should maintain relative accuracy")
    void testRelativeAccuracy() {
        // Add values 1 to 1000
        for (int i = 1; i <= 1000; i++) {
            sketch.update(i);
        }

        double median = sketch.quantile(0.5);
        double trueMedian = 500.5;

        double relativeError = Math.abs(median - trueMedian) / trueMedian;
        assertTrue(relativeError <= 0.02, // 1% accuracy allows ~2% actual error
                "Relative error " + relativeError + " exceeds bounds");
    }

    @Test
    @DisplayName("Should provide monotonic quantiles")
    void testMonotonicQuantiles() {
        for (int i = 1; i <= 1000; i++) {
            sketch.update(i);
        }

        double q1 = sketch.quantile(0.1);
        double q5 = sketch.quantile(0.5);
        double q9 = sketch.quantile(0.9);

        assertTrue(q1 <= q5);
        assertTrue(q5 <= q9);
    }

    // ==================== Batch Quantile Tests ====================

    @Test
    @DisplayName("Should query multiple quantiles efficiently")
    void testBatchQuantiles() {
        for (int i = 1; i <= 1000; i++) {
            sketch.update(i);
        }

        double[] results = sketch.quantiles(new double[]{0.25, 0.5, 0.75});

        assertEquals(3, results.length);
        assertTrue(results[0] <= results[1]);
        assertTrue(results[1] <= results[2]);
    }

    @Test
    @DisplayName("Should match single and batch quantile results")
    void testBatchVsSingleQuantiles() {
        for (int i = 1; i <= 1000; i++) {
            sketch.update(i);
        }

        double[] single = new double[5];
        for (int i = 0; i < 5; i++) {
            single[i] = sketch.quantile(0.2 * (i + 1));
        }

        double[] batch = sketch.quantiles(new double[]{0.2, 0.4, 0.6, 0.8, 1.0});

        for (int i = 0; i < 5; i++) {
            assertEquals(single[i], batch[i], 0.01);
        }
    }

    @Test
    @DisplayName("Should reject null quantiles array")
    void testQuantilesNullArray() {
        sketch.update(100.0);
        assertThrows(NullPointerException.class, () -> sketch.quantiles(null));
    }

    @Test
    @DisplayName("Should reject invalid quantile in batch")
    void testBatchInvalidQuantile() {
        sketch.update(100.0);
        assertThrows(IllegalArgumentException.class,
                () -> sketch.quantiles(new double[]{0.5, 1.5}));
    }

    @Test
    @DisplayName("Should handle empty quantiles array")
    void testEmptyQuantilesArray() {
        sketch.update(100.0);
        double[] results = sketch.quantiles(new double[]{});
        assertEquals(0, results.length);
    }

    // ==================== Merge Tests ====================

    @Test
    @DisplayName("Should merge two compatible sketches")
    void testMergeCompatible() {
        DDSketch sketch2 = new DDSketch(0.01);

        // Add values to first sketch
        for (int i = 1; i <= 500; i++) {
            sketch.update(i);
        }

        // Add different values to second sketch
        for (int i = 501; i <= 1000; i++) {
            sketch2.update(i);
        }

        sketch.merge(sketch2);

        double median = sketch.quantile(0.5);
        double trueMedian = 500.5;
        double relativeError = Math.abs(median - trueMedian) / trueMedian;

        assertTrue(relativeError <= 0.1, "Merged sketch accuracy: " + relativeError);

        sketch2.close();
    }

    @Test
    @DisplayName("Should reject merge with different relative accuracy")
    void testMergeDifferentAccuracy() {
        DDSketch sketch2 = new DDSketch(0.05);

        assertThrows(IllegalArgumentException.class, () -> sketch.merge(sketch2));

        sketch2.close();
    }

    @Test
    @DisplayName("Should reject merge with null")
    void testMergeNull() {
        assertThrows(NullPointerException.class, () -> sketch.merge(null));
    }

    @Test
    @DisplayName("Should reject merge on closed sketch")
    void testMergeClosedSketch() {
        DDSketch sketch2 = new DDSketch(0.01);
        sketch.close();

        assertThrows(IllegalStateException.class, () -> sketch.merge(sketch2));

        sketch2.close();
    }

    @Test
    @DisplayName("Should merge sketches with overlapping data")
    void testMergeOverlappingData() {
        DDSketch sketch2 = new DDSketch(0.01);

        for (int i = 1; i <= 500; i++) {
            sketch.update(i);
            sketch2.update(i);
        }

        sketch.merge(sketch2);

        double median = sketch.quantile(0.5);
        assertTrue(median > 0);

        sketch2.close();
    }

    // ==================== Serialization Tests ====================

    @Test
    @DisplayName("Should serialize and deserialize empty sketch")
    void testSerializeEmptySketch() {
        byte[] serialized = sketch.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);

        DDSketch restored = DDSketch.deserialize(serialized);
        assertNotNull(restored);
        restored.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize sketch with data")
    void testSerializeSketchWithData() {
        for (int i = 1; i <= 1000; i++) {
            sketch.update(i);
        }

        byte[] serialized = sketch.serialize();

        DDSketch restored = DDSketch.deserialize(serialized);
        double original = sketch.quantile(0.5);
        double restoredValue = restored.quantile(0.5);

        assertEquals(original, restoredValue, 1.0);

        restored.close();
    }

    @Test
    @DisplayName("Should preserve data through serialization round-trip")
    void testSerializationRoundTrip() {
        // Add different distribution
        for (int i = 0; i < 1000; i++) {
            sketch.update(Math.random() * 1000);
        }

        byte[] serialized = sketch.serialize();
        DDSketch restored = DDSketch.deserialize(serialized);

        double original25 = sketch.quantile(0.25);
        double original50 = sketch.quantile(0.5);
        double original75 = sketch.quantile(0.75);

        double restored25 = restored.quantile(0.25);
        double restored50 = restored.quantile(0.5);
        double restored75 = restored.quantile(0.75);

        // Allow some variance due to quantile estimation
        assertEquals(original25, restored25, Math.max(original25 * 0.1, 1.0));
        assertEquals(original50, restored50, Math.max(original50 * 0.1, 1.0));
        assertEquals(original75, restored75, Math.max(original75 * 0.1, 1.0));

        restored.close();
    }

    @Test
    @DisplayName("Should reject invalid serialized data")
    void testDeserializeInvalidData() {
        byte[] invalidData = {1, 2, 3, 4, 5};
        assertThrows(IllegalArgumentException.class, () -> DDSketch.deserialize(invalidData));
    }

    @Test
    @DisplayName("Should reject null serialized data")
    void testDeserializeNull() {
        assertThrows(NullPointerException.class, () -> DDSketch.deserialize(null));
    }

    @Test
    @DisplayName("Should reject serialize on closed sketch")
    void testSerializeClosedSketch() {
        sketch.close();
        assertThrows(IllegalStateException.class, () -> sketch.serialize());
    }

    // ==================== String Representation Tests ====================

    @Test
    @DisplayName("Should provide meaningful toString")
    void testToString() {
        String str = sketch.toString();
        assertNotNull(str);
        assertTrue(str.contains("DDSketch"));
        assertTrue(str.contains("0.01"));
    }

    @Test
    @DisplayName("Should indicate closed state in toString")
    void testToStringClosed() {
        sketch.close();
        String str = sketch.toString();
        assertTrue(str.contains("closed") || str.contains("DDSketch"));
    }

    // ==================== Resource Management Tests ====================

    @Test
    @DisplayName("Should work with try-with-resources")
    void testTryWithResources() {
        try (DDSketch test = new DDSketch(0.01)) {
            for (int i = 1; i <= 100; i++) {
                test.update(i);
            }
            double median = test.quantile(0.5);
            assertTrue(median > 0);
        }
        // No exception should be thrown
    }

    // ==================== Distribution Tests ====================

    @Test
    @DisplayName("Should handle uniform distribution")
    void testUniformDistribution() {
        for (int i = 1; i <= 1000; i++) {
            sketch.update(i);
        }

        double q25 = sketch.quantile(0.25);
        double q50 = sketch.quantile(0.5);
        double q75 = sketch.quantile(0.75);

        // For uniform distribution, check rough spacing
        assertTrue(q25 > 0);
        assertTrue(q50 > q25);
        assertTrue(q75 > q50);
    }

    @Test
    @DisplayName("Should handle exponential distribution")
    void testExponentialDistribution() {
        // Simulate exponential distribution
        for (int i = 0; i < 1000; i++) {
            double value = -Math.log(1 - Math.random()) * 100;
            sketch.update(value);
        }

        double median = sketch.quantile(0.5);
        double p95 = sketch.quantile(0.95);

        assertTrue(median > 0);
        assertTrue(p95 > median);
    }

    @Test
    @DisplayName("Should handle bimodal distribution")
    void testBimodalDistribution() {
        // Two modes: around 100 and 900
        for (int i = 0; i < 500; i++) {
            sketch.update(100 + Math.random() * 100); // Mode 1: ~100-200
            sketch.update(800 + Math.random() * 200); // Mode 2: ~800-1000
        }

        double median = sketch.quantile(0.5);
        assertTrue(median > 0);
    }

    @Test
    @DisplayName("Should handle skewed distribution")
    void testSkewedDistribution() {
        // Right-skewed: most values small, few very large
        for (int i = 0; i < 900; i++) {
            sketch.update(Math.random() * 10);
        }
        for (int i = 0; i < 100; i++) {
            sketch.update(10 + Math.random() * 1000);
        }

        double median = sketch.quantile(0.5);
        double p95 = sketch.quantile(0.95);

        assertTrue(p95 > median);
    }

    // ==================== Large Dataset Tests ====================

    @Test
    @DisplayName("Should handle large dataset")
    void testLargeDataset() {
        for (int i = 0; i < 1000000; i++) {
            sketch.update(Math.random() * 1000);
        }

        double median = sketch.quantile(0.5);
        double p99 = sketch.quantile(0.99);

        assertTrue(median > 0);
        assertTrue(p99 >= median);
    }

    // ==================== Edge Case Tests ====================

    @Test
    @DisplayName("Should handle very small values")
    void testSmallValues() {
        for (int i = 0; i < 100; i++) {
            sketch.update(0.0001 * (i + 1));
        }

        double median = sketch.quantile(0.5);
        assertTrue(median > 0);
    }

    @Test
    @DisplayName("Should handle very large values")
    void testLargeValues() {
        for (int i = 0; i < 100; i++) {
            sketch.update(1e10 * (i + 1));
        }

        double median = sketch.quantile(0.5);
        assertTrue(median > 0);
    }

    @Test
    @DisplayName("Should handle mixed small and large values")
    void testMixedValueMagnitudes() {
        for (int i = 0; i < 50; i++) {
            sketch.update(0.001);
            sketch.update(1e8);
        }

        double median = sketch.quantile(0.5);
        assertTrue(median > 0);
    }

    @Test
    @DisplayName("Should work with different relative accuracy values")
    void testDifferentAccuracyValues() {
        for (double alpha : new double[]{0.001, 0.01, 0.05, 0.1}) {
            try (DDSketch test = new DDSketch(alpha)) {
                for (int i = 1; i <= 1000; i++) {
                    test.update(i);
                }
                double median = test.quantile(0.5);
                assertTrue(median > 0);
            }
        }
    }

    @Test
    @DisplayName("Should handle single value")
    void testSingleValue() {
        sketch.update(42.0);

        double q0 = sketch.quantile(0.0);
        double q50 = sketch.quantile(0.5);
        double q100 = sketch.quantile(1.0);

        // All quantiles should be near the single value
        assertTrue(Math.abs(q50 - 42.0) < 42.0 * 0.05);
    }

    @Test
    @DisplayName("Should handle constant values")
    void testConstantValues() {
        for (int i = 0; i < 1000; i++) {
            sketch.update(100.0);
        }

        double q25 = sketch.quantile(0.25);
        double q50 = sketch.quantile(0.5);
        double q75 = sketch.quantile(0.75);

        // All quantiles should be close to 100
        assertTrue(Math.abs(q25 - 100.0) < 100.0 * 0.05);
        assertTrue(Math.abs(q50 - 100.0) < 100.0 * 0.05);
        assertTrue(Math.abs(q75 - 100.0) < 100.0 * 0.05);
    }
}
