/**
 * Performance Comparison - Benchmarking Different Sketches
 *
 * Comprehensive benchmarks comparing memory usage, speed, and accuracy
 * across different sketch types to help choose the right algorithm.
 *
 * Run: npx ts-node 08_performance_comparison.ts
 */

import {
  HyperLogLog,
  CountMinSketch,
  BloomFilter,
  HeavyKeeper,
  GRF,
  UnivMon
} from '..';

console.log('=== Sketch Performance Comparison ===\n');

// ============================================================================
// Benchmark Utilities
// ============================================================================

function benchmark(name: string, fn: () => void, iterations: number): number {
  const start = process.hrtime.bigint();

  for (let i = 0; i < iterations; i++) {
    fn();
  }

  const end = process.hrtime.bigint();
  const totalNs = Number(end - start);
  const avgNs = totalNs / iterations;

  console.log(`  ${name.padEnd(40)} ${avgNs.toFixed(0).padStart(8)} ns/op`);
  return avgNs;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(2)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`;
}

// ============================================================================
// Cardinality Estimation Benchmark
// ============================================================================

console.log('1. CARDINALITY ESTIMATION');
console.log('=========================\n');

const numUniqueItems = 100000;
const items = Array.from({ length: numUniqueItems }, (_, i) =>
  Buffer.from(`item_${i}`)
);

console.log(`Dataset: ${numUniqueItems.toLocaleString()} unique items\n`);

// HyperLogLog variants
console.log('HyperLogLog (different precisions):');
console.log('-----------------------------------');

interface HLLResult {
  precision: number;
  memory: number;
  estimate: number;
  error: number;
  updateTime: number;
  estimateTime: number;
}

const hllResults: HLLResult[] = [];

for (const precision of [10, 12, 14, 16]) {
  const hll = new HyperLogLog(precision);

  // Benchmark update
  const updateTime = benchmark(
    `Update (precision ${precision})`,
    () => hll.update(items[Math.floor(Math.random() * items.length)]),
    10000
  );

  // Insert all items
  for (const item of items) {
    hll.update(item);
  }

  // Benchmark estimate
  const estimateTime = benchmark(
    `Estimate (precision ${precision})`,
    () => hll.estimate(),
    10000
  );

  const estimate = Math.round(hll.estimate());
  const error = Math.abs(estimate - numUniqueItems) / numUniqueItems * 100;
  const memory = Math.pow(2, precision);

  hllResults.push({
    precision,
    memory,
    estimate,
    error,
    updateTime,
    estimateTime
  });

  console.log(`    Memory: ${formatBytes(memory)}, Estimate: ${estimate.toLocaleString()}, Error: ${error.toFixed(2)}%`);
}

console.log('');

// ============================================================================
// Frequency Estimation Benchmark
// ============================================================================

console.log('2. FREQUENCY ESTIMATION');
console.log('=======================\n');

console.log('CountMinSketch (different error rates):');
console.log('---------------------------------------');

interface CMSResult {
  epsilon: number;
  delta: number;
  memory: number;
  updateTime: number;
  estimateTime: number;
  avgError: number;
}

const cmsResults: CMSResult[] = [];

const frequencies = new Map<string, number>();
const frequencyItems: Buffer[] = [];

// Generate Zipf distribution
for (let i = 0; i < 10000; i++) {
  const rank = Math.floor(Math.pow(Math.random(), 2) * 100) + 1;
  const item = `word_${rank}`;
  frequencies.set(item, (frequencies.get(item) || 0) + 1);
  frequencyItems.push(Buffer.from(item));
}

for (const [epsilon, delta] of [[0.1, 0.1], [0.01, 0.01], [0.001, 0.01]]) {
  const cms = new CountMinSketch(epsilon, delta);

  // Benchmark update
  const updateTime = benchmark(
    `Update (ε=${epsilon}, δ=${delta})`,
    () => cms.update(frequencyItems[Math.floor(Math.random() * frequencyItems.length)]),
    10000
  );

  // Insert all items
  for (const item of frequencyItems) {
    cms.update(item);
  }

  // Benchmark estimate
  const estimateTime = benchmark(
    `Estimate (ε=${epsilon}, δ=${delta})`,
    () => cms.estimate(Buffer.from('word_1')),
    10000
  );

  // Calculate average error
  let totalError = 0;
  let count = 0;

  for (const [word, actualFreq] of frequencies.entries()) {
    const estimated = cms.estimate(Buffer.from(word));
    totalError += Math.abs(estimated - actualFreq) / actualFreq;
    count++;
  }

  const avgError = (totalError / count * 100);

  // Estimate memory (rough calculation)
  const width = Math.ceil(Math.E / epsilon);
  const depth = Math.ceil(Math.log(1 / delta));
  const memory = width * depth * 8; // 8 bytes per counter

  cmsResults.push({
    epsilon,
    delta,
    memory,
    updateTime,
    estimateTime,
    avgError
  });

  console.log(`    Memory: ~${formatBytes(memory)}, Avg Error: ${avgError.toFixed(2)}%`);
}

console.log('');

// ============================================================================
// Membership Testing Benchmark
// ============================================================================

console.log('3. MEMBERSHIP TESTING');
console.log('=====================\n');

console.log('BloomFilter (different FPRs):');
console.log('----------------------------');

interface BFResult {
  fpr: number;
  memory: number;
  insertTime: number;
  containsTime: number;
  actualFPR: number;
}

const bfResults: BFResult[] = [];
const membershipItems = items.slice(0, 10000);
const nonMembers = Array.from({ length: 10000 }, (_, i) =>
  Buffer.from(`non_member_${i}`)
);

for (const targetFPR of [0.1, 0.01, 0.001]) {
  const bf = new BloomFilter(10000, targetFPR);

  // Benchmark insert
  const insertTime = benchmark(
    `Insert (FPR ${targetFPR})`,
    () => bf.insert(membershipItems[Math.floor(Math.random() * membershipItems.length)]),
    10000
  );

  // Insert all members
  for (const item of membershipItems) {
    bf.insert(item);
  }

  // Benchmark contains
  const containsTime = benchmark(
    `Contains (FPR ${targetFPR})`,
    () => bf.contains(membershipItems[Math.floor(Math.random() * membershipItems.length)]),
    10000
  );

  // Measure actual FPR
  let falsePositives = 0;
  for (const item of nonMembers) {
    if (bf.contains(item)) {
      falsePositives++;
    }
  }
  const actualFPR = falsePositives / nonMembers.length;

  const memory = bf.memoryUsage();

  bfResults.push({
    fpr: targetFPR,
    memory,
    insertTime,
    containsTime,
    actualFPR
  });

  console.log(`    Memory: ${formatBytes(memory)}, Actual FPR: ${(actualFPR * 100).toFixed(3)}%`);
}

console.log('');

// ============================================================================
// Heavy Hitter Detection Benchmark
// ============================================================================

console.log('4. HEAVY HITTER DETECTION');
console.log('=========================\n');

console.log('HeavyKeeper:');
console.log('-----------');

const hk = new HeavyKeeper(100, 0.001, 0.01);

// Generate stream with heavy hitters
const stream: Buffer[] = [];
for (let i = 0; i < 10000; i++) {
  if (i < 5000) {
    // 50% from top-10 items
    stream.push(Buffer.from(`heavy_${i % 10}`));
  } else {
    // 50% from many items
    stream.push(Buffer.from(`item_${i}`));
  }
}

const hkUpdateTime = benchmark(
  'Update',
  () => hk.update(stream[Math.floor(Math.random() * stream.length)]),
  10000
);

for (const item of stream) {
  hk.update(item);
}

const hkTopKTime = benchmark('Get Top-K', () => hk.topK(), 10000);
const hkEstimateTime = benchmark('Estimate', () => hk.estimate(Buffer.from('heavy_1')), 10000);

console.log(`    Update: ${hkUpdateTime.toFixed(0)} ns/op`);
console.log(`    Top-K: ${hkTopKTime.toFixed(0)} ns/op`);
console.log(`    Estimate: ${hkEstimateTime.toFixed(0)} ns/op`);
console.log(`    Top hitters found: ${hk.topK().length}\n`);

// ============================================================================
// Range Filter Benchmark
// ============================================================================

console.log('5. RANGE FILTERING');
console.log('==================\n');

console.log('GRF:');
console.log('---');

const rangeKeys = Array.from({ length: 10000 }, (_, i) => BigInt(i * 100));
const grf = GRF.build(rangeKeys, 6);

const grfBuildTime = (() => {
  const start = process.hrtime.bigint();
  GRF.build(rangeKeys, 6);
  const end = process.hrtime.bigint();
  return Number(end - start) / 1000000; // ms
})();

const grfQueryTime = benchmark(
  'Query Range',
  () => grf.mayContainRange(5000n, 6000n),
  10000
);

const grfPointTime = benchmark(
  'Query Point',
  () => grf.mayContain(5000n),
  10000
);

const grfStats = grf.stats();

console.log(`    Build: ${grfBuildTime.toFixed(2)} ms for 10,000 keys`);
console.log(`    Query Range: ${grfQueryTime.toFixed(0)} ns/op`);
console.log(`    Query Point: ${grfPointTime.toFixed(0)} ns/op`);
console.log(`    Memory: ${formatBytes(grfStats.memoryBytes)}\n`);

// ============================================================================
// Universal Monitoring Benchmark
// ============================================================================

console.log('6. UNIVERSAL MONITORING');
console.log('=======================\n');

console.log('UnivMon:');
console.log('-------');

const univmon = new UnivMon(100000n, 0.01, 0.01);

const univmonUpdateTime = benchmark(
  'Update',
  () => univmon.update(Buffer.from('test'), Math.random() * 1000),
  10000
);

// Add data
for (let i = 0; i < 10000; i++) {
  univmon.update(Buffer.from(`item_${i % 100}`), Math.random() * 1000);
}

const univmonL1Time = benchmark('Estimate L1', () => univmon.estimateL1(), 1000);
const univmonL2Time = benchmark('Estimate L2', () => univmon.estimateL2(), 1000);
const univmonEntropyTime = benchmark('Estimate Entropy', () => univmon.estimateEntropy(), 1000);
const univmonHHTime = benchmark('Heavy Hitters', () => univmon.heavyHitters(0.1), 1000);

console.log(`    Update: ${univmonUpdateTime.toFixed(0)} ns/op`);
console.log(`    L1 Norm: ${univmonL1Time.toFixed(0)} ns/op`);
console.log(`    L2 Norm: ${univmonL2Time.toFixed(0)} ns/op`);
console.log(`    Entropy: ${univmonEntropyTime.toFixed(0)} ns/op`);
console.log(`    Heavy Hitters: ${univmonHHTime.toFixed(0)} ns/op\n`);

// ============================================================================
// Summary Table
// ============================================================================

console.log('=== SUMMARY ===\n');

console.log('Cardinality (HyperLogLog):');
console.log('-------------------------');
console.log('Precision | Memory  | Error   | Update Time');
console.log('----------|---------|---------|------------');
for (const result of hllResults) {
  console.log(`${result.precision.toString().padStart(9)} | ${formatBytes(result.memory).padEnd(7)} | ${result.error.toFixed(2)}% | ${result.updateTime.toFixed(0).padStart(6)} ns`);
}
console.log('');

console.log('Frequency (CountMinSketch):');
console.log('--------------------------');
console.log('ε      | Memory  | Avg Error | Update Time');
console.log('-------|---------|-----------|------------');
for (const result of cmsResults) {
  console.log(`${result.epsilon.toString().padEnd(6)} | ${formatBytes(result.memory).padEnd(7)} | ${result.avgError.toFixed(2).padStart(7)}% | ${result.updateTime.toFixed(0).padStart(6)} ns`);
}
console.log('');

console.log('Membership (BloomFilter):');
console.log('------------------------');
console.log('Target FPR | Memory   | Actual FPR | Insert Time');
console.log('-----------|----------|------------|------------');
for (const result of bfResults) {
  console.log(`${(result.fpr * 100).toFixed(1).padStart(8)}% | ${formatBytes(result.memory).padEnd(8)} | ${(result.actualFPR * 100).toFixed(3).padStart(8)}% | ${result.insertTime.toFixed(0).padStart(6)} ns`);
}
console.log('');

console.log('Recommendations:');
console.log('---------------');
console.log('');
console.log('For Cardinality Estimation:');
console.log('  • Precision 14: Best balance (16 KB, ~0.8% error)');
console.log('  • Precision 12: Low memory (4 KB, ~1.6% error)');
console.log('  • Precision 16: High accuracy (64 KB, ~0.4% error)\n');

console.log('For Frequency Estimation:');
console.log('  • ε=0.01: Recommended (moderate memory, <2% error)');
console.log('  • ε=0.001: High accuracy (more memory, <0.2% error)');
console.log('  • Use HeavyKeeper if you only need top-K\n');

console.log('For Membership Testing:');
console.log('  • FPR=0.01: Most use cases (1% false positives)');
console.log('  • FPR=0.001: High precision needed');
console.log('  • Use VacuumFilter if deletions required\n');

console.log('For Complex Monitoring:');
console.log('  • Use UnivMon for multiple metrics from one sketch');
console.log('  • 6x memory savings vs separate sketches');
console.log('  • Ideal for network/infrastructure monitoring\n');

console.log('=== Benchmark Complete ===\n');

console.log('Note: Run with NODE_OPTIONS="--max-old-space-size=4096" for larger datasets');
console.log('Actual performance depends on CPU, data distribution, and workload patterns\n');
