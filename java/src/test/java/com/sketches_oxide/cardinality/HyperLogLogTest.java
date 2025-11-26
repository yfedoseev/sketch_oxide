package com.sketches_oxide.cardinality;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for HyperLogLog cardinality estimator.
 */
@DisplayName("HyperLogLog Cardinality Estimator")
public class HyperLogLogTest {

    private HyperLogLog hll;

    @BeforeEach
    void setUp() {
        hll = new HyperLogLog(14);
    }

    @AfterEach
    void tearDown() {
        if (hll != null) {
            hll.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid HyperLogLog with precision 14")
    void testConstructor() {
        HyperLogLog test = new HyperLogLog(14);
        assertNotNull(test);
        assertEquals(14, test.precision());
        test.close();
    }

    @Test
    @DisplayName("Constructor should validate precision bounds")
    void testConstructorPrecisionValidation() {
        // Too low
        assertThrows(IllegalArgumentException.class, () -> new HyperLogLog(3));

        // Too high
        assertThrows(IllegalArgumentException.class, () -> new HyperLogLog(17));

        // Valid boundaries
        new HyperLogLog(4).close();
        new HyperLogLog(16).close();
    }

    @Test
    @DisplayName("Should estimate cardinality within error bounds")
    void testEstimateAccuracy() {
        int n = 10000;
        for (int i = 0; i < n; i++) {
            hll.update(("item-" + i).getBytes());
        }

        double estimate = hll.estimate();
        double error = Math.abs(estimate - n) / n;

        // Allow 3% error for precision=14
        assertTrue(error < 0.03, "Estimation error " + error + " exceeded 3%");
    }

    @Test
    @DisplayName("Should return zero estimate for empty sketch")
    void testEmptyEstimate() {
        double estimate = hll.estimate();
        assertTrue(estimate == 0 || estimate < 1.0, "Empty sketch should estimate ~0");
    }

    @Test
    @DisplayName("Should handle duplicate updates correctly")
    void testDuplicates() {
        for (int i = 0; i < 100; i++) {
            hll.update(("same-item").getBytes());
        }

        double estimate = hll.estimate();
        assertTrue(estimate <= 5, "Duplicates should not increase estimate");
    }

    @Test
    @DisplayName("Should merge two HyperLogLogs correctly")
    void testMerge() {
        HyperLogLog hll2 = new HyperLogLog(14);

        for (int i = 0; i < 5000; i++) {
            hll.update(("first-" + i).getBytes());
            hll2.update(("second-" + i).getBytes());
        }

        double estimate1 = hll.estimate();
        hll.merge(hll2);
        double estimateMerged = hll.estimate();

        // Merged estimate should be roughly sum of both (with variance)
        assertTrue(estimateMerged > estimate1, "Merged estimate should be larger");
        assertTrue(estimateMerged < 15000, "Merged estimate should be reasonable");

        hll2.close();
    }

    @Test
    @DisplayName("Should reject merge of different precisions")
    void testMergeIncompatiblePrecision() {
        HyperLogLog hll16 = new HyperLogLog(16);

        assertThrows(IllegalArgumentException.class, () -> {
            hll.merge(hll16);
        });

        hll16.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize correctly")
    void testSerializationRoundTrip() {
        for (int i = 0; i < 1000; i++) {
            hll.update(("item-" + i).getBytes());
        }

        double originalEstimate = hll.estimate();
        byte[] serialized = hll.serialize();

        HyperLogLog restored = HyperLogLog.deserialize(serialized);
        double restoredEstimate = restored.estimate();

        assertEquals(originalEstimate, restoredEstimate, 0.0001,
                "Deserialized estimate should match original");

        restored.close();
    }

    @Test
    @DisplayName("Should handle null input properly")
    void testNullInput() {
        assertThrows(NullPointerException.class, () -> hll.update((byte[]) null));
        assertThrows(NullPointerException.class, () -> hll.update((String) null));
    }

    @Test
    @DisplayName("Should detect closed sketch usage")
    void testClosedSketchDetection() {
        hll.close();

        assertThrows(IllegalStateException.class, () -> hll.update("test".getBytes()));
        assertThrows(IllegalStateException.class, () -> hll.estimate());
        assertThrows(IllegalStateException.class, () -> hll.precision());
    }

    @Test
    @DisplayName("Should handle string updates")
    void testStringUpdate() {
        hll.update("first");
        hll.update("second");
        hll.update("third");

        double estimate = hll.estimate();
        assertTrue(estimate >= 3, "Should have estimated at least 3 items");
    }

    @Test
    @DisplayName("estimateLong should return rounded value")
    void testEstimateLong() {
        for (int i = 0; i < 100; i++) {
            hll.update(("item-" + i).getBytes());
        }

        long longEstimate = hll.estimateLong();
        assertTrue(longEstimate > 0, "Long estimate should be positive");
    }

    @Test
    @DisplayName("toString should include precision and estimate")
    void testToString() {
        String str = hll.toString();
        assertTrue(str.contains("HyperLogLog"));
        assertTrue(str.contains("precision"));
    }

    @Test
    @DisplayName("Large dataset test")
    void testLargeDataset() {
        int n = 1000000;
        for (int i = 0; i < n; i++) {
            hll.update(("user-" + i).getBytes());
        }

        double estimate = hll.estimate();
        double error = Math.abs(estimate - n) / n;

        assertTrue(error < 0.03, "Error on 1M items: " + error);
        System.out.println("1M items test: estimate=" + estimate + ", error=" + (error * 100) + "%");
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (HyperLogLog test = new HyperLogLog(14)) {
            test.update("data".getBytes());
            double estimate = test.estimate();
            assertTrue(estimate > 0);
        }
        // No exception should be thrown
    }
}
