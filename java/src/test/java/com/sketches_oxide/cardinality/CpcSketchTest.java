package com.sketches_oxide.cardinality;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for CPC Sketch (Compressed Probabilistic Counting).
 *
 * From "CPC Sketch: Compact Probabilistic Counting" (2017).
 * Provides accurate cardinality estimation with compression.
 */
@DisplayName("CPC Sketch - Compressed Probabilistic Counting")
public class CpcSketchTest {

    private CpcSketch cpc;

    @BeforeEach
    void setUp() {
        cpc = new CpcSketch(14);
    }

    @AfterEach
    void tearDown() {
        if (cpc != null) {
            cpc.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid CPC Sketch")
    void testConstructor() {
        CpcSketch test = new CpcSketch(14);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate cardinality for small set")
    void testSmallSetEstimate() {
        for (int i = 0; i < 100; i++) {
            cpc.update(("item-" + i).getBytes());
        }

        double estimate = cpc.estimate();
        assertTrue(estimate > 50);
        assertTrue(estimate < 200);
    }

    @Test
    @DisplayName("Should handle large cardinality estimates")
    void testLargeCardinality() {
        int n = 100000;
        for (int i = 0; i < n; i++) {
            cpc.update(("element-" + i).getBytes());
        }

        double estimate = cpc.estimate();
        double error = Math.abs(estimate - n) / n;
        assertTrue(error < 0.05, "Error: " + error);
    }

    @Test
    @DisplayName("Should merge two CPC Sketches")
    void testMerge() {
        CpcSketch cpc2 = new CpcSketch(14);

        for (int i = 0; i < 5000; i++) {
            cpc.update(("first-" + i).getBytes());
            cpc2.update(("second-" + i).getBytes());
        }

        double estimate1 = cpc.estimate();
        cpc.merge(cpc2);
        double estimateMerged = cpc.estimate();

        assertTrue(estimateMerged > estimate1);
        cpc2.close();
    }

    @Test
    @DisplayName("Should serialize correctly")
    void testSerialization() {
        for (int i = 0; i < 1000; i++) {
            cpc.update(("item-" + i).getBytes());
        }

        byte[] serialized = cpc.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Should return zero for empty sketch")
    void testEmptySketch() {
        double estimate = cpc.estimate();
        assertTrue(estimate == 0 || estimate < 1);
    }

    @Test
    @DisplayName("Compression should reduce size")
    void testCompression() {
        for (int i = 0; i < 10000; i++) {
            cpc.update(("item-" + i).getBytes());
        }
        byte[] serialized = cpc.serialize();
        // CPC uses compression, so size should be reasonable
        assertTrue(serialized.length < 16384);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (CpcSketch test = new CpcSketch(14)) {
            test.update("data".getBytes());
            assertTrue(test.estimate() > 0);
        }
    }
}
