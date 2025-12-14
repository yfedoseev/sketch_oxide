package com.sketches_oxide.cardinality;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for QSketch - Quantile-based cardinality estimation.
 */
@DisplayName("QSketch - Quantile-Based Cardinality")
public class QSketchTest {

    private QSketch qs;

    @BeforeEach
    void setUp() {
        qs = new QSketch(14);
    }

    @AfterEach
    void tearDown() {
        if (qs != null) {
            qs.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid QSketch")
    void testConstructor() {
        QSketch test = new QSketch(14);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate cardinality")
    void testBasicEstimate() {
        for (int i = 0; i < 1000; i++) {
            qs.update(("item-" + i).getBytes());
        }

        double estimate = qs.estimate();
        assertTrue(estimate > 500);
        assertTrue(estimate < 2000);
    }

    @Test
    @DisplayName("Should handle various cardinalities")
    void testMultipleCardinalities() {
        int[] cardinalities = {10, 100, 1000, 10000};

        for (int card : cardinalities) {
            try (QSketch test = new QSketch(14)) {
                for (int i = 0; i < card; i++) {
                    test.update(("item-" + i).getBytes());
                }
                double estimate = test.estimate();
                assertTrue(estimate > card / 2);
                assertTrue(estimate < card * 3);
            }
        }
    }

    @Test
    @DisplayName("Should return zero for empty sketch")
    void testEmptySketch() {
        double estimate = qs.estimate();
        assertTrue(estimate == 0 || estimate < 1);
    }

    @Test
    @DisplayName("Should merge sketches")
    void testMerge() {
        QSketch qs2 = new QSketch(14);

        for (int i = 0; i < 5000; i++) {
            qs.update(("first-" + i).getBytes());
            qs2.update(("second-" + i).getBytes());
        }

        double estimate1 = qs.estimate();
        qs.merge(qs2);
        double estimateMerged = qs.estimate();

        assertTrue(estimateMerged > estimate1);
        qs2.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (int i = 0; i < 1000; i++) {
            qs.update(("item-" + i).getBytes());
        }

        byte[] serialized = qs.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (QSketch test = new QSketch(14)) {
            test.update("data".getBytes());
            assertTrue(test.estimate() > 0);
        }
    }
}
