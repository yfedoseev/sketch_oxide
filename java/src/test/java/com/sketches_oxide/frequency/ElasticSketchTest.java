package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for Elastic Sketch - Dynamic frequency table.
 */
@DisplayName("Elastic Sketch - Dynamic Frequency")
public class ElasticSketchTest {

    private ElasticSketch es;

    @BeforeEach
    void setUp() {
        es = new ElasticSketch(1000, 0.01);
    }

    @AfterEach
    void tearDown() {
        if (es != null) {
            es.close();
        }
    }

    @Test
    @DisplayName("Should create Elastic Sketch")
    void testConstructor() {
        ElasticSketch test = new ElasticSketch(1000, 0.01);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate frequencies dynamically")
    void testDynamicEstimate() {
        for (int i = 0; i < 100; i++) {
            es.update("item".getBytes());
        }
        long estimate = es.estimate("item".getBytes());
        assertTrue(estimate > 0);
    }

    @Test
    @DisplayName("Should handle high cardinality")
    void testHighCardinality() {
        for (int i = 0; i < 10000; i++) {
            es.update(("item-" + (i % 1000)).getBytes());
        }

        long count = es.estimate("item-0".getBytes());
        assertTrue(count > 0);
    }

    @Test
    @DisplayName("Should merge sketches")
    void testMerge() {
        ElasticSketch es2 = new ElasticSketch(1000, 0.01);

        for (int i = 0; i < 50; i++) {
            es.update("item".getBytes());
            es2.update("item".getBytes());
        }

        es.merge(es2);
        es2.close();
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (ElasticSketch test = new ElasticSketch(1000, 0.01)) {
            test.update("data".getBytes());
            assertTrue(test.estimate("data".getBytes()) >= 0);
        }
    }
}
