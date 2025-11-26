package com.sketches.oxide;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;
import java.util.ArrayList;
import java.util.List;

/**
 * Comprehensive test suite for all Tier 2 sketches: GRF, NitroSketch, UnivMon, LearnedBloomFilter.
 * Total tests: 60+ across all sketches.
 */
public class Tier2SketchesTest {

    // ============================================================================
    // GRF TESTS (15 tests)
    // ============================================================================

    @Test
    public void testGRFBasicConstruction() {
        long[] keys = {10, 20, 30, 40, 50};
        try (GRF grf = GRF.build(keys, 6)) {
            assertNotNull(grf);
            assertEquals(5, grf.keyCount());
        }
    }

    @Test
    public void testGRFRangeQuery() {
        long[] keys = {10, 20, 30, 40, 50};
        try (GRF grf = GRF.build(keys, 6)) {
            assertTrue(grf.mayContainRange(15, 25)); // Contains 20
            assertTrue(grf.mayContainRange(10, 50)); // Full range
        }
    }

    @Test
    public void testGRFPointQuery() {
        long[] keys = {10, 20, 30, 40, 50};
        try (GRF grf = GRF.build(keys, 6)) {
            assertTrue(grf.mayContain(20));
            assertTrue(grf.mayContain(50));
        }
    }

    @Test
    public void testGRFInvalidEmptyKeys() {
        assertThrows(IllegalArgumentException.class, () -> {
            GRF.build(new long[0], 6);
        });
    }

    @Test
    public void testGRFInvalidBitsPerKey() {
        long[] keys = {10, 20, 30};
        assertThrows(IllegalArgumentException.class, () -> {
            GRF.build(keys, 1); // Too small
        });
    }

    @Test
    public void testGRFSegmentCount() {
        long[] keys = {1, 2, 3, 4, 5, 6, 7, 8, 9, 10};
        try (GRF grf = GRF.build(keys, 6)) {
            assertTrue(grf.segmentCount() > 0);
        }
    }

    @Test
    public void testGRFBitsPerKey() {
        long[] keys = {10, 20, 30};
        try (GRF grf = GRF.build(keys, 8)) {
            assertEquals(8, grf.bitsPerKey());
        }
    }

    @Test
    public void testGRFExpectedFpr() {
        long[] keys = {10, 20, 30, 40, 50};
        try (GRF grf = GRF.build(keys, 6)) {
            double fpr = grf.expectedFpr(10);
            assertTrue(fpr >= 0.0 && fpr <= 1.0);
        }
    }

    @Test
    public void testGRFFibonacciSequence() {
        long[] fibonacci = {1, 2, 3, 5, 8, 13, 21, 34, 55, 89};
        try (GRF grf = GRF.build(fibonacci, 6)) {
            assertTrue(grf.mayContain(13));
            assertTrue(grf.mayContainRange(10, 25));
        }
    }

    @Test
    public void testGRFTimeSeriesData() {
        long[] timestamps = new long[100];
        long base = 1700000000L; // Unix timestamp
        for (int i = 0; i < 100; i++) {
            timestamps[i] = base + i * 3600; // Hourly timestamps
        }
        try (GRF grf = GRF.build(timestamps, 6)) {
            assertTrue(grf.keyCount() == 100);
        }
    }

    @Test
    public void testGRFResourceManagement() {
        long[] keys = {1, 2, 3};
        GRF grf = GRF.build(keys, 6);
        grf.close();
        assertThrows(IllegalStateException.class, () -> {
            grf.keyCount();
        });
    }

    @Test
    public void testGRFLargeKeySet() {
        long[] keys = new long[1000];
        for (int i = 0; i < 1000; i++) {
            keys[i] = i * 10;
        }
        try (GRF grf = GRF.build(keys, 6)) {
            assertEquals(1000, grf.keyCount());
            assertTrue(grf.mayContain(500));
        }
    }

    @Test
    public void testGRFSkewedDistribution() {
        long[] keys = new long[100];
        // Power law distribution
        for (int i = 0; i < 100; i++) {
            keys[i] = (long) Math.pow(2, i % 20);
        }
        try (GRF grf = GRF.build(keys, 6)) {
            assertTrue(grf.segmentCount() > 0);
        }
    }

    @Test
    public void testGRFSingleKey() {
        long[] keys = {42};
        try (GRF grf = GRF.build(keys, 6)) {
            assertTrue(grf.mayContain(42));
            assertEquals(1, grf.keyCount());
        }
    }

    @Test
    public void testGRFDuplicateKeys() {
        long[] keys = {10, 20, 10, 30, 20, 40};
        try (GRF grf = GRF.build(keys, 6)) {
            // Should deduplicate
            assertTrue(grf.keyCount() <= 6);
        }
    }

    // ============================================================================
    // NITROSKETCH TESTS (15 tests)
    // ============================================================================

