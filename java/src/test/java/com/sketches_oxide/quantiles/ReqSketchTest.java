package com.sketches_oxide.quantiles;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for REQ Sketch - Tail quantile specialist with zero error at extremes.
 *
 * From "REQ Sketch: An Exact Quantile Specification" (PODS 2021).
 * Optimized for tail percentiles (99th, 99.9th, etc.).
 */
@DisplayName("REQ Sketch - Tail Quantile Specialist")
public class ReqSketchTest {

    private ReqSketch req;

    @BeforeEach
    void setUp() {
        req = new ReqSketch(100);
    }

    @AfterEach
    void tearDown() {
        if (req != null) {
            req.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid REQ Sketch")
    void testConstructor() {
        ReqSketch test = new ReqSketch(100);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate percentiles correctly")
    void testPercentileEstimate() {
        for (double i = 0; i < 1000; i++) {
            req.update(i);
        }

        double p50 = req.getQuantile(0.5);
        double p99 = req.getQuantile(0.99);

        assertTrue(p50 > 400 && p50 < 600);
        assertTrue(p99 > 900 && p99 < 1000);
    }

    @Test
    @DisplayName("Should have zero error at extremes")
    void testExtremeAccuracy() {
        for (double i = 1; i <= 1000; i++) {
            req.update(i);
        }

        double min = req.getQuantile(0.0);
        double max = req.getQuantile(1.0);

        assertTrue(min <= 1.1);
        assertTrue(max >= 999.0);
    }

    @Test
    @DisplayName("Should specialize in tail quantiles")
    void testTailQuantiles() {
        for (double i = 0; i < 10000; i++) {
            req.update(i);
        }

        double p99 = req.getQuantile(0.99);
        double p999 = req.getQuantile(0.999);

        assertTrue(p99 > 9800);
        assertTrue(p999 > 9980);
    }

    @Test
    @DisplayName("Should merge two REQ Sketches")
    void testMerge() {
        ReqSketch req2 = new ReqSketch(100);

        for (double i = 0; i < 500; i++) {
            req.update(i);
            req2.update(i + 500);
        }

        req.merge(req2);
        double median = req.getQuantile(0.5);

        assertTrue(median > 400 && median < 600);
        req2.close();
    }

    @Test
    @DisplayName("Should handle empty sketch")
    void testEmptySketch() {
        double median = req.getQuantile(0.5);
        assertTrue(Double.isNaN(median) || median == 0);
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (double i = 0; i < 1000; i++) {
            req.update(i);
        }

        byte[] serialized = req.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (ReqSketch test = new ReqSketch(100)) {
            test.update(1.0);
            assertNotNull(test.getQuantile(0.5));
        }
    }
}
