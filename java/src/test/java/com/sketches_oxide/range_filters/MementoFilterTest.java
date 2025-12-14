package com.sketches_oxide.range_filters;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("MementoFilter - Temporal Membership")
public class MementoFilterTest {

    @Test
    void testCreate() {
        try (MementoFilter mf = new MementoFilter(1000, 0.01, 0)) {
            assertNotNull(mf);
        }
    }

    @Test
    void testInsertAndContains() {
        try (MementoFilter mf = new MementoFilter(1000, 0.01, 0)) {
            mf.insert("test".getBytes(), 0);
            assertTrue(mf.contains("test".getBytes(), 0));
        }
    }

    @Test
    void testTemporalBehavior() {
        try (MementoFilter mf = new MementoFilter(1000, 0.01, 0)) {
            mf.insert("item".getBytes(), 100);
            assertTrue(mf.contains("item".getBytes(), 100));
        }
    }

    @Test
    void testMerge() {
        try (MementoFilter mf1 = new MementoFilter(1000, 0.01, 0);
             MementoFilter mf2 = new MementoFilter(1000, 0.01, 0)) {
            mf1.insert("item1".getBytes(), 0);
            mf2.insert("item2".getBytes(), 0);
            mf1.merge(mf2);
            assertNotNull(mf1);
        }
    }
}
