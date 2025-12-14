package com.sketches_oxide.universal;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("UnivMon - Universal Monitoring")
public class UnivMonTest {

    private UnivMon um;

    @BeforeEach
    void setUp() {
        um = new UnivMon(0.01, 0.001);
    }

    @AfterEach
    void tearDown() {
        if (um != null) {
            um.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(um);
    }

    @Test
    void testUpdate() {
        um.update("item".getBytes(), 1L);
        assertNotNull(um);
    }

    @Test
    void testWeightedUpdates() {
        um.update("heavy".getBytes(), 100L);
        um.update("light".getBytes(), 1L);
        assertNotNull(um);
    }

    @Test
    void testMerge() {
        UnivMon um2 = new UnivMon(0.01, 0.001);
        um.update("item".getBytes(), 1L);
        um2.update("item".getBytes(), 1L);
        um.merge(um2);
        um2.close();
    }

    @Test
    void testTryWithResources() {
        try (UnivMon test = new UnivMon(0.01, 0.001)) {
            test.update("data".getBytes(), 1L);
            assertNotNull(test);
        }
    }
}
