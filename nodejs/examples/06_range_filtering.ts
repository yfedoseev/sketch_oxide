/**
 * Range Filtering with GRF (Gorilla Range Filter)
 *
 * Demonstrates using GRF for efficient range queries in databases and LSM-trees.
 * GRF provides 30-50% better false positive rates than alternatives on real data
 * by using shape-based encoding optimized for skewed distributions.
 *
 * Run: npx ts-node 06_range_filtering.ts
 */

import { GRF, Grafite } from '..';

console.log('=== Range Filtering for Database Optimization ===\n');

// ============================================================================
// Example 1: LSM-Tree Range Query Optimization
// ============================================================================

console.log('Example 1: LSM-Tree SSTable Filtering');
console.log('=====================================\n');

interface SSTable {
  id: number;
  minKey: bigint;
  maxKey: bigint;
  keys: bigint[];
  filter: GRF | null;
}

class LSMTree {
  private sstables: SSTable[] = [];
  private bitsPerKey: number = 6;

  addSSTable(keys: bigint[]): void {
    const sortedKeys = keys.sort((a, b) => Number(a - b));
    const minKey = sortedKeys[0];
    const maxKey = sortedKeys[sortedKeys.length - 1];

    // Build range filter
    const filter = GRF.build(sortedKeys, this.bitsPerKey);

    this.sstables.push({
      id: this.sstables.length,
      minKey,
      maxKey,
      keys: sortedKeys,
      filter
    });
  }

  rangeQuery(low: bigint, high: bigint): bigint[] {
    const results: bigint[] = [];
    let tablesScanned = 0;
    let tablesSkipped = 0;

    for (const sstable of this.sstables) {
      // First check: coarse range bounds
      if (high < sstable.minKey || low > sstable.maxKey) {
        tablesSkipped++;
        continue;
      }

      // Second check: GRF filter
      if (!sstable.filter!.mayContainRange(low, high)) {
        tablesSkipped++;
        continue;
      }

      // Filter says "maybe" - must scan the table
      tablesScanned++;
      for (const key of sstable.keys) {
        if (key >= low && key <= high) {
          results.push(key);
        }
      }
    }

    console.log(`  Range [${low}, ${high}]: scanned ${tablesScanned}/${this.sstables.length} tables, skipped ${tablesSkipped}`);
    return results;
  }

  stats(): void {
    console.log(`\nLSM-Tree Statistics:`);
    console.log(`  Total SSTables: ${this.sstables.length}`);
    console.log(`  Bits per key: ${this.bitsPerKey}`);

    let totalKeys = 0;
    let totalFilterSize = 0;

    for (const sstable of this.sstables) {
      totalKeys += sstable.keys.length;
      const stats = sstable.filter!.stats();
      totalFilterSize += stats.memoryBytes;
    }

    console.log(`  Total keys: ${totalKeys.toLocaleString()}`);
    console.log(`  Total filter size: ${(totalFilterSize / 1024).toFixed(2)} KB`);
    console.log(`  Bytes per key: ${(totalFilterSize / totalKeys).toFixed(2)}`);
  }
}

// Create LSM-Tree with multiple levels
console.log('Building LSM-Tree with 10 SSTables...');
const lsmTree = new LSMTree();

// Generate SSTables with realistic timestamp-like keys
for (let i = 0; i < 10; i++) {
  const keys: bigint[] = [];
  const baseTime = BigInt(1000000 * i);

  // Each SSTable has 1000 keys with some clustering
  for (let j = 0; j < 1000; j++) {
    const offset = BigInt(Math.floor(Math.random() * 100000));
    keys.push(baseTime + offset);
  }

  lsmTree.addSSTable(keys);
}

lsmTree.stats();

// Perform range queries
console.log('\nPerforming range queries:');
lsmTree.rangeQuery(500000n, 550000n);     // Small range, single table
lsmTree.rangeQuery(1000000n, 2000000n);   // Medium range, multiple tables
lsmTree.rangeQuery(5000000n, 5100000n);   // Small range, mid-tree
lsmTree.rangeQuery(9000000n, 9900000n);   // Large range, end of tree

// ============================================================================
// Example 2: Time-Series Database Query Optimization
// ============================================================================

console.log('\n\nExample 2: Time-Series Database');
console.log('================================\n');

interface Metric {
  timestamp: bigint;
  value: number;
}

class TimeSeriesDB {
  private chunks: Array<{ timestamps: bigint[]; filter: GRF }> = [];
  private readonly chunkSize = 1000;

