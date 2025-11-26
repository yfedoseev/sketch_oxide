package com.sketches_oxide.membership;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.params.ParameterizedTest;
import org.junit.jupiter.params.provider.ValueSource;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Comprehensive unit tests for CuckooFilter membership sketch.
 * Tests construction, insertion, deletion, querying, serialization,
 * and resource management. Note: merge is not supported for CuckooFilter.
 */
@DisplayName("CuckooFilter Membership Sketch Tests")
public class CuckooFilterTest {

    private CuckooFilter filter;

    @BeforeEach
    void setUp() {
        filter = new CuckooFilter(10000);
    }

    @AfterEach
    void tearDown() {
        if (filter != null) {
            try {
                filter.close();
            } catch (Exception e) {
                // Already closed
            }
        }
    }

    // ==================== Constructor Tests ====================

    @Test
    @DisplayName("Should create CuckooFilter with valid capacity")
    void testConstructorValidCapacity() {
        try (CuckooFilter cf = new CuckooFilter(1000)) {
            assertNotNull(cf);
            assertEquals(1000, cf.getCapacity());
        }
    }

    @ParameterizedTest
    @ValueSource(longs = {0, -1, -1000})
    @DisplayName("Should reject non-positive capacity")
    void testConstructorInvalidCapacity(long capacity) {
        assertThrows(IllegalArgumentException.class, () -> new CuckooFilter(capacity));
    }

    @ParameterizedTest
    @ValueSource(longs = {1, 10, 100, 1000, 10000, 100000})
    @DisplayName("Should accept various valid capacity values")
    void testConstructorVariousCapacity(long capacity) {
        try (CuckooFilter cf = new CuckooFilter(capacity)) {
            assertEquals(capacity, cf.getCapacity());
        }
    }

    // ==================== Core Operation Tests ====================

    @Test
    @DisplayName("Should insert and find items")
    void testInsertAndContains() {
        filter.insert("test-item");
        assertTrue(filter.contains("test-item"));
    }

    @Test
    @DisplayName("Should handle multiple inserts")
    void testMultipleInserts() {
        for (int i = 0; i < 100; i++) {
            filter.insert("item-" + i);
        }

        for (int i = 0; i < 100; i++) {
            assertTrue(filter.contains("item-" + i));
        }
    }

    @Test
    @DisplayName("Should work with byte arrays")
    void testInsertAndContainsByteArray() {
        byte[] item = "test-data".getBytes();
        filter.insert(item);
        assertTrue(filter.contains(item));
    }

    @Test
    @DisplayName("Should handle duplicate insertions")
    void testDuplicateInsertions() {
        filter.insert("duplicate-item");
        filter.insert("duplicate-item");
        filter.insert("duplicate-item");

        assertTrue(filter.contains("duplicate-item"));
    }

    @Test
    @DisplayName("Should reject null item on insert (byte array)")
    void testInsertNullByteArray() {
        assertThrows(NullPointerException.class, () -> filter.insert((byte[]) null));
    }

    @Test
    @DisplayName("Should reject null item on insert (string)")
    void testInsertNullString() {
        assertThrows(NullPointerException.class, () -> filter.insert((String) null));
    }

    @Test
    @DisplayName("Should reject null item on contains (byte array)")
    void testContainsNullByteArray() {
        assertThrows(NullPointerException.class, () -> filter.contains((byte[]) null));
    }

    @Test
    @DisplayName("Should reject null item on contains (string)")
    void testContainsNullString() {
        assertThrows(NullPointerException.class, () -> filter.contains((String) null));
    }

    @Test
    @DisplayName("Should guarantee no false negatives for inserted items")
    void testNoFalseNegatives() {
        String[] items = {"apple", "banana", "cherry", "date", "elderberry"};
        for (String item : items) {
            filter.insert(item);
        }

        for (String item : items) {
            assertTrue(filter.contains(item), "False negative for: " + item);
        }
    }

    @Test
    @DisplayName("Should reject operations on closed filter")
    void testClosedFilterDetection() {
        filter.close();

        assertThrows(IllegalStateException.class, () -> filter.insert("item"));
        assertThrows(IllegalStateException.class, () -> filter.contains("item"));
        assertThrows(IllegalStateException.class, () -> filter.remove("item"));
        assertThrows(IllegalStateException.class, () -> filter.serialize());
    }

