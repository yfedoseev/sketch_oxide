package com.sketches_oxide.membership;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("RibbonFilter - Space Efficient Alternative")
public class RibbonFilterTest {

    @Test
    void testCreate() {
        try (RibbonFilter rf = new RibbonFilter(1000, 0.01)) {
            assertNotNull(rf);
        }
    }

    @Test
    void testInsertAndContains() {
        try (RibbonFilter rf = new RibbonFilter(1000, 0.01)) {
            rf.insert("test".getBytes());
            assertTrue(rf.contains("test".getBytes()));
        }
    }

    @Test
    void testLargeDataset() {
        try (RibbonFilter rf = new RibbonFilter(10000, 0.01)) {
            for (int i = 0; i < 1000; i++) {
                rf.insert(("item-" + i).getBytes());
            }
            assertTrue(rf.contains("item-500".getBytes()));
        }
    }
}
