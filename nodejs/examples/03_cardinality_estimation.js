/**
 * Cardinality Estimation - Real-Time Analytics Dashboard
 *
 * Demonstrates HyperLogLog for tracking unique visitors across multiple
 * dimensions in a real-time analytics system (similar to Google Analytics).
 *
 * Tracks:
 * - Unique visitors per page
 * - Unique visitors per geographic region
 * - Unique visitors per device type
 * - Global unique visitors across all dimensions
 *
 * Run: node 03_cardinality_estimation.js
 */

const { HyperLogLog } = require('..');

console.log('=== Real-Time Analytics Dashboard ===\n');

// ============================================================================
// Analytics Tracking System
// ============================================================================

class AnalyticsTracker {
  constructor(precision = 14) {
    this.precision = precision;

    // Global unique visitors
    this.globalVisitors = new HyperLogLog(precision);

    // Dimension: Pages
    this.pageVisitors = new Map();

    // Dimension: Geographic regions
    this.regionVisitors = new Map();

    // Dimension: Device types
    this.deviceVisitors = new Map();

    // Dimension: Time windows (hourly)
    this.hourlyVisitors = new Map();
  }

  trackVisit(event) {
    const { userId, page, region, device, timestamp } = event;
    const userKey = Buffer.from(userId);

    // Track global
    this.globalVisitors.update(userKey);

    // Track by page
    if (!this.pageVisitors.has(page)) {
      this.pageVisitors.set(page, new HyperLogLog(this.precision));
    }
    this.pageVisitors.get(page).update(userKey);

    // Track by region
    if (!this.regionVisitors.has(region)) {
      this.regionVisitors.set(region, new HyperLogLog(this.precision));
    }
    this.regionVisitors.get(region).update(userKey);

    // Track by device
    if (!this.deviceVisitors.has(device)) {
      this.deviceVisitors.set(device, new HyperLogLog(this.precision));
    }
    this.deviceVisitors.get(device).update(userKey);

    // Track by hour
    const hour = Math.floor(timestamp / 3600);
    if (!this.hourlyVisitors.has(hour)) {
      this.hourlyVisitors.set(hour, new HyperLogLog(this.precision));
    }
    this.hourlyVisitors.get(hour).update(userKey);
  }

  getReport() {
    // Global stats
    const globalUnique = Math.round(this.globalVisitors.estimate());

    // Top pages
    const pageStats = Array.from(this.pageVisitors.entries())
      .map(([page, hll]) => ({
        page,
        uniqueVisitors: Math.round(hll.estimate())
      }))
      .sort((a, b) => b.uniqueVisitors - a.uniqueVisitors);

    // Regional distribution
    const regionStats = Array.from(this.regionVisitors.entries())
      .map(([region, hll]) => ({
        region,
        uniqueVisitors: Math.round(hll.estimate())
      }))
      .sort((a, b) => b.uniqueVisitors - a.uniqueVisitors);

    // Device breakdown
    const deviceStats = Array.from(this.deviceVisitors.entries())
      .map(([device, hll]) => ({
        device,
        uniqueVisitors: Math.round(hll.estimate())
      }))
      .sort((a, b) => b.uniqueVisitors - a.uniqueVisitors);

    // Hourly traffic
    const hourlyStats = Array.from(this.hourlyVisitors.entries())
      .map(([hour, hll]) => ({
        hour,
        uniqueVisitors: Math.round(hll.estimate())
      }))
      .sort((a, b) => a.hour - b.hour);

    return {
      globalUnique,
      pageStats,
      regionStats,
      deviceStats,
      hourlyStats
    };
  }

  getMemoryUsage() {
    let total = 0;
    const registerSize = Math.pow(2, this.precision);

    // Global
    total += registerSize;

    // All dimensions
    total += this.pageVisitors.size * registerSize;
    total += this.regionVisitors.size * registerSize;
    total += this.deviceVisitors.size * registerSize;
    total += this.hourlyVisitors.size * registerSize;

    return total;
  }
}

// ============================================================================
// Generate Realistic Traffic
// ============================================================================