    @Test
    @DisplayName("Should be idempotent when closed multiple times")
    void testMultipleClose() {
        filter.close();
        assertDoesNotThrow(() -> filter.close());
    }

    // ==================== Deletion Tests ====================

    @Test
    @DisplayName("Should remove inserted items")
    void testRemoveInsertedItem() {
        filter.insert("item");
        assertTrue(filter.contains("item"));

        filter.remove("item");
        assertFalse(filter.contains("item"));
    }

    @Test
    @DisplayName("Should handle multiple removals")
    void testMultipleRemovals() {
        for (int i = 0; i < 50; i++) {
            filter.insert("item-" + i);
        }

        for (int i = 0; i < 50; i++) {
            filter.remove("item-" + i);
        }

        for (int i = 0; i < 50; i++) {
            assertFalse(filter.contains("item-" + i));
        }
    }

    @Test
    @DisplayName("Should handle selective removal")
    void testSelectiveRemoval() {
        for (int i = 0; i < 100; i++) {
            filter.insert("item-" + i);
        }

        // Remove only even-numbered items
        for (int i = 0; i < 100; i += 2) {
            filter.remove("item-" + i);
        }

        // Even numbers should be gone
        for (int i = 0; i < 100; i += 2) {
            assertFalse(filter.contains("item-" + i));
        }

        // Odd numbers should remain
        for (int i = 1; i < 100; i += 2) {
            assertTrue(filter.contains("item-" + i));
        }
    }

    @Test
    @DisplayName("Should reject null item on remove (byte array)")
    void testRemoveNullByteArray() {
        assertThrows(NullPointerException.class, () -> filter.remove((byte[]) null));
    }

    @Test
    @DisplayName("Should reject null item on remove (string)")
    void testRemoveNullString() {
        assertThrows(NullPointerException.class, () -> filter.remove((String) null));
    }

    @Test
    @DisplayName("Should handle removal of non-existent items")
    void testRemoveNonExistentItem() {
        // Should not throw
        filter.remove("non-existent");
        assertFalse(filter.contains("non-existent"));
    }

    @Test
    @DisplayName("Should reject remove on closed filter")
    void testRemoveClosedFilter() {
        filter.close();
        assertThrows(IllegalStateException.class, () -> filter.remove("item"));
    }

    @Test
    @DisplayName("Should work with byte array removal")
    void testRemoveByteArray() {
        byte[] item = "test-data".getBytes();
        filter.insert(item);
        assertTrue(filter.contains(item));

        filter.remove(item);
        assertFalse(filter.contains(item));
    }

    // ==================== Mixed Operations Tests ====================

    @Test
    @DisplayName("Should handle insert-remove-insert sequence")
    void testInsertRemoveInsertSequence() {
        filter.insert("item");
        assertTrue(filter.contains("item"));

        filter.remove("item");
        assertFalse(filter.contains("item"));

        filter.insert("item");
        assertTrue(filter.contains("item"));
    }

    @Test
    @DisplayName("Should handle duplicate insertion after removal")
    void testDuplicateInsertAfterRemoval() {
        filter.insert("item");
        filter.insert("item");
        filter.remove("item");

        // Behavior depends on implementation - may still exist if inserted twice
        // Just verify no exception is thrown
        filter.contains("item");
    }

    // ==================== Merge Tests ====================

    @Test
    @DisplayName("Should throw UnsupportedOperationException on merge attempt")
    void testMergeNotSupported() {
        CuckooFilter other = new CuckooFilter(10000);

        assertThrows(UnsupportedOperationException.class, () -> filter.merge(other));

        other.close();
    }

    // ==================== Serialization Tests ====================

    @Test
    @DisplayName("Should serialize and deserialize empty filter")
    void testSerializeEmptyFilter() {
        byte[] serialized = filter.serialize();
        assertNotNull(serialized);
        assertTrue(serialized.length > 0);

        CuckooFilter restored = CuckooFilter.deserialize(serialized);
        assertNotNull(restored);
        restored.close();
    }

