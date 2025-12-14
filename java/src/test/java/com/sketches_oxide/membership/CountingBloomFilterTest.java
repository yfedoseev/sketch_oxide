package com.sketches_oxide.membership;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("CountingBloomFilter - Supports Removal")
public class CountingBloomFilterTest {

    @Test
    void testCreate() {
        try (CountingBloomFilter cbf = new CountingBloomFilter(1000, 0.01)) {
            assertNotNull(cbf);
        }
    }

    @Test
    void testInsertAndContains() {
        try (CountingBloomFilter cbf = new CountingBloomFilter(1000, 0.01)) {
            cbf.insert("test".getBytes());
            assertTrue(cbf.contains("test".getBytes()));
        }
    }

    @Test
    void testMultipleInserts() {
        try (CountingBloomFilter cbf = new CountingBloomFilter(1000, 0.01)) {
            cbf.insert("item".getBytes());
            cbf.insert("item".getBytes());
            assertTrue(cbf.contains("item".getBytes()));
        }
    }
}
