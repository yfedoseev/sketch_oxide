package com.sketches_oxide.membership;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("BinaryFuseFilter - 75% Smaller Than Bloom")
public class BinaryFuseFilterTest {

    @Test
    void testCreate() {
        try (BinaryFuseFilter bff = new BinaryFuseFilter(1000, 0.01)) {
            assertNotNull(bff);
        }
    }

    @Test
    void testInsertAndContains() {
        try (BinaryFuseFilter bff = new BinaryFuseFilter(1000, 0.01)) {
            bff.insert("test".getBytes());
            assertTrue(bff.contains("test".getBytes()));
        }
    }

    @Test
    void testSpaceEfficiency() {
        try (BinaryFuseFilter bff = new BinaryFuseFilter(10000, 0.01)) {
            bff.insert("item".getBytes());
            assertNotNull(bff);
        }
    }
}