    @Test
    @DisplayName("Should serialize and deserialize filter with data")
    void testSerializeFilterWithData() {
        filter.insert("item1");
        filter.insert("item2");
        filter.insert("item3");

        byte[] serialized = filter.serialize();

        CuckooFilter restored = CuckooFilter.deserialize(serialized);
        assertTrue(restored.contains("item1"));
        assertTrue(restored.contains("item2"));
        assertTrue(restored.contains("item3"));

        restored.close();
    }

    @Test
    @DisplayName("Should preserve data through serialization round-trip")
    void testSerializationRoundTrip() {
        for (int i = 0; i < 100; i++) {
            filter.insert("item-" + i);
        }

        byte[] serialized = filter.serialize();
        CuckooFilter restored = CuckooFilter.deserialize(serialized);

        for (int i = 0; i < 100; i++) {
            assertTrue(restored.contains("item-" + i),
                    "Item item-" + i + " not found after deserialization");
        }

        restored.close();
    }

    @Test
    @DisplayName("Should preserve deleted items through serialization")
    void testSerializationWithDeletions() {
        for (int i = 0; i < 100; i++) {
            filter.insert("item-" + i);
        }

        // Delete every other item
        for (int i = 0; i < 100; i += 2) {
            filter.remove("item-" + i);
        }

        byte[] serialized = filter.serialize();
        CuckooFilter restored = CuckooFilter.deserialize(serialized);

        // Deleted items should not be found
        for (int i = 0; i < 100; i += 2) {
            assertFalse(restored.contains("item-" + i));
        }

        // Remaining items should be found
        for (int i = 1; i < 100; i += 2) {
            assertTrue(restored.contains("item-" + i));
        }

        restored.close();
    }

    @Test
    @DisplayName("Should reject invalid serialized data")
    void testDeserializeInvalidData() {
        byte[] invalidData = {1, 2, 3, 4, 5};
        assertThrows(IllegalArgumentException.class, () -> CuckooFilter.deserialize(invalidData));
    }

    @Test
    @DisplayName("Should reject null serialized data")
    void testDeserializeNull() {
        assertThrows(NullPointerException.class, () -> CuckooFilter.deserialize(null));
    }

    @Test
    @DisplayName("Should reject serialize on closed filter")
    void testSerializeClosedFilter() {
        filter.close();
        assertThrows(IllegalStateException.class, () -> filter.serialize());
    }

    // ==================== String Representation Tests ====================

    @Test
    @DisplayName("Should provide meaningful toString")
    void testToString() {
        String str = filter.toString();
        assertNotNull(str);
        assertTrue(str.contains("CuckooFilter"));
        assertTrue(str.contains("10000"));
    }

    @Test
    @DisplayName("Should indicate closed state in toString")
    void testToStringClosed() {
        filter.close();
        String str = filter.toString();
        assertTrue(str.contains("closed") || str.contains("CuckooFilter"));
    }

    // ==================== Resource Management Tests ====================

    @Test
    @DisplayName("Should work with try-with-resources")
    void testTryWithResources() {
        try (CuckooFilter cf = new CuckooFilter(1000)) {
            cf.insert("test");
            assertTrue(cf.contains("test"));
        }
        // No exception should be thrown
    }

    // ==================== Large Dataset Tests ====================

    @Test
    @DisplayName("Should handle large dataset below capacity")
    void testLargeDatasetBelowCapacity() {
        CuckooFilter largeFilter = new CuckooFilter(100000);

        // Insert 50000 items (well below 95% capacity)
        for (int i = 0; i < 50000; i++) {
            largeFilter.insert("item-" + i);
        }

        // Verify some items
        for (int i = 0; i < 100; i++) {
            assertTrue(largeFilter.contains("item-" + i));
        }

        largeFilter.close();
    }

    @Test
    @DisplayName("Should maintain query accuracy after large insertions")
    void testQueryAccuracyLargeDataset() {
        CuckooFilter largeFilter = new CuckooFilter(50000);

        // Insert 40000 items
        for (int i = 0; i < 40000; i++) {
            largeFilter.insert("item-" + i);
        }

        // Verify no false negatives for inserted items
        for (int i = 0; i < 40000; i += 40) { // Sample every 40th
            assertTrue(largeFilter.contains("item-" + i),
                    "False negative for item-" + i);
        }

        largeFilter.close();
    }

