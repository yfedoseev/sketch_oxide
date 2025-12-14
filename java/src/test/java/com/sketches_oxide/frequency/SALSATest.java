package com.sketches_oxide.frequency;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("SALSA - Streaming Algorithm")
public class SALSATest {

    private SALSA salsa;

    @BeforeEach
    void setUp() {
        salsa = new SALSA(0.01, 0.001);
    }

    @AfterEach
    void tearDown() {
        if (salsa != null) {
            salsa.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(salsa);
    }

    @Test
    void testUpdate() {
        salsa.update("item".getBytes());
        long query = salsa.query("item".getBytes());
        assertTrue(query > 0);
    }

    @Test
    void testMerge() {
        SALSA s2 = new SALSA(0.01, 0.001);
        salsa.update("item".getBytes());
        s2.update("item".getBytes());
        salsa.merge(s2);
        s2.close();
    }

    @Test
    void testTryWithResources() {
        try (SALSA test = new SALSA(0.01, 0.001)) {
            test.update("data".getBytes());
            assertTrue(test.query("data".getBytes()) > 0);
        }
    }
}
