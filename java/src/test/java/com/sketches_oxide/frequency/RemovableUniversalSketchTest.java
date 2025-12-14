package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("RemovableUniversalSketch - Removable Frequencies")
public class RemovableUniversalSketchTest {

    private RemovableUniversalSketch rus;

    @BeforeEach
    void setUp() {
        rus = new RemovableUniversalSketch(0.01, 0.001);
    }

    @AfterEach
    void tearDown() {
        if (rus != null) {
            rus.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(rus);
    }

    @Test
    void testUpdate() {
        rus.update("item".getBytes());
        long estimate = rus.estimate("item".getBytes());
        assertTrue(estimate > 0);
    }

    @Test
    void testMerge() {
        RemovableUniversalSketch rus2 = new RemovableUniversalSketch(0.01, 0.001);
        rus.update("item".getBytes());
        rus2.update("item".getBytes());
        rus.merge(rus2);
        rus2.close();
    }

    @Test
    void testTryWithResources() {
        try (RemovableUniversalSketch test = new RemovableUniversalSketch(0.01, 0.001)) {
            test.update("data".getBytes());
            assertTrue(test.estimate("data".getBytes()) > 0);
        }
    }
}
