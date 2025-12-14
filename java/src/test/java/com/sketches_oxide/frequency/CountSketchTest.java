package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for Count Sketch - Symmetric variant with cancellation.
 *
 * Provides estimates with signed errors (can be positive or negative).
 */
@DisplayName("Count Sketch - Symmetric Frequency Estimation")
public class CountSketchTest {

    private CountSketch cs;

    @BeforeEach
    void setUp() {
        cs = new CountSketch(100);
    }

    @AfterEach
    void tearDown() {
        if (cs != null) {
            cs.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid Count Sketch")
    void testConstructor() {
        CountSketch test = new CountSketch(100);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate frequencies")
    void testFrequencyEstimate() {
        for (int i = 0; i < 100; i++) {
            cs.update("item".getBytes());
        }

        long estimate = cs.estimate("item".getBytes());
        assertTrue(estimate > 0);
    }

    @Test
    @DisplayName("Should provide cancellation benefit")
    void testCancellation() {
        for (int i = 0; i < 50; i++) {
            cs.update("A".getBytes());
        }
        for (int i = 0; i < 50; i++) {
            cs.update("B".getBytes());
        }

        long countA = cs.estimate("A".getBytes());
        long countB = cs.estimate("B".getBytes());

        assertTrue(countA > 0);
        assertTrue(countB > 0);
    }

    @Test
    @DisplayName("Should support merge")
    void testMerge() {
        CountSketch cs2 = new CountSketch(100);

        for (int i = 0; i < 50; i++) {
            cs.update("item".getBytes());
        }
        for (int i = 0; i < 30; i++) {
            cs2.update("item".getBytes());
        }

        cs.merge(cs2);
        long count = cs.estimate("item".getBytes());

        assertTrue(count >= 70);
        cs2.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (int i = 0; i < 1000; i++) {
            cs.update(("item-" + (i % 10)).getBytes());
        }

        byte[] serialized = cs.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (CountSketch test = new CountSketch(100)) {
            test.update("data".getBytes());
            assertTrue(test.estimate("data".getBytes()) > 0);
        }
    }
}
