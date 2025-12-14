package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Unit tests for Conservative Count-Min - Conservative estimation variant.
 *
 * Never overestimates frequency, always underestimates or exact.
 */
@DisplayName("Conservative Count-Min - Conservative Estimates")
public class ConservativeCountMinTest {

    private ConservativeCountMin ccm;

    @BeforeEach
    void setUp() {
        ccm = new ConservativeCountMin(0.01, 0.001);
    }

    @AfterEach
    void tearDown() {
        if (ccm != null) {
            ccm.close();
        }
    }

    @Test
    @DisplayName("Should create valid Conservative Count-Min")
    void testConstructor() {
        ConservativeCountMin test = new ConservativeCountMin(0.01, 0.001);
        assertNotNull(test);
        test.close();
    }

    @Test
    @DisplayName("Should estimate frequencies conservatively")
    void testConservativeEstimate() {
        for (int i = 0; i < 100; i++) {
            ccm.update("item".getBytes());
        }

        long estimate = ccm.estimate("item".getBytes());
        assertTrue(estimate > 0);
        assertTrue(estimate <= 100);
    }

    @Test
    @DisplayName("Should never overestimate")
    void testNoOverestimate() {
        for (int i = 0; i < 1000; i++) {
            ccm.update(("item-" + (i % 10)).getBytes());
        }

        for (int i = 0; i < 10; i++) {
            long count = ccm.estimate(("item-" + i).getBytes());
            assertTrue(count <= 100);
        }
    }

    @Test
    @DisplayName("Should merge sketches")
    void testMerge() {
        ConservativeCountMin ccm2 = new ConservativeCountMin(0.01, 0.001);

        for (int i = 0; i < 50; i++) {
            ccm.update("item".getBytes());
        }
        for (int i = 0; i < 30; i++) {
            ccm2.update("item".getBytes());
        }

        ccm.merge(ccm2);
        long count = ccm.estimate("item".getBytes());

        assertTrue(count >= 70);
        ccm2.close();
    }

    @Test
    @DisplayName("Should serialize correctly")
    void testSerialization() {
        for (int i = 0; i < 1000; i++) {
            ccm.update(("item-" + (i % 10)).getBytes());
        }

        byte[] serialized = ccm.serialize();
        assertNotNull(serialized);
    }

    @Test
    @DisplayName("Try-with-resources should work")
    void testTryWithResources() {
        try (ConservativeCountMin test = new ConservativeCountMin(0.01, 0.001)) {
            test.update("data".getBytes());
            assertTrue(test.estimate("data".getBytes()) > 0);
        }
    }
}
