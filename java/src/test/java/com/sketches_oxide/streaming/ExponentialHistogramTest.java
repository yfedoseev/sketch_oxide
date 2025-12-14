package com.sketches_oxide.streaming;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("ExponentialHistogram - Time-Decaying")
public class ExponentialHistogramTest {

    private ExponentialHistogram eh;

    @BeforeEach
    void setUp() {
        eh = new ExponentialHistogram(1000, 0.01);
    }

    @AfterEach
    void tearDown() {
        if (eh != null) {
            eh.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(eh);
    }

    @Test
    void testInsert() {
        eh.insert();
        long count = eh.count();
        assertEquals(1, count);
    }

    @Test
    void testMultipleInserts() {
        for (int i = 0; i < 50; i++) {
            eh.insert();
        }
        long count = eh.count();
        assertEquals(50, count);
    }

    @Test
    void testTryWithResources() {
        try (ExponentialHistogram test = new ExponentialHistogram(1000, 0.01)) {
            test.insert();
            assertEquals(1, test.count());
        }
    }
}
