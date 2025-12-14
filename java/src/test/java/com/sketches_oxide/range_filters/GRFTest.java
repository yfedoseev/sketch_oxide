package com.sketches_oxide.range_filters;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for GRF (Gorilla Range Filter) - Range query support.
 */
@DisplayName("GRF - Range Filter")
public class GRFTest {

    @Test
    @DisplayName("Should create GRF from keys")
    void testCreateGRF() {
        long[] keys = {1L, 2L, 3L, 4L, 5L};
        try (GRF grf = GRF.build(keys, 8)) {
            assertNotNull(grf);
        }
    }

    @Test
    @DisplayName("Should contain inserted keys")
    void testContainsKey() {
        long[] keys = {10L, 20L, 30L, 40L, 50L};
        try (GRF grf = GRF.build(keys, 8)) {
            assertTrue(grf.mayContain(10L));
            assertTrue(grf.mayContain(30L));
            assertTrue(grf.mayContain(50L));
        }
    }

    @Test
    @DisplayName("Should reject keys outside range")
    void testRejectOutOfRange() {
        long[] keys = {10L, 20L, 30L};
        try (GRF grf = GRF.build(keys, 8)) {
            assertFalse(grf.mayContain(5L));
            assertFalse(grf.mayContain(100L));
        }
    }

    @Test
    @DisplayName("Should support range queries")
    void testRangeQueries() {
        long[] keys = {10L, 20L, 30L, 40L, 50L};
        try (GRF grf = GRF.build(keys, 8)) {
            assertTrue(grf.mayContainRange(10L, 50L));
            assertTrue(grf.mayContainRange(15L, 45L));
        }
    }

    @Test
    @DisplayName("Should reject empty range")
    void testEmptyRange() {
        long[] keys = {10L, 20L, 30L};
        try (GRF grf = GRF.build(keys, 8)) {
            assertFalse(grf.mayContainRange(100L, 200L));
        }
    }

    @Test
    @DisplayName("Should report key count")
    void testKeyCount() {
        long[] keys = {1L, 2L, 3L, 4L, 5L};
        try (GRF grf = GRF.build(keys, 8)) {
            assertEquals(5, grf.keyCount());
        }
    }

    @Test
    @DisplayName("Should reject invalid bits per key")
    void testInvalidBitsPerKey() {
        long[] keys = {1L, 2L, 3L};
        assertThrows(IllegalArgumentException.class, () -> GRF.build(keys, 1));
        assertThrows(IllegalArgumentException.class, () -> GRF.build(keys, 17));
    }

    @Test
    @DisplayName("Should reject empty keys")
    void testEmptyKeys() {
        long[] keys = {};
        assertThrows(IllegalArgumentException.class, () -> GRF.build(keys, 8));
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        long[] keys = {1L, 2L, 3L};
        try (GRF grf = GRF.build(keys, 8)) {
            assertTrue(grf.mayContain(1L));
        }
    }
}