  ingest(metrics: Metric[]): void {
    // Sort by timestamp
    metrics.sort((a, b) => Number(a.timestamp - b.timestamp));

    // Split into chunks
    for (let i = 0; i < metrics.length; i += this.chunkSize) {
      const chunk = metrics.slice(i, i + this.chunkSize);
      const timestamps = chunk.map(m => m.timestamp);

      // Build GRF for this chunk
      const filter = GRF.build(timestamps, 8);

      this.chunks.push({ timestamps, filter });
    }
  }

  queryRange(startTime: bigint, endTime: bigint): number {
    let chunksRead = 0;
    let dataPoints = 0;

    for (const chunk of this.chunks) {
      if (chunk.filter.mayContainRange(startTime, endTime)) {
        chunksRead++;
        // Count matching points
        for (const ts of chunk.timestamps) {
          if (ts >= startTime && ts <= endTime) {
            dataPoints++;
          }
        }
      }
    }

    console.log(`  Query [${startTime}, ${endTime}]: read ${chunksRead}/${this.chunks.length} chunks, found ${dataPoints} points`);
    return dataPoints;
  }
}

// Generate time-series data (24 hours of metrics at 1-second resolution)
console.log('Ingesting 24 hours of metrics (86,400 points)...');
const metrics: Metric[] = [];
const baseTimestamp = 1700000000n; // Unix timestamp

for (let i = 0; i < 86400; i++) {
  metrics.push({
    timestamp: baseTimestamp + BigInt(i),
    value: Math.random() * 100
  });
}

const tsdb = new TimeSeriesDB();
tsdb.ingest(metrics);

console.log(`✓ Created ${86400 / 1000} chunks\n`);

// Query different time ranges
console.log('Querying time ranges:');
tsdb.queryRange(baseTimestamp + 3600n, baseTimestamp + 7200n);      // 1 hour window
tsdb.queryRange(baseTimestamp + 43200n, baseTimestamp + 46800n);    // 1 hour midday
tsdb.queryRange(baseTimestamp, baseTimestamp + 86400n);             // Full day
tsdb.queryRange(baseTimestamp + 10000n, baseTimestamp + 10100n);    // 100 seconds

// ============================================================================
// Example 3: Financial Market Data Queries
// ============================================================================

console.log('\n\nExample 3: Financial Market Data');
console.log('=================================\n');

interface Trade {
  timestamp: bigint;
  price: number;
  volume: number;
}

class MarketDataStore {
  private partitions: Map<string, { trades: bigint[]; filter: GRF }> = new Map();

  addSymbol(symbol: string, trades: Trade[]): void {
    const timestamps = trades.map(t => t.timestamp).sort((a, b) => Number(a - b));
    const filter = GRF.build(timestamps, 6);

    this.partitions.set(symbol, { trades: timestamps, filter });
  }

  querySymbol(symbol: string, startTime: bigint, endTime: bigint): { found: boolean; count: number } {
    const partition = this.partitions.get(symbol);
    if (!partition) {
      return { found: false, count: 0 };
    }

    // Use GRF to quickly check if range might have data
    if (!partition.filter.mayContainRange(startTime, endTime)) {
      return { found: false, count: 0 };
    }

    // Count trades in range
    const count = partition.trades.filter(ts => ts >= startTime && ts <= endTime).length;
    return { found: true, count };
  }
}

console.log('Loading market data for 5 symbols...');
const marketData = new MarketDataStore();

const symbols = ['AAPL', 'GOOGL', 'MSFT', 'AMZN', 'TSLA'];
const marketOpen = 1700050800n; // 9:30 AM
const marketClose = 1700074800n; // 4:00 PM (6.5 hours = 23,400 seconds)

for (const symbol of symbols) {
  const trades: Trade[] = [];
  // Generate trades throughout the day
  let currentTime = marketOpen;

  while (currentTime < marketClose) {
    trades.push({
      timestamp: currentTime,
      price: 100 + Math.random() * 50,
      volume: Math.floor(Math.random() * 1000)
    });
    // Random interval between trades (1-60 seconds)
    currentTime += BigInt(Math.floor(Math.random() * 60) + 1);
  }

  marketData.addSymbol(symbol, trades);
  console.log(`  ${symbol}: ${trades.length} trades`);
}

console.log('\nQuerying specific time windows:');

