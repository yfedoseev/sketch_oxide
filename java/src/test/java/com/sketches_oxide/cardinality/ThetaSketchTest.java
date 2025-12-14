package com.sketches_oxide.cardinality;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for Theta Sketch - Set operations with full algebra support.
 *
 * From "Theta Sketch Framework for Set Operations" (2015).
 * Supports union, intersection, and difference operations on cardinality estimates.
 */
@DisplayName("Theta Sketch - Set Operations")
public class ThetaSketchTest {

    private ThetaSketch ts;

    @BeforeEach
    void setUp() {
        ts = new ThetaSketch(4096);
    }

    @AfterEach
    void tearDown() {
        if (ts != null) {
            ts.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid Theta Sketch")
    void testConstructor() {
        ThetaSketch test = new ThetaSketch(4096);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate cardinality")
    void testBasicEstimate() {
        for (int i = 0; i < 1000; i++) {
            ts.update(("item-" + i).getBytes());
        }

        double estimate = ts.estimate();
        assertTrue(estimate > 500);
        assertTrue(estimate < 2000);
    }

    @Test
    @DisplayName("Should handle set union")
    void testUnion() {
        ThetaSketch ts2 = new ThetaSketch(4096);

        for (int i = 0; i < 5000; i++) {
            ts.update(("first-" + i).getBytes());
            ts2.update(("second-" + i).getBytes());
        }

        double estimate1 = ts.estimate();
        double estimate2 = ts2.estimate();

        ts.merge(ts2);
        double unionEstimate = ts.estimate();

        // Union should be >= either set
        assertTrue(unionEstimate >= Math.max(estimate1, estimate2) * 0.95);
        ts2.close();
    }

    @Test
    @DisplayName("Should return zero for empty sketch")
    void testEmptySketch() {
        double estimate = ts.estimate();
        assertTrue(estimate == 0 || estimate < 1);
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (int i = 0; i < 1000; i++) {
            ts.update(("item-" + i).getBytes());
        }

        byte[] serialized = ts.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);

        ThetaSketch restored = ThetaSketch.deserialize(serialized);
        assertEquals(ts.estimate(), restored.estimate(), 1.0);
        restored.close();
    }

    @Test
    @DisplayName("Should handle duplicates correctly")
    void testDuplicates() {
        for (int i = 0; i < 100; i++) {
            ts.update(("same").getBytes());
        }

        double estimate = ts.estimate();
        assertTrue(estimate <= 5);
    }

    @Test
    @DisplayName("Should support multiple merges")
    void testMultipleMerges() {
        ThetaSketch ts2 = new ThetaSketch(4096);
        ThetaSketch ts3 = new ThetaSketch(4096);

        for (int i = 0; i < 2000; i++) {
            ts.update(("set1-" + i).getBytes());
            ts2.update(("set2-" + i).getBytes());
            ts3.update(("set3-" + i).getBytes());
        }

        ts.merge(ts2);
        ts.merge(ts3);

        double unionEstimate = ts.estimate();
        assertTrue(unionEstimate > 2000);

        ts2.close();
        ts3.close();
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (ThetaSketch test = new ThetaSketch(4096)) {
            test.update("data".getBytes());
            assertTrue(test.estimate() > 0);
        }
    }

    @Test
    @DisplayName("Large dataset test")
    void testLargeDataset() {
        int n = 100000;
        for (int i = 0; i < n; i++) {
            ts.update(("item-" + i).getBytes());
        }

        double estimate = ts.estimate();
        double error = Math.abs(estimate - n) / n;
        assertTrue(error < 0.1, "Error should be < 10% for 100K items");
    }
}
