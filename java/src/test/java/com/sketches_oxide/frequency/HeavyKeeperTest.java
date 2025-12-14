package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("HeavyKeeper - Heavy Hitter Detection")
public class HeavyKeeperTest {

    private HeavyKeeper hk;

    @BeforeEach
    void setUp() {
        hk = new HeavyKeeper(1000, 0.01);
    }

    @AfterEach
    void tearDown() {
        if (hk != null) {
            hk.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(hk);
    }

    @Test
    void testUpdate() {
        for (int i = 0; i < 100; i++) {
            hk.update("heavy".getBytes());
        }
        long estimate = hk.estimate("heavy".getBytes());
        assertTrue(estimate > 0);
    }

    @Test
    void testDecay() {
        hk.update("item".getBytes());
        hk.decay();
        assertNotNull(hk);
    }

    @Test
    void testMerge() {
        HeavyKeeper hk2 = new HeavyKeeper(1000, 0.01);
        hk.update("item".getBytes());
        hk2.update("item".getBytes());
        hk.merge(hk2);
        hk2.close();
    }

    @Test
    void testTryWithResources() {
        try (HeavyKeeper test = new HeavyKeeper(1000, 0.01)) {
            test.update("data".getBytes());
            assertTrue(test.estimate("data".getBytes()) >= 0);
        }
    }
}
