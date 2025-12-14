package com.sketches_oxide.streaming;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("SlidingHyperLogLog - Cardinality Over Windows")
public class SlidingHyperLogLogTest {

    private SlidingHyperLogLog shll;

    @BeforeEach
    void setUp() {
        shll = new SlidingHyperLogLog(14, 1000);
    }

    @AfterEach
    void tearDown() {
        if (shll != null) {
            shll.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(shll);
    }

    @Test
    void testUpdate() {
        shll.update("item".getBytes());
        double estimate = shll.estimate();
        assertTrue(estimate >= 0);
    }

    @Test
    void testMultipleUpdates() {
        for (int i = 0; i < 100; i++) {
            shll.update(("item-" + i).getBytes());
        }
        double estimate = shll.estimate();
        assertTrue(estimate > 50);
    }

    @Test
    void testTryWithResources() {
        try (SlidingHyperLogLog test = new SlidingHyperLogLog(14, 1000)) {
            test.update("data".getBytes());
            assertTrue(test.estimate() > 0);
        }
    }
}