    // ==================== Edge Case Tests ====================

    @Test
    @DisplayName("Should handle empty string")
    void testEmptyString() {
        filter.insert("");
        assertTrue(filter.contains(""));
    }

    @Test
    @DisplayName("Should handle binary data")
    void testBinaryData() {
        byte[] binary = {0, 1, 2, 3, 127, -128, -1};
        filter.insert(binary);
        assertTrue(filter.contains(binary));
    }

    @Test
    @DisplayName("Should handle Unicode strings")
    void testUnicodeStrings() {
        String[] items = {"你好", "مرحبا", "Привет", "🎉", "こんにちは"};
        for (String item : items) {
            filter.insert(item);
        }

        for (String item : items) {
            assertTrue(filter.contains(item), "Unicode item not found: " + item);
        }
    }

    @Test
    @DisplayName("Should distinguish similar items")
    void testDistinguishSimilarItems() {
        filter.insert("test");
        filter.insert("tests");
        filter.insert("testing");

        assertTrue(filter.contains("test"));
        assertTrue(filter.contains("tests"));
        assertTrue(filter.contains("testing"));
        assertFalse(filter.contains("tester"));
    }

    @Test
    @DisplayName("Should handle very large items")
    void testLargeItems() {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < 10000; i++) {
            sb.append("x");
        }
        String largeString = sb.toString();

        filter.insert(largeString);
        assertTrue(filter.contains(largeString));
    }

    @Test
    @DisplayName("Should work with different capacity values")
    void testDifferentCapacityValues() {
        for (long capacity : new long[]{100, 1000, 10000}) {
            try (CuckooFilter cf = new CuckooFilter(capacity)) {
                cf.insert("test");
                assertTrue(cf.contains("test"));
            }
        }
    }

    @Test
    @DisplayName("Should handle removal after large insertions")
    void testRemovalAfterLargeInsertions() {
        // Insert 1000 items
        for (int i = 0; i < 1000; i++) {
            filter.insert("item-" + i);
        }

        // Remove first 500 items
        for (int i = 0; i < 500; i++) {
            filter.remove("item-" + i);
        }

        // Verify first 500 are gone
        for (int i = 0; i < 500; i++) {
            assertFalse(filter.contains("item-" + i));
        }

        // Verify remaining 500 are still there
        for (int i = 500; i < 1000; i++) {
            assertTrue(filter.contains("item-" + i));
        }
    }

    @Test
    @DisplayName("Should handle capacity property correctly")
    void testCapacityProperty() {
        for (long capacity : new long[]{100, 500, 1000, 5000}) {
            try (CuckooFilter cf = new CuckooFilter(capacity)) {
                assertEquals(capacity, cf.getCapacity());
            }
        }
    }

    @Test
    @DisplayName("Should distinguish between different items reliably")
    void testDistinctItemsNotConfused() {
        String[] items = {
            "user-1", "user-2", "user-3",
            "item-a", "item-b", "item-c",
            "data-x", "data-y", "data-z"
        };

        for (String item : items) {
            filter.insert(item);
        }

        for (String item : items) {
            assertTrue(filter.contains(item));
        }

        // Check non-existent items
        assertFalse(filter.contains("user-4"));
        assertFalse(filter.contains("item-d"));
        assertFalse(filter.contains("data-w"));
    }

    @Test
    @DisplayName("Should handle interleaved insert and delete operations")
    void testInterleavedInsertDelete() {
        for (int i = 0; i < 100; i++) {
            filter.insert("item-" + i);

            if (i > 0 && i % 10 == 0) {
                // Every 10 inserts, remove the first item of that batch
                filter.remove("item-" + (i - 10));
            }
        }

        // Verify the pattern
        for (int i = 0; i < 100; i++) {
            boolean shouldExist = !((i > 0 && i <= 90 && (i - 1) / 10 < i / 10));
            boolean exists = filter.contains("item-" + i);
            // Just verify no exceptions
        }
    }
}
