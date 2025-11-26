package com.sketches_oxide.tier1;

import com.sketches_oxide.frequency.HeavyKeeper;
import com.sketches_oxide.reconciliation.RatelessIBLT;
import com.sketches_oxide.range_filters.Grafite;
import com.sketches_oxide.range_filters.MementoFilter;
import com.sketches_oxide.streaming.SlidingHyperLogLog;
import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

public class NewSketchesTest {

    // ==========================
    // HeavyKeeper Tests
    // ==========================

    @Test
    void testHeavyKeeperConstruction() {
        try (HeavyKeeper hk = new HeavyKeeper(10, 0.001, 0.01)) {
            assertNotNull(hk);
        }
    }

    @Test
    void testHeavyKeeperUpdate() {
        try (HeavyKeeper hk = new HeavyKeeper(10, 0.001, 0.01)) {
            hk.update("item1".getBytes());
            hk.update("item2".getBytes());
            hk.update("item1".getBytes());

            int count = hk.estimate("item1".getBytes());
            assertTrue(count >= 2);
        }
    }

    @Test
    void testHeavyKeeperTopK() {
        try (HeavyKeeper hk = new HeavyKeeper(5, 0.001, 0.01)) {
            for (int i = 0; i < 100; i++) {
                hk.update(("item_" + (i % 10)).getBytes());
            }

            var topK = hk.topK();
            assertFalse(topK.isEmpty());
            assertTrue(topK.size() <= 5);
        }
    }

    @Test
    void testHeavyKeeperDecay() {
        try (HeavyKeeper hk = new HeavyKeeper(10, 0.001, 0.01)) {
            for (int i = 0; i < 100; i++) {
                hk.update("item".getBytes());
            }

            int before = hk.estimate("item".getBytes());
            hk.decay();
            int after = hk.estimate("item".getBytes());

            assertTrue(after < before);
        }
    }

    @Test
    void testHeavyKeeperMerge() {
        try (HeavyKeeper hk1 = new HeavyKeeper(10, 0.001, 0.01);
             HeavyKeeper hk2 = new HeavyKeeper(10, 0.001, 0.01)) {

            for (int i = 0; i < 50; i++) {
                hk1.update("item".getBytes());
            }
            for (int i = 0; i < 30; i++) {
                hk2.update("item".getBytes());
            }

            hk1.merge(hk2);
            int count = hk1.estimate("item".getBytes());
            assertTrue(count >= 70 && count <= 90);
        }
    }

    // ==========================
    // RatelessIBLT Tests
    // ==========================

    @Test
    void testRatelessIBLTConstruction() {
        try (RatelessIBLT iblt = new RatelessIBLT(100, 32)) {
            assertNotNull(iblt);
        }
    }

    @Test
    void testRatelessIBLTInsert() {
        try (RatelessIBLT iblt = new RatelessIBLT(10, 32)) {
            iblt.insert("key1".getBytes(), "value1".getBytes());
            iblt.insert("key2".getBytes(), "value2".getBytes());
        }
    }

    @Test
    void testRatelessIBLTDecode() {
        try (RatelessIBLT iblt = new RatelessIBLT(10, 32)) {
            iblt.insert("key1".getBytes(), "value1".getBytes());
            iblt.insert("key2".getBytes(), "value2".getBytes());

            var diff = iblt.decode();
            assertNotNull(diff);
            assertEquals(2, diff.toInsert.size());
            assertEquals(0, diff.toRemove.size());
        }
    }

    @Test
    void testRatelessIBLTSubtract() {
        try (RatelessIBLT alice = new RatelessIBLT(10, 32);
             RatelessIBLT bob = new RatelessIBLT(10, 32)) {

            alice.insert("shared".getBytes(), "value".getBytes());
            bob.insert("shared".getBytes(), "value".getBytes());

            alice.insert("alice_only".getBytes(), "a_value".getBytes());
            bob.insert("bob_only".getBytes(), "b_value".getBytes());

            alice.subtract(bob);
            var diff = alice.decode();

            assertNotNull(diff);
            assertTrue(diff.toInsert.size() >= 1); // alice_only
            assertTrue(diff.toRemove.size() >= 1); // bob_only
        }
    }

    // ==========================
    // Grafite Tests
    // ==========================

    @Test
    void testGrafiteBuild() {
        long[] keys = {10, 20, 30, 40, 50};
        try (Grafite filter = Grafite.build(keys, 6)) {
            assertNotNull(filter);
            assertEquals(5, filter.keyCount());
        }
    }

