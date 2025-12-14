package com.sketches_oxide.reconciliation;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;

import static org.junit.jupiter.api.Assertions.*;

@DisplayName("RatelessIBLT - Set Reconciliation")
public class RatelessIBLTTest {

    private RatelessIBLT iblt;

    @BeforeEach
    void setUp() {
        iblt = new RatelessIBLT(100, 3);
    }

    @AfterEach
    void tearDown() {
        if (iblt != null) {
            iblt.close();
        }
    }

    @Test
    void testConstructor() {
        assertNotNull(iblt);
    }

    @Test
    void testInsert() {
        iblt.insert("item".getBytes());
        assertNotNull(iblt);
    }

    @Test
    void testMultipleInserts() {
        for (int i = 0; i < 10; i++) {
            iblt.insert(("item-" + i).getBytes());
        }
        assertNotNull(iblt);
    }

    @Test
    void testTryWithResources() {
        try (RatelessIBLT test = new RatelessIBLT(100, 3)) {
            test.insert("data".getBytes());
            assertNotNull(test);
        }
    }
}
