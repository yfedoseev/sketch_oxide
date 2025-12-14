package com.sketches_oxide.membership;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("LearnedBloomFilter - ML-Optimized")
public class LearnedBloomFilterTest {

    @Test
    void testCreate() {
        try (LearnedBloomFilter lbf = new LearnedBloomFilter(1000, 0.01)) {
            assertNotNull(lbf);
        }
    }

    @Test
    void testInsertAndContains() {
        try (LearnedBloomFilter lbf = new LearnedBloomFilter(1000, 0.01)) {
            lbf.insert("test".getBytes());
            assertTrue(lbf.contains("test".getBytes()));
        }
    }

    @Test
    void testMLOptimization() {
        try (LearnedBloomFilter lbf = new LearnedBloomFilter(10000, 0.01)) {
            for (int i = 0; i < 1000; i++) {
                lbf.insert(("item-" + (i % 100)).getBytes());
            }
            assertTrue(lbf.contains("item-50".getBytes()));
        }
    }
}
