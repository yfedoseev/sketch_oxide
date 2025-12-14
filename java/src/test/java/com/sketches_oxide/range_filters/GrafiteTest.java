package com.sketches_oxide.range_filters;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("Grafite - Grafite Range Filter")
public class GrafiteTest {

    @Test
    void testCreate() {
        long[] keys = {1L, 2L, 3L, 4L, 5L};
        try (Grafite g = Grafite.build(keys, 8)) {
            assertNotNull(g);
        }
    }

    @Test
    void testMayContain() {
        long[] keys = {10L, 20L, 30L, 40L, 50L};
        try (Grafite g = Grafite.build(keys, 8)) {
            assertTrue(g.mayContain(10L));
            assertTrue(g.mayContain(30L));
        }
    }

    @Test
    void testMayContainRange() {
        long[] keys = {10L, 20L, 30L, 40L, 50L};
        try (Grafite g = Grafite.build(keys, 8)) {
            assertTrue(g.mayContainRange(10L, 50L));
        }
    }

    @Test
    void testKeyCount() {
        long[] keys = {1L, 2L, 3L, 4L, 5L};
        try (Grafite g = Grafite.build(keys, 8)) {
            assertTrue(g.keyCount() > 0);
        }
    }
}