function generateTraffic(numEvents) {
  const pages = [
    '/', '/products', '/about', '/contact', '/blog',
    '/pricing', '/docs', '/support', '/login', '/signup'
  ];

  const regions = ['US-East', 'US-West', 'EU-West', 'Asia-Pacific', 'South-America'];
  const devices = ['Desktop', 'Mobile', 'Tablet'];

  const events = [];
  const uniqueUsers = new Set();

  // Generate events with realistic patterns
  for (let i = 0; i < numEvents; i++) {
    // 70% returning visitors, 30% new visitors
    const isReturning = Math.random() < 0.7 && uniqueUsers.size > 0;

    let userId;
    if (isReturning) {
      // Pick random existing user
      const users = Array.from(uniqueUsers);
      userId = users[Math.floor(Math.random() * users.length)];
    } else {
      // Create new user
      userId = `user_${uniqueUsers.size + 1}`;
      uniqueUsers.add(userId);
    }

    // Page popularity: exponential distribution
    const pageIndex = Math.floor(Math.pow(Math.random(), 2) * pages.length);

    // Region distribution: weighted
    const regionRand = Math.random();
    const region = regionRand < 0.4 ? regions[0]  // 40% US-East
      : regionRand < 0.7 ? regions[1]             // 30% US-West
        : regions[Math.floor(Math.random() * (regions.length - 2)) + 2]; // 30% others

    // Device distribution
    const deviceRand = Math.random();
    const device = deviceRand < 0.5 ? 'Mobile'     // 50% mobile
      : deviceRand < 0.85 ? 'Desktop'              // 35% desktop
        : 'Tablet';                                // 15% tablet

    // Timestamp: spread over 24 hours
    const timestamp = i * 86400 / numEvents;

    events.push({
      userId,
      page: pages[pageIndex],
      region,
      device,
      timestamp
    });
  }

  return { events, actualUniqueUsers: uniqueUsers.size };
}

// ============================================================================
// Run Analytics Simulation
// ============================================================================

console.log('Simulating 24-hour website traffic...\n');

const NUM_EVENTS = 100000;
const { events, actualUniqueUsers } = generateTraffic(NUM_EVENTS);

console.log(`Generated ${NUM_EVENTS.toLocaleString()} events`);
console.log(`Actual unique users: ${actualUniqueUsers.toLocaleString()}\n`);

// Track all events
const tracker = new AnalyticsTracker(14);

console.log('Processing events...');
const startTime = Date.now();

for (const event of events) {
  tracker.trackVisit(event);
}

const processingTime = Date.now() - startTime;

console.log(`✓ Processed ${NUM_EVENTS.toLocaleString()} events in ${processingTime}ms`);
console.log(`  Throughput: ${Math.round(NUM_EVENTS / processingTime * 1000).toLocaleString()} events/sec\n`);

// ============================================================================
// Generate Report
// ============================================================================

const report = tracker.getReport();

console.log('=== ANALYTICS DASHBOARD ===\n');

// Global stats
console.log('GLOBAL STATISTICS');
console.log('-----------------');
console.log(`Total Events: ${NUM_EVENTS.toLocaleString()}`);
console.log(`Unique Visitors (estimated): ${report.globalUnique.toLocaleString()}`);
console.log(`Unique Visitors (actual): ${actualUniqueUsers.toLocaleString()}`);
const error = Math.abs(report.globalUnique - actualUniqueUsers);
const errorPct = (error / actualUniqueUsers * 100).toFixed(2);
console.log(`Estimation Error: ${error} (${errorPct}%)\n`);

// Page statistics
console.log('TOP PAGES BY UNIQUE VISITORS');
console.log('----------------------------');
report.pageStats.slice(0, 5).forEach((stat, i) => {
  const bar = '█'.repeat(Math.floor(stat.uniqueVisitors / 100));
  console.log(`${i + 1}. ${stat.page.padEnd(15)} ${stat.uniqueVisitors.toString().padStart(6)} ${bar}`);
});
console.log('');

// Regional distribution
console.log('GEOGRAPHIC DISTRIBUTION');
console.log('-----------------------');
const totalRegional = report.regionStats.reduce((sum, s) => sum + s.uniqueVisitors, 0);
report.regionStats.forEach(stat => {
  const percentage = (stat.uniqueVisitors / totalRegional * 100).toFixed(1);
  const bar = '█'.repeat(Math.floor(stat.uniqueVisitors / 100));
  console.log(`${stat.region.padEnd(15)} ${percentage.padStart(5)}% ${bar}`);
});
console.log('');

// Device breakdown
console.log('DEVICE BREAKDOWN');
console.log('----------------');
const totalDevice = report.deviceStats.reduce((sum, s) => sum + s.uniqueVisitors, 0);
report.deviceStats.forEach(stat => {
  const percentage = (stat.uniqueVisitors / totalDevice * 100).toFixed(1);
  const bar = '█'.repeat(Math.floor(stat.uniqueVisitors / 100));
  console.log(`${stat.device.padEnd(10)} ${percentage.padStart(5)}% ${bar}`);
});
console.log('');

