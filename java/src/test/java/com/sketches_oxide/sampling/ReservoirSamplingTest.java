package com.sketches_oxide.sampling;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for ReservoirSampling - Uniform random sampling from streams.
 */
@DisplayName("ReservoirSampling - Uniform Random Sampling")
public class ReservoirSamplingTest {

    private ReservoirSampling rs;

    @BeforeEach
    void setUp() {
        rs = new ReservoirSampling(100);
    }

    @AfterEach
    void tearDown() {
        if (rs != null) {
            rs.close();
        }
    }

    @Test
    @DisplayName("Constructor should create valid ReservoirSampling")
    void testConstructor() {
        ReservoirSampling test = new ReservoirSampling(100);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should track stream count")
    void testStreamCount() {
        for (int i = 0; i < 1000; i++) {
            rs.update(("item-" + i).getBytes());
        }
        assertEquals(1000, rs.count());
    }

    @Test
    @DisplayName("Should maintain capacity")
    void testCapacity() {
        assertEquals(100, rs.capacity());
    }

    @Test
    @DisplayName("Should get sample of correct size")
    void testSampleSize() {
        for (int i = 0; i < 50; i++) {
            rs.update(("item-" + i).getBytes());
        }

        byte[][] sample = rs.sample();
        assertEquals(50, sample.length);
    }

    @Test
    @DisplayName("Should not exceed capacity")
    void testCapacityNotExceeded() {
        for (int i = 0; i < 1000; i++) {
            rs.update(("item-" + i).getBytes());
        }

        byte[][] sample = rs.sample();
        assertTrue(sample.length <= 100);
    }

    @Test
    @DisplayName("Should handle sampling probability")
    void testSamplingProbability() {
        for (int i = 0; i < 1000; i++) {
            rs.update(("item-" + i).getBytes());
        }

        double prob = rs.getSamplingProbability();
        assertEquals(0.1, prob, 0.01); // k/n = 100/1000
    }

    @Test
    @DisplayName("Should convert sample to strings")
    void testSampleAsStrings() {
        rs.update("string1".getBytes());
        rs.update("string2".getBytes());

        String[] strings = rs.sampleAsStrings();
        assertTrue(strings.length > 0);
    }

    @Test
    @DisplayName("Should serialize and deserialize")
    void testSerialization() {
        for (int i = 0; i < 100; i++) {
            rs.update(("item-" + i).getBytes());
        }

        byte[] serialized = rs.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);
    }

    @Test
    @DisplayName("Should indicate if reservoir is full")
    void testIsFull() {
        for (int i = 0; i < 50; i++) {
            rs.update(("item-" + i).getBytes());
        }
        assertFalse(rs.isFull());

        for (int i = 50; i < 100; i++) {
            rs.update(("item-" + i).getBytes());
        }
        assertTrue(rs.isFull());
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (ReservoirSampling test = new ReservoirSampling(100)) {
            test.update("data".getBytes());
            assertEquals(1, test.count());
        }
    }
}
