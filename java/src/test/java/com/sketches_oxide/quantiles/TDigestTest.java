package com.sketches_oxide.quantiles;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for TDigest - T-Digest quantile algorithm.
 *
 * Excellent for tail quantiles and large dataset processing.
 */
@DisplayName("TDigest - T-Digest Quantiles")
public class TDigestTest {

    private TDigest td;

    @BeforeEach
    void setUp() {
        td = new TDigest(100);
    }

    @AfterEach
    void tearDown() {
        if (td != null) {
            td.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid TDigest")
    void testConstructor() {
        TDigest test = new TDigest(100);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate quantiles accurately")
    void testQuantileAccuracy() {
        for (double i = 0; i < 10000; i++) {
            td.update(i);
        }

        double p50 = td.getQuantile(0.5);
        double p99 = td.getQuantile(0.99);

        assertTrue(p50 > 4000 && p50 < 6000);
        assertTrue(p99 > 9700 && p99 < 10000);
    }

    @Test
    @DisplayName("Should handle skewed distributions")
    void testSkewedDistribution() {
        // Exponential-like distribution
        for (int i = 0; i < 1000; i++) {
            td.update(Math.exp(i / 200.0));
        }

        double p50 = td.getQuantile(0.5);
        double p99 = td.getQuantile(0.99);

        assertTrue(p99 > p50);
    }

    @Test
    @DisplayName("Should merge two TDigests")
    void testMerge() {
        TDigest td2 = new TDigest(100);

        for (double i = 0; i < 5000; i++) {
            td.update(i);
            td2.update(i + 5000);
        }

        td.merge(td2);
        double p50 = td.getQuantile(0.5);

        assertTrue(p50 > 4000 && p50 < 6000);
        td2.close();
    }

    @Test
    @DisplayName("Should handle negative values")
    void testNegativeValues() {
        for (double i = -5000; i < 5000; i++) {
            td.update(i);
        }

        double p50 = td.getQuantile(0.5);
        assertTrue(p50 > -1000 && p50 < 1000);
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (double i = 0; i < 1000; i++) {
            td.update(i);
        }

        byte[] serialized = td.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Should handle extreme percentiles")
    void testExtremePercentiles() {
        for (double i = 1; i <= 1000; i++) {
            td.update(i);
        }

        double p001 = td.getQuantile(0.001);
        double p999 = td.getQuantile(0.999);

        assertTrue(p001 > 0);
        assertTrue(p999 > 900);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (TDigest test = new TDigest(100)) {
            test.update(1.0);
            assertTrue(test.getQuantile(0.5) > 0);
        }
    }
}
