package com.sketches_oxide.similarity;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("SimHash - Fingerprint Similarity")
public class SimHashTest {

    private SimHash sh;

    @BeforeEach
    void setUp() {
        sh = new SimHash();
    }

    @AfterEach
    void tearDown() {
        if (sh != null) {
            sh.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(sh);
    }

    @Test
    void testUpdate() {
        sh.update("text content".getBytes());
        long fingerprint = sh.fingerprint();
        assertTrue(fingerprint != 0);
    }

    @Test
    void testSimilarity() {
        SimHash sh1 = new SimHash();
        SimHash sh2 = new SimHash();
        sh1.update("text".getBytes());
        sh2.update("text".getBytes());
        double sim = sh1.similarity(sh2);
        assertTrue(sim >= 0 && sim <= 1.0);
        sh1.close();
        sh2.close();
    }

    @Test
    void testHammingDistance() {
        SimHash sh2 = new SimHash();
        sh.update("hello".getBytes());
        sh2.update("hello".getBytes());
        int dist = sh.hammingDistance(sh2);
        assertTrue(dist >= 0);
        sh2.close();
    }

    @Test
    void testTryWithResources() {
        try (SimHash test = new SimHash()) {
            test.update("data".getBytes());
            assertTrue(test.fingerprint() != 0);
        }
    }
}
