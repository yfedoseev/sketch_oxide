/**
 * Basic Usage Example - BloomFilter and HyperLogLog
 *
 * Demonstrates the two most commonly used sketches for simple use cases.
 *
 * Compile: javac -cp "../target/*" BasicUsage.java
 * Run: java -cp ".:../target/*" BasicUsage
 */

import com.sketches_oxide.cardinality.HyperLogLog;
import com.sketches_oxide.membership.BloomFilter;

import java.nio.charset.StandardCharsets;
import java.util.*;

public class BasicUsage {

    public static void main(String[] args) {
        System.out.println("=== Basic Usage Example ===\n");

        example1_BloomFilter();
        example2_HyperLogLog();
        example3_Merging();
        example4_Serialization();

        System.out.println("=== Example Complete ===\n");
    }

    /**
     * Example 1: BloomFilter - Email Deduplication
     */
    private static void example1_BloomFilter() {
        System.out.println("1. BloomFilter - Email Deduplication");
        System.out.println("-------------------------------------\n");

        // Create Bloom filter for 100,000 emails with 0.1% false positive rate
        try (BloomFilter emailFilter = new BloomFilter(100000, 0.001)) {

            String[] emails = {
                "user@example.com",
                "admin@company.org",
                "user@example.com",  // Duplicate
                "contact@service.net",
                "admin@company.org",  // Duplicate
                "new.user@example.com"
            };

            int processed = 0;
            int duplicates = 0;

            System.out.println("Processing emails:");
            for (String email : emails) {
                byte[] key = email.getBytes(StandardCharsets.UTF_8);

                if (emailFilter.contains(key)) {
                    System.out.println("  ✗ DUPLICATE: " + email);
                    duplicates++;
                } else {
                    System.out.println("  ✓ NEW: " + email);
                    emailFilter.insert(key);
                    processed++;
                }
            }

            System.out.printf("\nResults: %d unique, %d duplicates\n", processed, duplicates);
            System.out.printf("Memory usage: ~%d KB\n\n", emailFilter.memoryUsage() / 1024);
        }
    }

    /**
     * Example 2: HyperLogLog - Website Analytics
     */
    private static void example2_HyperLogLog() {
        System.out.println("2. HyperLogLog - Unique Visitor Counting");
        System.out.println("----------------------------------------\n");

        try (HyperLogLog visitorCounter = new HyperLogLog(14)) {

            // Simulate tracking visitors
            String[][] visits = {
                {"192.168.1.100", "/home"},
                {"192.168.1.101", "/home"},
                {"192.168.1.100", "/about"},  // Same visitor
                {"192.168.1.102", "/home"},
                {"192.168.1.101", "/contact"}, // Same visitor
                {"192.168.1.103", "/home"},
                {"192.168.1.104", "/products"},
                {"192.168.1.100", "/products"} // Same visitor
            };

            System.out.println("Tracking visits:");
            for (String[] visit : visits) {
                visitorCounter.update(visit[0].getBytes(StandardCharsets.UTF_8));
                System.out.printf("  Visit: %s -> %s\n", visit[0], visit[1]);
            }

            long uniqueVisitors = Math.round(visitorCounter.estimate());
            System.out.printf("\nTotal visits: %d\n", visits.length);
            System.out.printf("Unique visitors: %d\n", uniqueVisitors);
            System.out.printf("Actual unique IPs: 5\n");
            System.out.printf("Error: %d (%.1f%%)\n\n",
                Math.abs(uniqueVisitors - 5),
                Math.abs(uniqueVisitors - 5) / 5.0 * 100);

            // Show precision parameter effect
            System.out.println("Precision vs Memory tradeoff:");
            for (int precision : new int[]{10, 12, 14, 16}) {
                try (HyperLogLog hll = new HyperLogLog(precision)) {
                    double memory = Math.pow(2, precision) / 1024.0;
                    double error = (1.04 / Math.sqrt(Math.pow(2, precision)) * 100);
                    System.out.printf("  Precision %d: %.1f KB, ~%.2f%% error\n",
                        precision, memory, error);
                }
            }
            System.out.println();
        }
    }

    /**
     * Example 3: Merging HyperLogLogs from Distributed Sources
     */
    private static void example3_Merging() {
        System.out.println("3. Merging HyperLogLogs from Distributed Sources");
        System.out.println("------------------------------------------------\n");

        try (HyperLogLog server1 = new HyperLogLog(14);
             HyperLogLog server2 = new HyperLogLog(14);
             HyperLogLog server3 = new HyperLogLog(14)) {

            // Server 1 sees visitors 1-5
            for (int i = 1; i <= 5; i++) {
                server1.update(("visitor_" + i).getBytes(StandardCharsets.UTF_8));
            }

            // Server 2 sees visitors 4-8 (overlap)
            for (int i = 4; i <= 8; i++) {
                server2.update(("visitor_" + i).getBytes(StandardCharsets.UTF_8));
            }

            // Server 3 sees visitors 7-12 (overlap)
            for (int i = 7; i <= 12; i++) {
                server3.update(("visitor_" + i).getBytes(StandardCharsets.UTF_8));
            }

            System.out.printf("Server 1 unique visitors: %d\n", Math.round(server1.estimate()));
            System.out.printf("Server 2 unique visitors: %d\n", Math.round(server2.estimate()));
            System.out.printf("Server 3 unique visitors: %d\n", Math.round(server3.estimate()));

            // Merge all servers into server1
            server1.merge(server2);
            server1.merge(server3);

            long totalUnique = Math.round(server1.estimate());
            System.out.printf("\nTotal unique visitors (merged): %d\n", totalUnique);
            System.out.printf("Actual unique visitors: 12\n\n");
        }
    }

    /**
     * Example 4: Serialization for Persistence
     */
    private static void example4_Serialization() {
        System.out.println("4. Serialization - Save and Restore State");
        System.out.println("----------------------------------------\n");

        byte[] serialized;
        double originalEstimate;

        // Create and populate a sketch
        try (HyperLogLog original = new HyperLogLog(12)) {
            for (int i = 0; i < 1000; i++) {
                original.update(("item_" + i).getBytes(StandardCharsets.UTF_8));
            }

            originalEstimate = original.estimate();
            System.out.printf("Original estimate: %d\n", Math.round(originalEstimate));

            // Serialize
            serialized = original.serialize();
            System.out.printf("Serialized size: %d bytes\n", serialized.length);
        }

        // Deserialize to new sketch
        try (HyperLogLog restored = HyperLogLog.deserialize(serialized)) {
            double restoredEstimate = restored.estimate();
            System.out.printf("Restored estimate: %d\n", Math.round(restoredEstimate));
            System.out.printf("Match: %s\n\n", originalEstimate == restoredEstimate ? "✓" : "✗");
        }
    }
}
