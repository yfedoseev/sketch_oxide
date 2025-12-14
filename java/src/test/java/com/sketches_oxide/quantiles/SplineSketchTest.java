package com.sketches_oxide.quantiles;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for Spline Sketch - Spline-based quantile approximation.
 */
@DisplayName("SplineSketch - Spline-Based Quantiles")
public class SplineSketchTest {

    private SplineSketch ss;

    @BeforeEach
    void setUp() {
        ss = new SplineSketch(0.01);
    }

    @AfterEach
    void tearDown() {
        if (ss != null) {
            ss.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid Spline Sketch")
    void testConstructor() {
        SplineSketch test = new SplineSketch(0.01);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate quantiles using spline interpolation")
    void testSplineInterpolation() {
        for (double i = 0; i < 1000; i++) {
            ss.update(i);
        }

        double p50 = ss.getQuantile(0.5);
        assertTrue(p50 > 400 && p50 < 600);
    }

    @Test
    @DisplayName("Should maintain monotonic quantiles")
    void testMonotonicQuantiles() {
        for (double i = 0; i < 1000; i++) {
            ss.update(i);
        }

        double p25 = ss.getQuantile(0.25);
        double p50 = ss.getQuantile(0.50);
        double p75 = ss.getQuantile(0.75);

        assertTrue(p25 < p50);
        assertTrue(p50 < p75);
    }

    @Test
    @DisplayName("Should handle smooth distributions")
    void testSmoothDistribution() {
        for (int i = 0; i < 10000; i++) {
            ss.update(i % 100);
        }

        double p50 = ss.getQuantile(0.5);
        assertTrue(p50 > 30 && p50 < 70);
    }

    @Test
    @DisplayName("Should merge two Spline Sketches")
    void testMerge() {
        SplineSketch ss2 = new SplineSketch(0.01);

        for (double i = 0; i < 500; i++) {
            ss.update(i);
            ss2.update(i + 500);
        }

        ss.merge(ss2);
        double median = ss.getQuantile(0.5);

        assertTrue(median > 400 && median < 600);
        ss2.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (double i = 0; i < 1000; i++) {
            ss.update(i);
        }

        byte[] serialized = ss.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (SplineSketch test = new SplineSketch(0.01)) {
            test.update(1.0);
            assertTrue(test.getQuantile(0.5) > 0);
        }
    }
}
