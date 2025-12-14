package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("NitroSketch - Weighted Frequent Items")
public class NitroSketchTest {

    private NitroSketch ns;

    @BeforeEach
    void setUp() {
        ns = new NitroSketch(100);
    }

    @AfterEach
    void tearDown() {
        if (ns != null) {
            ns.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(ns);
    }

    @Test
    void testUpdate() {
        ns.update("item".getBytes(), 1.0);
        assertNotNull(ns);
    }

    @Test
    void testWeightedUpdates() {
        ns.update("heavy".getBytes(), 100.0);
        ns.update("light".getBytes(), 1.0);
        assertNotNull(ns);
    }

    @Test
    void testMerge() {
        NitroSketch ns2 = new NitroSketch(100);
        ns.update("item".getBytes(), 1.0);
        ns2.update("item".getBytes(), 1.0);
        ns.merge(ns2);
        ns2.close();
    }

    @Test
    void testTryWithResources() {
        try (NitroSketch test = new NitroSketch(100)) {
            test.update("data".getBytes(), 1.0);
            assertNotNull(test);
        }
    }
}
