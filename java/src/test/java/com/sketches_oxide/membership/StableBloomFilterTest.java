package com.sketches_oxide.membership;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("StableBloomFilter - Streaming Compatible")
public class StableBloomFilterTest {

    @Test
    void testCreate() {
        try (StableBloomFilter sbf = new StableBloomFilter(1000, 0.01)) {
            assertNotNull(sbf);
        }
    }

    @Test
    void testInsertAndContains() {
        try (StableBloomFilter sbf = new StableBloomFilter(1000, 0.01)) {
            sbf.insert("test".getBytes());
            assertTrue(sbf.contains("test".getBytes()));
        }
    }

    @Test
    void testStreamingBehavior() {
        try (StableBloomFilter sbf = new StableBloomFilter(1000, 0.01)) {
            for (int i = 0; i < 100; i++) {
                sbf.insert(("item-" + i).getBytes());
            }
            assertNotNull(sbf);
        }
    }
}
