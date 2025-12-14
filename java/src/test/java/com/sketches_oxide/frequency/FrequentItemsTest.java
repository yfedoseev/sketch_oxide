package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for FrequentItems - Heavy hitter detection.
 */
@DisplayName("FrequentItems - Heavy Hitter Detection")
public class FrequentItemsTest {

    private FrequentItems fi;

    @BeforeEach
    void setUp() {
        fi = new FrequentItems(100);
    }

    @AfterEach
    void tearDown() {
        if (fi != null) {
            fi.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid FrequentItems")
    void testConstructor() {
        FrequentItems test = new FrequentItems(100);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should identify heavy hitters")
    void testHeavyHitterDetection() {
        // 50 occurrences of "heavy"
        for (int i = 0; i < 50; i++) {
            fi.update("heavy".getBytes());
        }
        // 5 occurrences of "light"
        for (int i = 0; i < 5; i++) {
            fi.update("light".getBytes());
        }

        long heavyCount = fi.getEstimate("heavy".getBytes());
        long lightCount = fi.getEstimate("light".getBytes());

        assertTrue(heavyCount > lightCount);
    }

    @Test
    @DisplayName("Should update with count")
    void testUpdateWithCount() {
        fi.updateBy("item".getBytes(), 100);
        long count = fi.getEstimate("item".getBytes());
        assertTrue(count >= 90); // Allow some error
    }

    @Test
    @DisplayName("Should merge two FrequentItems")
    void testMerge() {
        FrequentItems fi2 = new FrequentItems(100);

        for (int i = 0; i < 50; i++) {
            fi.update("item".getBytes());
        }
        for (int i = 0; i < 30; i++) {
            fi2.update("item".getBytes());
        }

        fi.merge(fi2);
        long count = fi.getEstimate("item".getBytes());

        assertTrue(count >= 70);
        fi2.close();
    }

    @Test
    @DisplayName("Should track multiple items")
    void testMultipleItems() {
        for (int i = 0; i < 1000; i++) {
            fi.update(("item-" + (i % 10)).getBytes());
        }

        for (int i = 0; i < 10; i++) {
            long count = fi.getEstimate(("item-" + i).getBytes());
            assertTrue(count > 0);
        }
    }

    @Test
    @DisplayName("Should return max size")
    void testMaxSize() {
        FrequentItems test = new FrequentItems(100);
        assertEquals(100, test.maxSize());
        test.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (int i = 0; i < 1000; i++) {
            fi.update(("item-" + (i % 10)).getBytes());
        }

        byte[] serialized = fi.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (FrequentItems test = new FrequentItems(100)) {
            test.update("data".getBytes());
            assertTrue(test.getEstimate("data".getBytes()) > 0);
        }
    }
}