    @Test
    void testGrafiteMayContain() {
        long[] keys = {100, 200, 300};
        try (Grafite filter = Grafite.build(keys, 6)) {
            assertTrue(filter.mayContain(200));
        }
    }

    @Test
    void testGrafiteMayContainRange() {
        long[] keys = {10, 20, 30, 40, 50};
        try (Grafite filter = Grafite.build(keys, 6)) {
            assertTrue(filter.mayContainRange(15, 25)); // Contains 20
            assertTrue(filter.mayContainRange(10, 50)); // Contains all
        }
    }

    @Test
    void testGrafiteExpectedFpr() {
        long[] keys = {1, 2, 3};
        try (Grafite filter = Grafite.build(keys, 6)) {
            double fpr = filter.expectedFpr(10);
            assertTrue(fpr >= 0.0 && fpr <= 1.0);
        }
    }

    // ==========================
    // MementoFilter Tests
    // ==========================

    @Test
    void testMementoFilterConstruction() {
        try (MementoFilter filter = new MementoFilter(1000, 0.01)) {
            assertNotNull(filter);
            assertEquals(0, filter.len());
        }
    }

    @Test
    void testMementoFilterInsert() {
        try (MementoFilter filter = new MementoFilter(1000, 0.01)) {
            filter.insert(42, "value1".getBytes());
            filter.insert(100, "value2".getBytes());

            assertEquals(2, filter.len());
        }
    }

    @Test
    void testMementoFilterMayContainRange() {
        try (MementoFilter filter = new MementoFilter(1000, 0.01)) {
            filter.insert(50, "value".getBytes());

            assertTrue(filter.mayContainRange(45, 55));
        }
    }

    // ==========================
    // SlidingHyperLogLog Tests
    // ==========================

    @Test
    void testSlidingHLLConstruction() {
        try (SlidingHyperLogLog hll = new SlidingHyperLogLog(12, 3600)) {
            assertNotNull(hll);
            assertEquals(12, hll.precision());
        }
    }

    @Test
    void testSlidingHLLUpdate() {
        try (SlidingHyperLogLog hll = new SlidingHyperLogLog(12, 3600)) {
            for (int i = 0; i < 100; i++) {
                hll.update(("item_" + i).getBytes(), 1000);
            }

            double estimate = hll.estimateTotal();
            assertTrue(estimate >= 70 && estimate <= 130);
        }
    }

    @Test
    void testSlidingHLLWindowEstimation() {
        try (SlidingHyperLogLog hll = new SlidingHyperLogLog(12, 3600)) {
            // Add items at timestamp 1000
            for (int i = 0; i < 50; i++) {
                hll.update(("early_" + i).getBytes(), 1000);
            }

            // Add items at timestamp 2000
            for (int i = 0; i < 50; i++) {
                hll.update(("late_" + i).getBytes(), 2000);
            }

            // Query window covering only second batch
            double windowEstimate = hll.estimateWindow(2500, 600);
            assertTrue(windowEstimate >= 20);
        }
    }

    @Test
    void testSlidingHLLDecay() {
        try (SlidingHyperLogLog hll = new SlidingHyperLogLog(12, 3600)) {
            hll.update("old_item".getBytes(), 1000);

            hll.decay(5000, 600);

            // Old item should be removed
            double estimate = hll.estimateWindow(5000, 600);
            assertTrue(estimate < 1.5);
        }
    }

    @Test
    void testSlidingHLLMerge() {
        try (SlidingHyperLogLog hll1 = new SlidingHyperLogLog(12, 3600);
             SlidingHyperLogLog hll2 = new SlidingHyperLogLog(12, 3600)) {

            for (int i = 0; i < 50; i++) {
                hll1.update(("item_" + i).getBytes(), 1000);
            }
            for (int i = 25; i < 75; i++) {
                hll2.update(("item_" + i).getBytes(), 1000);
            }

            hll1.merge(hll2);
            double estimate = hll1.estimateTotal();
            assertTrue(estimate >= 50 && estimate <= 100);
        }
    }

    @Test
    void testSlidingHLLSerialization() {
        try (SlidingHyperLogLog hll = new SlidingHyperLogLog(12, 3600)) {
            for (int i = 0; i < 100; i++) {
                hll.update(("item_" + i).getBytes(), 1000);
            }

            byte[] data = hll.serialize();
            assertNotNull(data);
            assertTrue(data.length > 0);

            try (SlidingHyperLogLog restored = SlidingHyperLogLog.deserialize(data)) {
                assertEquals(hll.precision(), restored.precision());
            }
        }
    }
}
