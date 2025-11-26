package com.sketches.oxide;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive test suite for VacuumFilter.
 * Categories: construction, basic operations, advanced features, edge cases, real-world scenarios.
 */
public class VacuumFilterTest {

    // CONSTRUCTION TESTS

    @Test
    public void testBasicConstruction() {
        try (VacuumFilter filter = new VacuumFilter(1000, 0.01)) {
            assertNotNull(filter);
            assertTrue(filter.isEmpty());
            assertEquals(0, filter.size());
        }
    }

    @Test
    public void testInvalidCapacityZero() {
        assertThrows(IllegalArgumentException.class, () -> {
            new VacuumFilter(0, 0.01);
        });
    }

    @Test
    public void testInvalidCapacityNegative() {
        assertThrows(IllegalArgumentException.class, () -> {
            new VacuumFilter(-100, 0.01);
        });
    }

    @Test
    public void testInvalidFprZero() {
        assertThrows(IllegalArgumentException.class, () -> {
            new VacuumFilter(1000, 0.0);
        });
    }

    @Test
    public void testInvalidFprOne() {
        assertThrows(IllegalArgumentException.class, () -> {
            new VacuumFilter(1000, 1.0);
        });
    }

    @Test
    public void testInvalidFprNegative() {
        assertThrows(IllegalArgumentException.class, () -> {
            new VacuumFilter(1000, -0.01);
        });
    }

    // BASIC OPERATIONS TESTS

    @Test
    public void testInsertAndContains() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            byte[] key = "hello".getBytes();
            filter.insert(key);
            assertTrue(filter.contains(key));
            assertEquals(1, filter.size());
        }
    }

    @Test
    public void testContainsNonexistent() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            assertFalse(filter.contains("nonexistent".getBytes()));
        }
    }

    @Test
    public void testDeleteExisting() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            byte[] key = "delete_me".getBytes();
            filter.insert(key);
            assertTrue(filter.contains(key));

            assertTrue(filter.delete(key));
            assertFalse(filter.contains(key));
            assertEquals(0, filter.size());
        }
    }

    @Test
    public void testDeleteNonexistent() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            assertFalse(filter.delete("nonexistent".getBytes()));
        }
    }

    @Test
    public void testMultipleInserts() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            for (int i = 0; i < 50; i++) {
                filter.insert(("key" + i).getBytes());
            }
            assertEquals(50, filter.size());

            for (int i = 0; i < 50; i++) {
                assertTrue(filter.contains(("key" + i).getBytes()));
            }
        }
    }

    @Test
    public void testClear() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            filter.insert("key1".getBytes());
            filter.insert("key2".getBytes());
            assertFalse(filter.isEmpty());

            filter.clear();
            assertTrue(filter.isEmpty());
            assertEquals(0, filter.size());
            assertFalse(filter.contains("key1".getBytes()));
        }
    }

    // ADVANCED FEATURES TESTS

    @Test
    public void testLoadFactor() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            assertEquals(0.0, filter.loadFactor(), 0.001);

            filter.insert("key1".getBytes());
            assertTrue(filter.loadFactor() > 0.0);
            assertTrue(filter.loadFactor() <= 1.0);
        }
    }

    @Test
    public void testCapacity() {
        try (VacuumFilter filter = new VacuumFilter(1000, 0.01)) {
            long capacity = filter.capacity();
            assertTrue(capacity >= 1000);
        }
    }

    @Test
    public void testMemoryUsage() {
        try (VacuumFilter filter = new VacuumFilter(1000, 0.01)) {
            long memory = filter.memoryUsage();
            assertTrue(memory > 0);
        }
    }

    @Test
    public void testIsEmpty() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            assertTrue(filter.isEmpty());
            filter.insert("key".getBytes());
            assertFalse(filter.isEmpty());
            filter.delete("key".getBytes());
            assertTrue(filter.isEmpty());
        }
    }

    // EDGE CASES TESTS

    @Test
    public void testNullKeyInsert() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            assertThrows(NullPointerException.class, () -> {
                filter.insert(null);
            });
        }
    }

    @Test
    public void testNullKeyContains() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            assertThrows(NullPointerException.class, () -> {
                filter.contains(null);
            });
        }
    }

    @Test
    public void testNullKeyDelete() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            assertThrows(NullPointerException.class, () -> {
                filter.delete(null);
            });
        }
    }

    @Test
    public void testEmptyKey() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            byte[] empty = new byte[0];
            filter.insert(empty);
            assertTrue(filter.contains(empty));
        }
    }

    @Test
    public void testDuplicateInserts() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            byte[] key = "duplicate".getBytes();
            filter.insert(key);
            filter.insert(key);
            filter.insert(key);
            assertTrue(filter.contains(key));
            // Size behavior for duplicates may vary
        }
    }

    @Test
    public void testLargeKey() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            byte[] largeKey = new byte[1024];
            for (int i = 0; i < largeKey.length; i++) {
                largeKey[i] = (byte) (i % 256);
            }
            filter.insert(largeKey);
            assertTrue(filter.contains(largeKey));
        }
    }

    // REAL-WORLD SCENARIOS TESTS

    @Test
    public void testUrlFiltering() {
        try (VacuumFilter filter = new VacuumFilter(1000, 0.01)) {
            String[] urls = {
                "https://example.com",
                "https://google.com",
                "https://github.com",
                "https://stackoverflow.com"
            };

            for (String url : urls) {
                filter.insert(url.getBytes());
            }

            for (String url : urls) {
                assertTrue(filter.contains(url.getBytes()));
            }

            assertFalse(filter.contains("https://unknown.com".getBytes()));
        }
    }

    @Test
    public void testEmailFiltering() {
        try (VacuumFilter filter = new VacuumFilter(500, 0.01)) {
            String[] emails = {
                "user1@example.com",
                "user2@example.com",
                "admin@company.com"
            };

            for (String email : emails) {
                filter.insert(email.getBytes());
            }

            assertTrue(filter.contains("user1@example.com".getBytes()));
            assertFalse(filter.contains("spam@unknown.com".getBytes()));
        }
    }

    @Test
    public void testIPAddressFiltering() {
        try (VacuumFilter filter = new VacuumFilter(256, 0.01)) {
            for (int i = 0; i < 100; i++) {
                String ip = "192.168.1." + i;
                filter.insert(ip.getBytes());
            }

            assertTrue(filter.contains("192.168.1.50".getBytes()));
            assertFalse(filter.contains("10.0.0.1".getBytes()));
        }
    }

    @Test
    public void testDynamicMembershipUpdates() {
        try (VacuumFilter filter = new VacuumFilter(200, 0.01)) {
            // Simulate user session tracking
            String[] activeSessions = {"session1", "session2", "session3"};

            for (String session : activeSessions) {
                filter.insert(session.getBytes());
            }

            // Expire session1
            filter.delete("session1".getBytes());
            assertFalse(filter.contains("session1".getBytes()));
            assertTrue(filter.contains("session2".getBytes()));
        }
    }

    @Test
    public void testResourceManagement() {
        VacuumFilter filter = new VacuumFilter(100, 0.01);
        filter.insert("test".getBytes());
        filter.close();

        // Operations after close should throw
        assertThrows(IllegalStateException.class, () -> {
            filter.contains("test".getBytes());
        });
    }

    @Test
    public void testTryWithResources() {
        try (VacuumFilter filter = new VacuumFilter(100, 0.01)) {
            filter.insert("test".getBytes());
            assertTrue(filter.contains("test".getBytes()));
        } // Automatically closed
    }
}
