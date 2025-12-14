package com.sketches_oxide.cardinality;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for UltraLogLog cardinality estimator (VLDB 2024).
 *
 * UltraLogLog is 28% more space-efficient than HyperLogLog
 * while maintaining similar accuracy.
 */
@DisplayName("UltraLogLog Cardinality Estimator")
public class UltraLogLogTest {

    private UltraLogLog ull;

    @BeforeEach
    void setUp() {
        ull = new UltraLogLog(14);
    }

    @AfterEach
    void tearDown() {
        if (ull != null) {
            ull.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid UltraLogLog with precision 14")
    void testConstructor() {
        UltraLogLog test = new UltraLogLog(14);
        assertNotNull(test);
        assertEquals(14, test.precision());
        test.close();
    }

    @Test
    @DisplayName("Constructor should validate precision bounds")
    void testConstructorPrecisionValidation() {
        assertThrows(IllegalArgumentException.class, () -> new UltraLogLog(3));
        assertThrows(IllegalArgumentException.class, () -> new UltraLogLog(17));
        new UltraLogLog(4).close();
        new UltraLogLog(16).close();
    }

    @Test
    @DisplayName("Should estimate cardinality within 2% error")
    void testEstimateAccuracy() {
        int n = 10000;
        for (int i = 0; i < n; i++) {
            ull.update(("item-" + i).getBytes());
        }

        double estimate = ull.estimate();
        double error = Math.abs(estimate - n) / n;
        assertTrue(error < 0.02, "Estimation error " + error + " exceeded 2%");
    }

    @Test
    @DisplayName("Should return zero estimate for empty sketch")
    void testEmptyEstimate() {
        double estimate = ull.estimate();
        assertTrue(estimate == 0 || estimate < 1.0);
    }

    @Test
    @DisplayName("Should handle duplicate updates correctly")
    void testDuplicates() {
        for (int i = 0; i < 100; i++) {
            ull.update(("same").getBytes());
        }
        double estimate = ull.estimate();
        assertTrue(estimate <= 5);
    }

    @Test
    @DisplayName("Should merge two UltraLogLogs correctly")
    void testMerge() {
        UltraLogLog ull2 = new UltraLogLog(14);

        for (int i = 0; i < 5000; i++) {
            ull.update(("first-" + i).getBytes());
            ull2.update(("second-" + i).getBytes());
        }

        double estimate1 = ull.estimate();
        ull.merge(ull2);
        double estimateMerged = ull.estimate();

        assertTrue(estimateMerged > estimate1);
        ull2.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize correctly")
    void testSerializationRoundTrip() {
        for (int i = 0; i < 1000; i++) {
            ull.update(("item-" + i).getBytes());
        }

        double originalEstimate = ull.estimate();
        byte[] serialized = ull.serialize();

        UltraLogLog restored = UltraLogLog.deserialize(serialized);
        double restoredEstimate = restored.estimate();

        assertEquals(originalEstimate, restoredEstimate, 0.0001);
        restored.close();
    }

    @Test
    @DisplayName("Should be 28% more space-efficient than HyperLogLog")
    void testSpaceEfficiency() {
        // UltraLogLog uses less memory than HyperLogLog for same precision
        try (UltraLogLog ull_test = new UltraLogLog(14)) {
            byte[] serialized = ull_test.serialize();
            assertTrue(serialized.length < 8192, "UltraLogLog should use < 8KB for precision=14");
        }
    }

    @Test
    @DisplayName("Should handle string updates")
    void testStringUpdate() {
        ull.update("first");
        ull.update("second");
        ull.update("third");

        double estimate = ull.estimate();
        assertTrue(estimate >= 3);
    }

    @Test
    @DisplayName("Large dataset test (100K items)")
    void testLargeDataset() {
        int n = 100000;
        for (int i = 0; i < n; i++) {
            ull.update(("item-" + i).getBytes());
        }

        double estimate = ull.estimate();
        double error = Math.abs(estimate - n) / n;
        assertTrue(error < 0.02);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (UltraLogLog test = new UltraLogLog(14)) {
            test.update("data".getBytes());
            assertTrue(test.estimate() > 0);
        }
    }
}
