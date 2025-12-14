package com.sketches_oxide.membership;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("BlockedBloomFilter - Cache Efficient")
public class BlockedBloomFilterTest {

    @Test
    void testCreate() {
        try (BlockedBloomFilter bbf = new BlockedBloomFilter(1000, 0.01)) {
            assertNotNull(bbf);
        }
    }

    @Test
    void testInsertAndContains() {
        try (BlockedBloomFilter bbf = new BlockedBloomFilter(1000, 0.01)) {
            bbf.insert("test".getBytes());
            assertTrue(bbf.contains("test".getBytes()));
        }
    }

    @Test
    void testMerge() {
        try (BlockedBloomFilter bbf1 = new BlockedBloomFilter(1000, 0.01);
             BlockedBloomFilter bbf2 = new BlockedBloomFilter(1000, 0.01)) {
            bbf1.insert("item1".getBytes());
            bbf2.insert("item2".getBytes());
            bbf1.merge(bbf2);
            assertNotNull(bbf1);
        }
    }
}