    @Test
    public void testNitroBasicConstruction() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.1)) {
            assertNotNull(nitro);
            assertEquals(0.1, nitro.sampleRate(), 0.001);
        }
    }

    @Test
    public void testNitroUpdateAndQuery() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.1)) {
            nitro.updateSampled("flow1".getBytes());
            nitro.sync(1.0);
            long freq = nitro.query("flow1".getBytes());
            assertTrue(freq >= 0);
        }
    }

    @Test
    public void testNitroSamplingCounts() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.5)) {
            for (int i = 0; i < 1000; i++) {
                nitro.updateSampled(("flow" + i).getBytes());
            }
            assertTrue(nitro.sampledCount() > 0);
            assertTrue(nitro.unsampledCount() >= 0);
        }
    }

    @Test
    public void testNitroInvalidSampleRateZero() {
        assertThrows(IllegalArgumentException.class, () -> {
            new NitroSketch(0.01, 0.01, 0.0);
        });
    }

    @Test
    public void testNitroInvalidSampleRateTooLarge() {
        assertThrows(IllegalArgumentException.class, () -> {
            new NitroSketch(0.01, 0.01, 1.1);
        });
    }

    @Test
    public void testNitroResetStats() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.5)) {
            nitro.updateSampled("test".getBytes());
            assertTrue(nitro.sampledCount() + nitro.unsampledCount() > 0);

            nitro.resetStats();
            assertEquals(0, nitro.sampledCount() + nitro.unsampledCount());
        }
    }

    @Test
    public void testNitroHighThroughput() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.01)) {
            // Simulate 100K updates
            for (int i = 0; i < 100000; i++) {
                nitro.updateSampled(("packet" + (i % 1000)).getBytes());
            }
            nitro.sync(1.0);
            assertTrue(nitro.sampledCount() > 0);
        }
    }

    @Test
    public void testNitroNetworkFlows() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.1)) {
            String[] flows = {
                "192.168.1.1:443->10.0.0.1:8080",
                "192.168.1.2:443->10.0.0.1:8080",
                "192.168.1.1:443->10.0.0.2:8080"
            };
            for (String flow : flows) {
                nitro.updateSampled(flow.getBytes());
            }
            nitro.sync(1.0);
        }
    }

    @Test
    public void testNitroSync() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.5)) {
            for (int i = 0; i < 100; i++) {
                nitro.updateSampled(("item" + i).getBytes());
            }
            nitro.sync(1.0);
            nitro.sync(0.5);
        }
    }

    @Test
    public void testNitroNullKey() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.5)) {
            assertThrows(NullPointerException.class, () -> {
                nitro.updateSampled(null);
            });
        }
    }

    @Test
    public void testNitroResourceManagement() {
        NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.5);
        nitro.updateSampled("test".getBytes());
        nitro.close();
        assertThrows(IllegalStateException.class, () -> {
            nitro.sampleRate();
        });
    }

    @Test
    public void testNitroFullSampling() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 1.0)) {
            for (int i = 0; i < 100; i++) {
                nitro.updateSampled(("item" + i).getBytes());
            }
            // With 100% sampling, all items should be sampled
            assertEquals(100, nitro.sampledCount());
            assertEquals(0, nitro.unsampledCount());
        }
    }

    @Test
    public void testNitroLowSampling() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.01)) {
            for (int i = 0; i < 1000; i++) {
                nitro.updateSampled(("item" + i).getBytes());
            }
            // With 1% sampling, most should be unsampled
            assertTrue(nitro.unsampledCount() > nitro.sampledCount());
        }
    }

    @Test
    public void testNitroEmptyKey() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.5)) {
            nitro.updateSampled(new byte[0]);
            nitro.sync(1.0);
        }
    }

    @Test
    public void testNitroLargeKey() {
        try (NitroSketch nitro = new NitroSketch(0.01, 0.01, 0.5)) {
            byte[] largeKey = new byte[1024];
            nitro.updateSampled(largeKey);
            nitro.sync(1.0);
        }
    }

    // ============================================================================
    // UNIVMON TESTS (15 tests)
    // ============================================================================

    @Test
    public void testUnivMonBasicConstruction() {
        try (UnivMon univmon = new UnivMon(10000, 0.01, 0.01)) {
            assertNotNull(univmon);
            assertTrue(univmon.numLayers() >= 3);
        }
    }

    @Test
    public void testUnivMonUpdate() {
        try (UnivMon univmon = new UnivMon(10000, 0.01, 0.01)) {
            univmon.update("item1".getBytes(), 100.0);
            univmon.update("item2".getBytes(), 200.0);
            assertTrue(univmon.totalUpdates() == 2);
        }
    }

    @Test
    public void testUnivMonL1Estimation() {
        try (UnivMon univmon = new UnivMon(10000, 0.01, 0.01)) {
            univmon.update("A".getBytes(), 100.0);
            univmon.update("B".getBytes(), 200.0);
            univmon.update("C".getBytes(), 300.0);

            double l1 = univmon.estimateL1();
            assertTrue(l1 > 0.0);
            // Should be approximately 600.0
            assertTrue(Math.abs(l1 - 600.0) < 100.0);
        }
    }

    @Test
    public void testUnivMonL2Estimation() {
        try (UnivMon univmon = new UnivMon(10000, 0.01, 0.01)) {
            univmon.update("A".getBytes(), 100.0);
            univmon.update("B".getBytes(), 100.0);

            double l2 = univmon.estimateL2();
            assertTrue(l2 > 0.0);
        }
    }

    @Test
    public void testUnivMonEntropyEstimation() {
        try (UnivMon univmon = new UnivMon(10000, 0.01, 0.01)) {
            // Create uniform distribution
            for (int i = 0; i < 100; i++) {
                univmon.update(("item" + i).getBytes(), 1.0);
            }

            double entropy = univmon.estimateEntropy();
            assertTrue(entropy >= 0.0);
        }
    }

    @Test
    public void testUnivMonChangeDetection() {
        try (UnivMon baseline = new UnivMon(10000, 0.01, 0.01);
             UnivMon current = new UnivMon(10000, 0.01, 0.01)) {

            // Normal traffic
            for (int i = 0; i < 100; i++) {
                baseline.update(("flow" + i).getBytes(), 1.0);
            }

            // Anomalous traffic
            for (int i = 0; i < 100; i++) {
                current.update("attack_flow".getBytes(), 1.0);
            }

            double change = baseline.detectChange(current);
            assertTrue(change > 0.0);
        }
    }

    @Test
    public void testUnivMonMerge() {
        try (UnivMon um1 = new UnivMon(10000, 0.01, 0.01);
             UnivMon um2 = new UnivMon(10000, 0.01, 0.01)) {

            um1.update("item1".getBytes(), 100.0);
            um2.update("item2".getBytes(), 200.0);

            um1.merge(um2);
            assertTrue(um1.totalUpdates() == 2);
        }
    }

    @Test
    public void testUnivMonInvalidStreamSize() {
        assertThrows(IllegalArgumentException.class, () -> {
            new UnivMon(0, 0.01, 0.01);
        });
    }

    @Test
    public void testUnivMonInvalidEpsilon() {
        assertThrows(IllegalArgumentException.class, () -> {
            new UnivMon(10000, 0.0, 0.01);
        });
    }

    @Test
    public void testUnivMonInvalidDelta() {
        assertThrows(IllegalArgumentException.class, () -> {
            new UnivMon(10000, 0.01, 1.0);
        });
    }

    @Test
    public void testUnivMonNegativeValue() {
        try (UnivMon univmon = new UnivMon(10000, 0.01, 0.01)) {
            assertThrows(IllegalArgumentException.class, () -> {
                univmon.update("item".getBytes(), -100.0);
            });
        }
    }

    @Test
    public void testUnivMonNetworkMonitoring() {
        try (UnivMon univmon = new UnivMon(1_000_000, 0.01, 0.01)) {
            // Simulate network packets
            univmon.update("192.168.1.1".getBytes(), 1500.0);
            univmon.update("192.168.1.2".getBytes(), 800.0);
            univmon.update("192.168.1.1".getBytes(), 1200.0);

            double totalBytes = univmon.estimateL1();
            assertTrue(totalBytes > 0.0);
        }
    }

    @Test
    public void testUnivMonMultipleMetrics() {
        try (UnivMon univmon = new UnivMon(10000, 0.01, 0.01)) {
            for (int i = 0; i < 50; i++) {
                univmon.update(("item" + i).getBytes(), i + 1.0);
            }

            double l1 = univmon.estimateL1();
            double l2 = univmon.estimateL2();
            double entropy = univmon.estimateEntropy();

            assertTrue(l1 > 0.0);
            assertTrue(l2 > 0.0);
            assertTrue(entropy >= 0.0);
        }
    }

    @Test
    public void testUnivMonResourceManagement() {
        UnivMon univmon = new UnivMon(10000, 0.01, 0.01);
        univmon.update("test".getBytes(), 1.0);
        univmon.close();
        assertThrows(IllegalStateException.class, () -> {
            univmon.estimateL1();
        });
    }

    @Test
    public void testUnivMonNumLayers() {
        try (UnivMon univmon = new UnivMon(1_000_000, 0.01, 0.01)) {
            int layers = univmon.numLayers();
            // log2(1M) ≈ 20
            assertTrue(layers >= 3 && layers <= 25);
        }
    }

    // ============================================================================
    // LEARNED BLOOM FILTER TESTS (15 tests)
    // ============================================================================

    @Test
    public void testLearnedBloomBasicConstruction() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 100; i++) {
            keys.add(("key" + i).getBytes());
        }
        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            assertNotNull(lbf);
        }
    }

    @Test
    public void testLearnedBloomContains() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 100; i++) {
            keys.add(("key" + i).getBytes());
        }
        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            // All training keys should be found
            for (byte[] key : keys) {
                assertTrue(lbf.contains(key), "False negative for training key");
            }
        }
    }

    @Test
    public void testLearnedBloomInvalidEmptyKeys() {
        assertThrows(IllegalArgumentException.class, () -> {
            new LearnedBloomFilter(new ArrayList<>(), 0.01);
        });
    }

    @Test
    public void testLearnedBloomInvalidFewKeys() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 5; i++) {
            keys.add(("key" + i).getBytes());
        }
        assertThrows(IllegalArgumentException.class, () -> {
            new LearnedBloomFilter(keys, 0.01);
        });
    }

    @Test
    public void testLearnedBloomInvalidFpr() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 100; i++) {
            keys.add(("key" + i).getBytes());
        }
        assertThrows(IllegalArgumentException.class, () -> {
            new LearnedBloomFilter(keys, 0.0);
        });
    }

    @Test
    public void testLearnedBloomMemoryUsage() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 1000; i++) {
            keys.add(("key" + i).getBytes());
        }
        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            long memory = lbf.memoryUsage();
            assertTrue(memory > 0);
        }
    }

    @Test
    public void testLearnedBloomFpr() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 100; i++) {
            keys.add(("key" + i).getBytes());
        }
        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.02)) {
            assertEquals(0.02, lbf.fpr(), 0.001);
        }
    }

    @Test
    public void testLearnedBloomUrlDataset() {
        List<byte[]> urls = new ArrayList<>();
        urls.add("https://example.com/page1".getBytes());
        urls.add("https://example.com/page2".getBytes());
        urls.add("https://github.com/user/repo".getBytes());
        urls.add("https://stackoverflow.com/questions/123".getBytes());
        for (int i = 0; i < 100; i++) {
            urls.add(("https://site" + i + ".com").getBytes());
        }

        try (LearnedBloomFilter lbf = new LearnedBloomFilter(urls, 0.01)) {
            assertTrue(lbf.contains("https://example.com/page1".getBytes()));
        }
    }

    @Test
    public void testLearnedBloomPatternedData() {
        List<byte[]> keys = new ArrayList<>();
        // Create patterned data that should be learnable
        for (int i = 0; i < 200; i++) {
            String key = "user_" + String.format("%05d", i);
            keys.add(key.getBytes());
        }

        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            assertTrue(lbf.contains("user_00050".getBytes()));
        }
    }

    @Test
    public void testLearnedBloomNullKey() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 100; i++) {
            keys.add(("key" + i).getBytes());
        }
        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            assertThrows(NullPointerException.class, () -> {
                lbf.contains(null);
            });
        }
    }

    @Test
    public void testLearnedBloomResourceManagement() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 100; i++) {
            keys.add(("key" + i).getBytes());
        }
        LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01);
        lbf.close();
        assertThrows(IllegalStateException.class, () -> {
            lbf.contains("key0".getBytes());
        });
    }

    @Test
    public void testLearnedBloomNumericKeys() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 500; i++) {
            keys.add(String.valueOf(i * 7).getBytes()); // Pattern: multiples of 7
        }

        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            assertTrue(lbf.contains("0".getBytes()));
            assertTrue(lbf.contains("7".getBytes()));
        }
    }

    @Test
    public void testLearnedBloomLargeDataset() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 10000; i++) {
            keys.add(("item_" + i).getBytes());
        }

        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            assertTrue(lbf.contains("item_5000".getBytes()));
            long memory = lbf.memoryUsage();
            assertTrue(memory > 0);
        }
    }

    @Test
    public void testLearnedBloomMixedDataTypes() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 50; i++) {
            keys.add(("string_" + i).getBytes());
            keys.add(String.valueOf(i).getBytes());
        }

        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            assertTrue(lbf.contains("string_10".getBytes()));
            assertTrue(lbf.contains("25".getBytes()));
        }
    }

    @Test
    public void testLearnedBloomZeroFalseNegatives() {
        List<byte[]> keys = new ArrayList<>();
        for (int i = 0; i < 200; i++) {
            keys.add(("test_" + i).getBytes());
        }

        try (LearnedBloomFilter lbf = new LearnedBloomFilter(keys, 0.01)) {
            // All training keys MUST be found (zero false negatives guarantee)
            int falseNegatives = 0;
            for (byte[] key : keys) {
                if (!lbf.contains(key)) {
                    falseNegatives++;
                }
            }
            assertEquals(0, falseNegatives, "Learned Bloom must have zero false negatives");
        }
    }
}