// Hourly traffic pattern
console.log('HOURLY UNIQUE VISITORS (First 12 hours)');
console.log('---------------------------------------');
report.hourlyStats.slice(0, 12).forEach(stat => {
  const hour = `${stat.hour.toString().padStart(2, '0')}:00`;
  const bar = '█'.repeat(Math.floor(stat.uniqueVisitors / 20));
  console.log(`${hour} ${bar} ${stat.uniqueVisitors}`);
});
console.log('');

// ============================================================================
// Memory Efficiency Analysis
// ============================================================================

console.log('MEMORY EFFICIENCY');
console.log('-----------------');

const memoryUsed = tracker.getMemoryUsage();
const totalHLLs = 1 + tracker.pageVisitors.size + tracker.regionVisitors.size +
  tracker.deviceVisitors.size + tracker.hourlyVisitors.size;

console.log(`Total HyperLogLog sketches: ${totalHLLs}`);
console.log(`Memory used: ${(memoryUsed / 1024).toFixed(2)} KB`);
console.log(`Per-sketch memory: ${(memoryUsed / totalHLLs / 1024).toFixed(2)} KB`);

// Compare with exact counting
const exactMemory = actualUniqueUsers * 50; // ~50 bytes per user ID
console.log(`\nComparison with exact counting:`);
console.log(`  Exact approach: ${(exactMemory / 1024).toFixed(2)} KB (storing all user IDs)`);
console.log(`  HyperLogLog: ${(memoryUsed / 1024).toFixed(2)} KB`);
console.log(`  Space savings: ${((1 - memoryUsed / exactMemory) * 100).toFixed(1)}%`);
console.log(`  Accuracy: ${(100 - parseFloat(errorPct)).toFixed(2)}%\n`);

// ============================================================================
// Advanced Use Case: Distributed Analytics
// ============================================================================

console.log('=== DISTRIBUTED ANALYTICS ===\n');

// Simulate 3 regional data centers
console.log('Simulating 3 regional data centers...');
const dc1 = new HyperLogLog(14);
const dc2 = new HyperLogLog(14);
const dc3 = new HyperLogLog(14);

// Split traffic across DCs
events.slice(0, Math.floor(events.length / 3)).forEach(e =>
  dc1.update(Buffer.from(e.userId))
);
events.slice(Math.floor(events.length / 3), Math.floor(events.length * 2 / 3)).forEach(e =>
  dc2.update(Buffer.from(e.userId))
);
events.slice(Math.floor(events.length * 2 / 3)).forEach(e =>
  dc3.update(Buffer.from(e.userId))
);

console.log(`DC1 unique visitors: ${Math.round(dc1.estimate()).toLocaleString()}`);
console.log(`DC2 unique visitors: ${Math.round(dc2.estimate()).toLocaleString()}`);
console.log(`DC3 unique visitors: ${Math.round(dc3.estimate()).toLocaleString()}`);

// Merge for global count
dc1.merge(dc2);
dc1.merge(dc3);

const globalEstimate = Math.round(dc1.estimate());
console.log(`\nGlobal unique (merged): ${globalEstimate.toLocaleString()}`);
console.log(`Actual unique: ${actualUniqueUsers.toLocaleString()}`);
console.log(`Merge accuracy: ${(100 - Math.abs(globalEstimate - actualUniqueUsers) / actualUniqueUsers * 100).toFixed(2)}%`);

// Serialize for data transfer
const serialized = dc1.serialize();
console.log(`\nSerialized size: ${serialized.length} bytes`);
console.log('(Can be transferred between DCs for aggregation)\n');

console.log('=== Summary ===\n');
console.log('Key Benefits:');
console.log('✓ 95%+ space savings vs exact counting');
console.log('✓ <1% error with standard parameters');
console.log('✓ O(1) update and query time');
console.log('✓ Mergeable across distributed systems');
console.log('✓ Serializable for persistence and transfer\n');

console.log('Production Use Cases:');
console.log('• Website/app analytics (Google Analytics style)');
console.log('• Ad impression unique reach estimation');
console.log('• Database query optimization (distinct counts)');
console.log('• Time-series database cardinality estimation');
console.log('• Network flow unique source/destination tracking\n');