// Morning trades (9:30-10:30)
console.log('\n9:30-10:30 AM window:');
for (const symbol of symbols) {
  const result = marketData.querySymbol(symbol, marketOpen, marketOpen + 3600n);
  if (result.found) {
    console.log(`  ${symbol}: ${result.count} trades`);
  }
}

// Afternoon trades (2:00-3:00 PM)
console.log('\n2:00-3:00 PM window:');
const afternoonStart = marketOpen + 16200n; // 4.5 hours later
for (const symbol of symbols) {
  const result = marketData.querySymbol(symbol, afternoonStart, afternoonStart + 3600n);
  if (result.found) {
    console.log(`  ${symbol}: ${result.count} trades`);
  }
}

// ============================================================================
// Comparison: GRF vs Grafite
// ============================================================================

console.log('\n\n=== GRF vs Grafite Comparison ===\n');

// Generate skewed distribution (Zipf-like, common in real data)
const zipfKeys: bigint[] = [];
for (let i = 1; i <= 10000; i++) {
  // Zipf distribution: frequency proportional to 1/rank
  const key = BigInt(Math.floor(1000000 / i));
  zipfKeys.push(key);
}
zipfKeys.sort((a, b) => Number(a - b));

// Build both filters
const grf = GRF.build(zipfKeys, 6);
const grafite = Grafite.build(zipfKeys, 6);

console.log('Testing on Zipf-distributed keys (10,000 keys):');
console.log('\nFilter Statistics:');

const grfStats = grf.stats();
console.log(`\nGRF:`);
console.log(`  Segments: ${grfStats.segmentCount}`);
console.log(`  Keys per segment: ${grfStats.avgKeysPerSegment.toFixed(1)}`);
console.log(`  Memory: ${grfStats.memoryBytes} bytes`);

const grafiteStats = grafite.stats();
console.log(`\nGrafite:`);
console.log(`  Keys: ${grafiteStats.keyCount}`);
console.log(`  Memory: ${Number(grafiteStats.totalBits) / 8} bytes`);

// Test query performance
console.log('\nQuery Performance (100 random ranges):');
let grfFalsePositives = 0;
let grafiteFalsePositives = 0;

const keySet = new Set(zipfKeys.map(k => k.toString()));

for (let i = 0; i < 100; i++) {
  const low = BigInt(Math.floor(Math.random() * 1000000));
  const high = low + BigInt(Math.floor(Math.random() * 1000) + 1);

  // Check actual presence
  let actuallyPresent = false;
  for (const key of zipfKeys) {
    if (key >= low && key <= high) {
      actuallyPresent = true;
      break;
    }
  }

  const grfSays = grf.mayContainRange(low, high);
  const grafiteSays = grafite.mayContainRange(low, high);

  if (!actuallyPresent && grfSays) grfFalsePositives++;
  if (!actuallyPresent && grafiteSays) grafiteFalsePositives++;
}

console.log(`  GRF false positives: ${grfFalsePositives}/100`);
console.log(`  Grafite false positives: ${grafiteFalsePositives}/100`);
console.log(`  GRF improvement: ${((1 - grfFalsePositives / grafiteFalsePositives) * 100).toFixed(1)}%`);

// ============================================================================
// Summary
// ============================================================================

console.log('\n\n=== Summary ===\n');

console.log('GRF Benefits:');
console.log('  ✓ 30-50% better FPR than Grafite on skewed data');
console.log('  ✓ Shape-based encoding optimizes for real distributions');
console.log('  ✓ Ideal for LSM-trees and time-series databases');
console.log('  ✓ Comparable memory to other range filters\n');

console.log('Performance Characteristics:');
console.log('  Build: O(n log n) - one-time cost');
console.log('  Query: O(log n) - binary search + segment checks');
console.log('  Space: B bits per key (configurable)\n');

console.log('Production Use Cases:');
console.log('  • RocksDB/LevelDB SSTable filtering');
console.log('  • InfluxDB/TimescaleDB time-range queries');
console.log('  • Financial market data retrieval');
console.log('  • Log aggregation systems');
console.log('  • Distributed tracing backends\n');

console.log('Best Practices:');
console.log('  1. Use 6-8 bits per key for most workloads');
console.log('  2. Prefer GRF over Grafite for skewed distributions');
console.log('  3. Combine with coarse bounds checking');
console.log('  4. Monitor FPR and adjust bits per key');
console.log('  5. Rebuild filters periodically for compacted data\n');

console.log('=== Example Complete ===\n');
