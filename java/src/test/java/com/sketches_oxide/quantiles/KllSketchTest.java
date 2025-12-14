package com.sketches_oxide.quantiles;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for KLL Sketch - Compact quantile estimation.
 */
@DisplayName("KLL Sketch - Compact Quantiles")
public class KllSketchTest {

    private KllSketch kll;

    @BeforeEach
    void setUp() {
        kll = new KllSketch(100);
    }

    @AfterEach
    void tearDown() {
        if (kll != null) {
            kll.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid KLL Sketch")
    void testConstructor() {
        KllSketch test = new KllSketch(100);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate quantiles correctly")
    void testQuantileEstimate() {
        for (double i = 0; i < 1000; i++) {
            kll.update(i);
        }

        double median = kll.getQuantile(0.5);
        assertTrue(median > 400 && median < 600);
    }

    @Test
    @DisplayName("Should merge two KLL Sketches")
    void testMerge() {
        KllSketch kll2 = new KllSketch(100);

        for (double i = 0; i < 500; i++) {
            kll.update(i);
            kll2.update(i + 500);
        }

        kll.merge(kll2);
        double median = kll.getQuantile(0.5);

        assertTrue(median > 400 && median < 600);
        kll2.close();
    }

    @Test
    @DisplayName("Should handle multiple quantiles")
    void testMultipleQuantiles() {
        for (double i = 0; i < 1000; i++) {
            kll.update(i);
        }

        double[] quantiles = {0.25, 0.5, 0.75, 0.99};
        double prev = -1;
        for (double q : quantiles) {
            double val = kll.getQuantile(q);
            assertTrue(val > prev, "Quantiles should be monotonic");
            prev = val;
        }
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (double i = 0; i < 1000; i++) {
            kll.update(i);
        }

        byte[] serialized = kll.serialize();
        assertNotNull(serialized);
    }

    @Test
    @DisplayName("Should be memory efficient")
    void testMemoryEfficiency() {
        for (double i = 0; i < 100000; i++) {
            kll.update(i);
        }
        byte[] serialized = kll.serialize();
        assertTrue(serialized.length < 10000);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (KllSketch test = new KllSketch(100)) {
            test.update(1.0);
            assertTrue(test.getQuantile(0.5) > 0);
        }
    }
}
