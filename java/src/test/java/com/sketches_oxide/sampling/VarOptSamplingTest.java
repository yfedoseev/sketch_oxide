package com.sketches_oxide.sampling;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for VarOptSampling - Weighted sampling with variance optimization.
 */
@DisplayName("VarOptSampling - Weighted Sampling")
public class VarOptSamplingTest {

    private VarOptSampling vos;

    @BeforeEach
    void setUp() {
        vos = new VarOptSampling(100);
    }

    @AfterEach
    void tearDown() {
        if (vos != null) {
            vos.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid VarOptSampling")
    void testConstructor() {
        VarOptSampling test = new VarOptSampling(100);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should track stream count")
    void testStreamCount() {
        for (int i = 0; i < 1000; i++) {
            vos.update(("item-" + i).getBytes(), 1.0);
        }
        assertEquals(1000, vos.count());
    }

    @Test
    @DisplayName("Should track total weight")
    void testTotalWeight() {
        for (int i = 0; i < 100; i++) {
            vos.update(("item-" + i).getBytes(), 2.0);
        }

        double totalWeight = vos.totalWeight();
        assertEquals(200.0, totalWeight, 0.1);
    }

    @Test
    @DisplayName("Should maintain capacity")
    void testCapacity() {
        assertEquals(100, vos.capacity());
    }

    @Test
    @DisplayName("Should handle weighted items")
    void testWeightedItems() {
        vos.update("heavy".getBytes(), 100.0);
        vos.update("light".getBytes(), 1.0);

        double totalWeight = vos.totalWeight();
        assertTrue(totalWeight > 100);
    }

    @Test
    @DisplayName("Should return sample")
    void testSample() {
        for (int i = 0; i < 50; i++) {
            vos.update(("item-" + i).getBytes(), 1.0);
        }

        byte[][] sample = vos.sample();
        assertTrue(sample.length > 0);
    }

    @Test
    @DisplayName("Should not exceed capacity")
    void testCapacityNotExceeded() {
        for (int i = 0; i < 1000; i++) {
            vos.update(("item-" + i).getBytes(), 1.0);
        }

        byte[][] sample = vos.sample();
        assertTrue(sample.length <= 100);
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (int i = 0; i < 100; i++) {
            vos.update(("item-" + i).getBytes(), 1.0);
        }

        byte[] serialized = vos.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (VarOptSampling test = new VarOptSampling(100)) {
            test.update("data".getBytes(), 1.0);
            assertEquals(1, test.count());
        }
    }
}
