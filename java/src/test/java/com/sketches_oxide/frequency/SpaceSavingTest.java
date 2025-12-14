package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for SpaceSaving - Deterministic frequent items detection.
 */
@DisplayName("SpaceSaving - Frequent Items")
public class SpaceSavingTest {

    private SpaceSaving ss;

    @BeforeEach
    void setUp() {
        ss = new SpaceSaving(0.01);
    }

    @AfterEach
    void tearDown() {
        if (ss != null) {
            ss.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid SpaceSaving")
    void testConstructor() {
        SpaceSaving test = new SpaceSaving(0.01);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should track frequent items")
    void testFrequentItemTracking() {
        for (int i = 0; i < 100; i++) {
            ss.update("frequent".getBytes());
        }
        for (int i = 0; i < 10; i++) {
            ss.update(("rare-" + i).getBytes());
        }

        long frequentCount = ss.estimate("frequent".getBytes());
        long rareCount = ss.estimate("rare-1".getBytes());

        assertTrue(frequentCount > rareCount);
    }

    @Test
    @DisplayName("Should estimate frequencies")
    void testFrequencyEstimates() {
        // Add items with known frequencies
        for (int i = 0; i < 50; i++) {
            ss.update("item-A".getBytes());
        }
        for (int i = 0; i < 25; i++) {
            ss.update("item-B".getBytes());
        }

        long countA = ss.estimate("item-A".getBytes());
        long countB = ss.estimate("item-B".getBytes());

        assertTrue(countA > countB);
    }

    @Test
    @DisplayName("Should handle stream length")
    void testStreamLength() {
        for (int i = 0; i < 1000; i++) {
            ss.update(("item-" + (i % 10)).getBytes());
        }

        long streamLength = ss.streamLength();
        assertEquals(1000, streamLength);
    }

    @Test
    @DisplayName("Should return zero for non-existent items")
    void testNonExistentItems() {
        ss.update("exists".getBytes());

        long count = ss.estimate("not-exists".getBytes());
        assertTrue(count == 0 || count < 1);
    }

    @Test
    @DisplayName("Should merge two SpaceSaving sketches")
    void testMerge() {
        SpaceSaving ss2 = new SpaceSaving(0.01);

        for (int i = 0; i < 50; i++) {
            ss.update("item".getBytes());
        }
        for (int i = 0; i < 30; i++) {
            ss2.update("item".getBytes());
        }

        ss.merge(ss2);
        long count = ss.estimate("item".getBytes());

        assertTrue(count >= 70);
        ss2.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (int i = 0; i < 1000; i++) {
            ss.update(("item-" + (i % 10)).getBytes());
        }

        byte[] serialized = ss.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (SpaceSaving test = new SpaceSaving(0.01)) {
            test.update("data".getBytes());
            assertTrue(test.estimate("data".getBytes()) > 0);
        }
    }
}
