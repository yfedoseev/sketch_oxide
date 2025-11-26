package com.sketches_oxide;

import com.sketches_oxide.frequency.CountMinSketch;
import com.sketches_oxide.membership.BloomFilter;

/**
 * Phase 3 FFI Optimization Benchmarks - Java Batch APIs
 *
 * Measures performance improvements from batch API implementations.
 */
public class Phase3Benchmarks {

    public static void benchmarkBloomFilter() {
        System.out.println("\n=== BloomFilter Batch API Benchmarks ===");

        // Generate test data
        byte[][] keys = new byte[1000][];
        for (int i = 0; i < 1000; i++) {
            keys[i] = ("key_" + i).getBytes();
        }

        // Individual inserts
        BloomFilter bf_individual = new BloomFilter(1000, 0.01);
        long start = System.nanoTime();
        for (byte[] key : keys) {
            bf_individual.insert(key);
        }
        long individual_time = System.nanoTime() - start;

        // Batch inserts
        BloomFilter bf_batch = new BloomFilter(1000, 0.01);
        start = System.nanoTime();
        bf_batch.insertBatch(keys);
        long batch_time = System.nanoTime() - start;

        double speedup = (double) individual_time / batch_time;
        System.out.printf("Insert 1000 items:\n");
        System.out.printf("  Individual: %.2f ms\n", individual_time / 1_000_000.0);
        System.out.printf("  Batch:      %.2f ms\n", batch_time / 1_000_000.0);
        System.out.printf("  Speedup:    %.2f x\n", speedup);

        // Batch contains
        start = System.nanoTime();
        for (byte[] key : keys) {
            bf_individual.contains(key);
        }
        individual_time = System.nanoTime() - start;

        start = System.nanoTime();
        bf_batch.containsBatch(keys);
        batch_time = System.nanoTime() - start;

        speedup = (double) individual_time / batch_time;
        System.out.printf("\nContains check 1000 items:\n");
        System.out.printf("  Individual: %.2f ms\n", individual_time / 1_000_000.0);
        System.out.printf("  Batch:      %.2f ms\n", batch_time / 1_000_000.0);
        System.out.printf("  Speedup:    %.2f x\n", speedup);

        bf_individual.close();
        bf_batch.close();
    }

    public static void benchmarkCountMinSketch() {
        System.out.println("\n=== CountMinSketch Batch API Benchmarks ===");

        // Generate test data
        byte[][] items = new byte[1000][];
        for (int i = 0; i < 1000; i++) {
            items[i] = ("item_" + i).getBytes();
        }

        // Individual updates
        CountMinSketch cms_individual = new CountMinSketch(0.01, 0.01);
        long start = System.nanoTime();
        for (byte[] item : items) {
            cms_individual.update(item);
        }
        long individual_time = System.nanoTime() - start;

        // Batch updates
        CountMinSketch cms_batch = new CountMinSketch(0.01, 0.01);
        start = System.nanoTime();
        cms_batch.updateBatch(items);
        long batch_time = System.nanoTime() - start;

        double speedup = (double) individual_time / batch_time;
        System.out.printf("Update 1000 items:\n");
        System.out.printf("  Individual: %.2f ms\n", individual_time / 1_000_000.0);
        System.out.printf("  Batch:      %.2f ms\n", batch_time / 1_000_000.0);
        System.out.printf("  Speedup:    %.2f x\n", speedup);

        // Batch estimate
        start = System.nanoTime();
        for (byte[] item : items) {
            cms_individual.estimate(item);
        }
        individual_time = System.nanoTime() - start;

        start = System.nanoTime();
        cms_batch.estimateBatch(items);
        batch_time = System.nanoTime() - start;

        speedup = (double) individual_time / batch_time;
        System.out.printf("\nEstimate 1000 items:\n");
        System.out.printf("  Individual: %.2f ms\n", individual_time / 1_000_000.0);
        System.out.printf("  Batch:      %.2f ms\n", batch_time / 1_000_000.0);
        System.out.printf("  Speedup:    %.2f x\n", speedup);

        cms_individual.close();
        cms_batch.close();
    }

    public static void main(String[] args) {
        System.out.println("============================================================");
        System.out.println("Phase 3 FFI Optimization Benchmarks - Java");
        System.out.println("============================================================");

        benchmarkBloomFilter();
        benchmarkCountMinSketch();

        System.out.println("\n============================================================");
        System.out.println("Benchmarks Complete");
        System.out.println("============================================================");
    }
}
