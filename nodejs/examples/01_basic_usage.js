/**
 * Basic Usage Example - BloomFilter and HyperLogLog
 *
 * Demonstrates the two most commonly used sketches for simple use cases:
 * - BloomFilter: Fast membership testing ("have we seen this item?")
 * - HyperLogLog: Count unique items with minimal memory
 *
 * Run: node 01_basic_usage.js
 */

const { BloomFilter, HyperLogLog } = require('..');

console.log('=== Basic Usage Example ===\n');

// ============================================================================
// Example 1: BloomFilter - Deduplication in Email Processing
// ============================================================================
console.log('1. BloomFilter - Email Deduplication');
console.log('-------------------------------------');

// Create a Bloom filter for 100,000 emails with 0.1% false positive rate
const emailFilter = new BloomFilter(100000, 0.001);

// Simulate processing incoming emails
const emails = [
  'user@example.com',
  'admin@company.org',
  'user@example.com',  // Duplicate
  'contact@service.net',
  'admin@company.org',  // Duplicate
  'new.user@example.com'
];

let processed = 0;
let duplicates = 0;

console.log('Processing emails:');
for (const email of emails) {
  const key = Buffer.from(email);

  if (emailFilter.contains(key)) {
    console.log(`  ✗ DUPLICATE: ${email}`);
    duplicates++;
  } else {
    console.log(`  ✓ NEW: ${email}`);
    emailFilter.insert(key);
    processed++;
  }
}

console.log(`\nResults: ${processed} unique, ${duplicates} duplicates`);
console.log(`Memory usage: ~${Math.round(emailFilter.memoryUsage() / 1024)} KB\n`);

// ============================================================================
// Example 2: HyperLogLog - Website Analytics
// ============================================================================
console.log('2. HyperLogLog - Unique Visitor Counting');
console.log('----------------------------------------');

// Create HyperLogLog with precision 14 (~0.8% error, 16KB memory)
const visitorCounter = new HyperLogLog(14);

// Simulate tracking visitors across multiple pages
const visits = [
  { ip: '192.168.1.100', page: '/home' },
  { ip: '192.168.1.101', page: '/home' },
  { ip: '192.168.1.100', page: '/about' },  // Same visitor
  { ip: '192.168.1.102', page: '/home' },
  { ip: '192.168.1.101', page: '/contact' }, // Same visitor
  { ip: '192.168.1.103', page: '/home' },
  { ip: '192.168.1.104', page: '/products' },
  { ip: '192.168.1.100', page: '/products' }, // Same visitor
];

console.log('Tracking visits:');
for (const visit of visits) {
  visitorCounter.update(Buffer.from(visit.ip));
  console.log(`  Visit: ${visit.ip} -> ${visit.page}`);
}

const uniqueVisitors = Math.round(visitorCounter.estimate());
console.log(`\nTotal visits: ${visits.length}`);
console.log(`Unique visitors: ${uniqueVisitors}`);
console.log(`Actual unique IPs: 5`);
console.log(`Error: ${Math.abs(uniqueVisitors - 5)} (${(Math.abs(uniqueVisitors - 5) / 5 * 100).toFixed(1)}%)`);

// Show precision parameter effect
console.log('\nPrecision vs Memory tradeoff:');
for (const precision of [10, 12, 14, 16]) {
  const hll = new HyperLogLog(precision);
  const memory = Math.pow(2, precision) / 1024;
  const error = (1.04 / Math.sqrt(Math.pow(2, precision)) * 100).toFixed(2);
  console.log(`  Precision ${precision}: ${memory.toFixed(1)} KB, ~${error}% error`);
}

// ============================================================================
// Example 3: Merging Sketches from Multiple Sources
// ============================================================================
console.log('\n3. Merging HyperLogLogs from Distributed Sources');
console.log('------------------------------------------------');

// Simulate 3 different web servers tracking visitors
const server1 = new HyperLogLog(14);
const server2 = new HyperLogLog(14);
const server3 = new HyperLogLog(14);

// Server 1 sees visitors 1-5
for (let i = 1; i <= 5; i++) {
  server1.update(Buffer.from(`visitor_${i}`));
}

// Server 2 sees visitors 4-8 (overlap with server1)
for (let i = 4; i <= 8; i++) {
  server2.update(Buffer.from(`visitor_${i}`));
}

// Server 3 sees visitors 7-12 (overlap with server2)
for (let i = 7; i <= 12; i++) {
  server3.update(Buffer.from(`visitor_${i}`));
}

console.log(`Server 1 unique visitors: ${Math.round(server1.estimate())}`);
console.log(`Server 2 unique visitors: ${Math.round(server2.estimate())}`);
console.log(`Server 3 unique visitors: ${Math.round(server3.estimate())}`);

// Merge all servers into server1
server1.merge(server2);
server1.merge(server3);

const totalUnique = Math.round(server1.estimate());
console.log(`\nTotal unique visitors (merged): ${totalUnique}`);
console.log(`Actual unique visitors: 12`);

// ============================================================================
// Example 4: Serialization for Persistence
// ============================================================================
console.log('\n4. Serialization - Save and Restore State');
console.log('----------------------------------------');

// Create and populate a sketch
const original = new HyperLogLog(12);
for (let i = 0; i < 1000; i++) {
  original.update(Buffer.from(`item_${i}`));
}

const originalEstimate = original.estimate();
console.log(`Original estimate: ${Math.round(originalEstimate)}`);

// Serialize to binary
const serialized = original.serialize();
console.log(`Serialized size: ${serialized.length} bytes`);

// Deserialize to new sketch
const restored = HyperLogLog.deserialize(serialized);
const restoredEstimate = restored.estimate();
console.log(`Restored estimate: ${Math.round(restoredEstimate)}`);
console.log(`Match: ${originalEstimate === restoredEstimate ? '✓' : '✗'}`);

// ============================================================================
// Example 5: Error Handling
// ============================================================================
console.log('\n5. Error Handling');
console.log('-----------------');

try {
  // Invalid precision (must be 4-16)
  const invalid = new HyperLogLog(20);
} catch (error) {
  console.log('✓ Caught invalid precision error');
}

try {
  // Merging HLLs with different precisions
  const hll1 = new HyperLogLog(12);
  const hll2 = new HyperLogLog(14);
  hll1.merge(hll2);
} catch (error) {
  console.log('✓ Caught precision mismatch error');
}

console.log('\n=== Example Complete ===\n');
console.log('Key Takeaways:');
console.log('1. BloomFilter: Fast membership testing, configurable false positive rate');
console.log('2. HyperLogLog: Count unique items with <1% error using tiny memory');
console.log('3. Both support serialization for persistence and distributed computing');
console.log('4. Choose precision/parameters based on memory vs accuracy tradeoff\n');
