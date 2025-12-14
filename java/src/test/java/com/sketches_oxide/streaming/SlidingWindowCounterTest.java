package com.sketches_oxide.streaming;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("SlidingWindowCounter - Frequency Over Windows")
public class SlidingWindowCounterTest {

    private SlidingWindowCounter swc;

    @BeforeEach
    void setUp() {
        swc = new SlidingWindowCounter(1000, 0.01);
    }

    @AfterEach
    void tearDown() {
        if (swc != null) {
            swc.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(swc);
    }

    @Test
    void testIncrement() {
        swc.increment();
        long count = swc.count();
        assertEquals(1, count);
    }

    @Test
    void testMultipleIncrements() {
        for (int i = 0; i < 100; i++) {
            swc.increment();
        }
        long count = swc.count();
        assertEquals(100, count);
    }

    @Test
    void testTryWithResources() {
        try (SlidingWindowCounter test = new SlidingWindowCounter(1000, 0.01)) {
            test.increment();
            assertEquals(1, test.count());
        }
    }
}
