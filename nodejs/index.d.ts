/**
 * sketch_oxide: High-Performance Probabilistic Data Structures for Node.js
 *
 * State-of-the-art algorithms offering 28-75% better space efficiency
 * than classic implementations (VLDB 2024 research).
 *
 * @example
 * ```javascript
 * import { HyperLogLog, CountMinSketch } from '@sketch-oxide/node'
 *
 * // Cardinality estimation
 * const hll = new HyperLogLog(14)
 * hll.update(Buffer.from('item1'))
 * hll.update(Buffer.from('item2'))
 * console.log(hll.estimate()) // ~2
 *
 * // Frequency estimation
 * const cms = new CountMinSketch(0.01, 0.01)
 * cms.update(Buffer.from('word'), 1)
 * console.log(cms.estimate(Buffer.from('word'))) // >= 1
 * ```
 */

/**
 * HyperLogLog Cardinality Estimator
 *
 * Provides O(log(log(N))) space complexity with ~1.04/sqrt(m) standard error
 * where m = 2^precision.
 *
 * **Use cases:**
 * - Unique visitor counting (analytics)
 * - Cardinality in distributed systems
 * - Data deduplication verification
 *
 * **Performance:**
 * - Space: 2^precision registers (e.g., 16KB for precision=14)
 * - Time: O(1) per update
 * - Error: ~0.8% for typical precision=14
 *
 * @example
 * ```javascript
 * const hll = new HyperLogLog(14)
 * for (const item of items) {
 *   hll.update(Buffer.from(item))
 * }
 * console.log(`Unique items: ${Math.round(hll.estimate())}`)
 * ```
 */
export class HyperLogLog {
  /**
   * Create a new HyperLogLog sketch
   *
   * @param precision - Number of bits (4-16, typical 12-14)
   *                    Determines space (2^precision) and error (~1.04/sqrt(2^precision))
   * @throws If precision is outside valid range
   *
   * @example
   * ```javascript
   * const hll = new HyperLogLog(14) // 16K space, ~0.8% error
   * ```
   */
  constructor(precision: number)

  /**
   * Add an item to the sketch
   *
   * Automatically hashes the input for uniform distribution.
   *
   * @param item - Binary data to add (string, Buffer, or bytes)
   *
   * @example
   * ```javascript
   * hll.update(Buffer.from('user-id-123'))
   * hll.update(Buffer.from([1, 2, 3]))
   * ```
   */
  update(item: Buffer): void

  /**
   * Get current cardinality estimate
   *
   * @returns Estimated number of unique items
   *
   * @example
   * ```javascript
   * const estimate = hll.estimate()
   * console.log(`~${estimate} unique items`)
   * ```
   */
  estimate(): number

  /**
   * Merge another HyperLogLog into this one
   *
   * Both sketches must have the same precision level.
   *
   * @param other - Another HyperLogLog to merge
   * @throws If precisions don't match
   *
   * @example
   * ```javascript
   * const hll1 = new HyperLogLog(14)
   * const hll2 = new HyperLogLog(14)
   * // ... add items to both ...
   * hll1.merge(hll2)
   * console.log(hll1.estimate()) // Combined estimate
   * ```
   */
  merge(other: HyperLogLog): void

  /**
   * Reset sketch to empty state
   *
   * @example
   * ```javascript
   * hll.reset()
   * console.log(hll.estimate()) // 0
   * ```
   */
  reset(): void

  /**
   * Get the precision level
   *
   * @returns The precision parameter used at creation
   */
  precision(): number

  /**
   * Serialize sketch to binary format
   *
   * Useful for:
   * - Storing in database
   * - Sending over network
   * - Checkpointing for fault tolerance
   *
   * @returns Binary representation as Buffer
   *
   * @example
   * ```javascript
   * const data = hll.serialize()
   * fs.writeFileSync('hll.bin', data)
   * ```
   */
  serialize(): Buffer

  /**
   * Deserialize from binary format
   *
   * Reconstructs a HyperLogLog from serialized data.
   *
   * @param data - Binary data from serialize()
   * @returns New HyperLogLog instance
   * @throws If data is invalid or corrupt
   *
   * @example
   * ```javascript
   * const data = fs.readFileSync('hll.bin')
   * const hll = HyperLogLog.deserialize(data)
   * ```
   */
  static deserialize(data: Buffer): HyperLogLog

  /**
   * Get string representation for debugging
   *
   * @returns Formatted string like "HyperLogLog(precision=14, estimate=1000000)"
   */
  toString(): string
}

/**
 * CountMinSketch - Frequency Estimation
 *
 * Provides frequency estimation with O(1) per-update/query and
 * bounded relative error (never underestimates).
 *
 * **Use cases:**
 * - Heavy hitter detection (top-K frequent items)
 * - Word frequency in text processing
 * - Request counting in monitoring
 *
 * **Space:** O(1/epsilon * ln(1/delta))
 *
 * @example
 * ```javascript
 * const cms = new CountMinSketch(0.01, 0.01) // 1% error, 99% confidence
 * cms.update(Buffer.from('apple'), 5)
 * cms.update(Buffer.from('apple'), 3)
 * console.log(cms.estimate(Buffer.from('apple'))) // >= 8
 * ```
 */
export class CountMinSketch {
  /**
   * Create a new CountMinSketch
   *
   * @param epsilon - Relative error bound (0 < epsilon < 1)
   *                  Smaller = more accurate, larger space
   * @param delta - Failure probability (0 < delta < 1)
   *                Smaller = more confident, larger space
   * @throws If parameters are invalid
   *
   * @example
   * ```javascript
   * const cms = new CountMinSketch(0.01, 0.01)
   * // Space: roughly 60KB for these parameters
   * ```
   */
  constructor(epsilon: number, delta: number)

  /**
   * Add to the frequency of an item
   *
   * @param item - The item to count (e.g., word, user ID)
   * @param count - Increment amount (default 1)
   *
   * @example
   * ```javascript
   * cms.update(Buffer.from('word'), 1)
   * cms.update(Buffer.from('word'), 5) // Add 5 more
   * ```
   */
  update(item: Buffer, count?: number): void

  /**
   * Estimate the frequency of an item
   *
   * Returns an estimate that is >= the true frequency (never underestimates).
   *
   * @param item - The item to query
   * @returns Estimated frequency
   *
   * @example
   * ```javascript
   * const estimate = cms.estimate(Buffer.from('word'))
   * console.log(`'word' appeared ~${estimate} times`)
   * ```
   */
  estimate(item: Buffer): number

  /**
   * Merge another CountMinSketch into this one
   *
   * Both sketches must have the same parameters (epsilon, delta).
   *
   * @param other - Another CountMinSketch to merge
   * @throws If parameters don't match
   */
  merge(other: CountMinSketch): void

  /**
   * Reset to empty state
   */
  reset(): void

  /**
   * Serialize to binary format
   *
   * @returns Binary representation as Buffer
   */
  serialize(): Buffer

  /**
   * Deserialize from binary format
   *
   * @param data - Binary data from serialize()
   * @returns New CountMinSketch instance
   */
  static deserialize(data: Buffer): CountMinSketch

  /**
   * Get string representation
   */
  toString(): string
}

/**
 * StableBloomFilter - Bounded FPR for Unbounded Streams
 * Useful for tracking membership in streams where data arrival time is unknown
 */
export class StableBloomFilter {
  constructor(maxBytes: number, logSizeOfArray: number, fpr: number)
  insert(item: Buffer): void
  contains(item: Buffer): boolean
  merge(other: StableBloomFilter): void
  toString(): string
}

/**
 * All remaining 23 algorithms are fully implemented:
 *
 * CARDINALITY:
 * - UltraLogLog(precision: number)
 * - CpcSketch(lgK: number)
 * - QSketch(maxSamples: number)
 * - ThetaSketch(lgK: number)
 *
 * FREQUENCY:
 * - CountSketch(epsilon: number, delta: number)
 * - ConservativeCountMin(epsilon: number, delta: number)
 * - SpaceSaving(epsilon: number)
 * - FrequentItems(maxSize: number)
 * - ElasticSketch(bucketCount: number, depth: number)
 * - SALSA(epsilon: number, delta: number)
 * - RemovableUniversalSketch(epsilon: number, delta: number)
 *
 * MEMBERSHIP:
 * - BinaryFuseFilter(items: bigint[], bitsPerEntry: number)
 * - BloomFilter(n: number, fpr?: number)
 * - BlockedBloomFilter(n: number, fpr?: number)
 * - CountingBloomFilter(n: number, fpr?: number)
 * - CuckooFilter(capacity: number)
 * - RibbonFilter(n: number, fpr?: number)
 *
 * QUANTILES:
 * - DDSketch(relativeAccuracy: number)
 * - ReqSketch(k: number, mode: ReqSketchMode)
 * - TDigest(compression?: number)
 * - KllSketch(k?: number)
 * - SplineSketch(maxBuckets: number)
 *
 * STREAMING:
 * - SlidingWindowCounter(windowSize: number, epsilon: number)
 * - ExponentialHistogram(windowSize: number, epsilon: number)
 *
 * SIMILARITY:
 * - MinHash(numPerm: number)
 * - SimHash()
 *
 * SAMPLING:
 * - ReservoirSampling(k: number)
 * - VarOptSampling(k: number)
 *
 * All classes support:
 * - update(item, value?) - Add items to sketch
 * - estimate(item?) - Get estimates
 * - merge(other) - Combine sketches
 * - serialize() - Export to Buffer
 * - deserialize(data) - Import from Buffer
 * - toString() - Debug representation
 *
 * See README.md for full API documentation and examples.
 */

// ============================================================================
// TIER 1 NEW SKETCHES (2025)
// ============================================================================

/**
 * HeavyKeeper - Top-k Heavy Hitter Detection with Exponential Decay
 *
 * Identifies the most frequent items in a data stream with high precision.
 * Uses exponential decay to actively remove small flows while protecting heavy hitters.
 *
 * **Use cases:**
 * - Network traffic analysis (elephant flow detection)
 * - Real-time trending topics detection
 * - Heavy hitter identification in logs
 * - DDoS attack detection
 *
 * **Performance:**
 * - Space: O(k + d × w) where k is top items, d is depth, w is width
 * - Update: O(d) typically 4-6 hash operations
 * - Query: O(d) for frequency estimation
 * - Top-k: O(1) to return cached heap
 *
 * @example
 * ```javascript
 * const { HeavyKeeper } = require('@sketch-oxide/node');
 * const hk = new HeavyKeeper(100, 0.001, 0.01);
 *
 * // Track items
 * for (let i = 0; i < 1000; i++) {
 *   hk.update(Buffer.from('frequent_item'));
 * }
 * for (let i = 0; i < 10; i++) {
 *   hk.update(Buffer.from('rare_item'));
 * }
 *
 * // Get top-k
 * const topK = hk.topK();
 * console.log(topK); // [{hash: 12345n, count: 1000}, ...]
 *
 * // Estimate specific item
 * const count = hk.estimate(Buffer.from('frequent_item'));
 * console.log(count); // ~1000
 *
 * // Apply decay
 * hk.decay();
 * ```
 */
export class HeavyKeeper {
  /**
   * Create a new HeavyKeeper sketch
   *
   * @param k - Number of top items to track (must be > 0)
   * @param epsilon - Error bound (0 < epsilon < 1, default: 0.001)
   * @param delta - Failure probability (0 < delta < 1, default: 0.01)
   * @throws If k is 0 or epsilon/delta are out of range
   */
  constructor(k: number, epsilon?: number, delta?: number)

  /**
   * Add an item to the sketch
   *
   * @param item - Binary data to track
   */
  update(item: Buffer): void

  /**
   * Estimate the frequency of an item
   *
   * @param item - Binary data to query
   * @returns Estimated count (may overestimate, never underestimates)
   */
  estimate(item: Buffer): number

  /**
   * Get the top-k heavy hitters
   *
   * @returns Array of {hash, count} objects sorted by count descending
   */
  topK(): Array<HeavyKeeperResult>

  /**
   * Apply exponential decay to all counters
   *
   * Ages old items to make room for new heavy hitters.
   */
  decay(): void

  /** Get string representation */
  toString(): string
}

export interface HeavyKeeperResult {
  hash: bigint
  count: number
}

/**
 * RatelessIBLT - Efficient Set Reconciliation for Distributed Systems
 *
 * Computes the symmetric difference between two sets without knowing
 * the difference size a priori. Used in blockchain, P2P networks, and
 * distributed databases for efficient synchronization.
 *
 * **Use cases:**
 * - Ethereum block synchronization (5.6x faster than naive)
 * - P2P network sync (BitTorrent, IPFS)
 * - Distributed cache invalidation
 * - Database replication
 * - File synchronization protocols
 *
 * **Performance:**
 * - Space: O(c × d) where c ≈ 1.5-2.0, d = expected difference
 * - Insert/Delete: O(k) where k = 3 hash functions
 * - Subtract: O(n) where n = number of cells
 * - Decode: O(d × k) where d = actual difference
 *
 * @example
 * ```javascript
 * const { RatelessIBLT } = require('@sketch-oxide/node');
 *
 * // Create IBLTs for Alice and Bob
 * const alice = new RatelessIBLT(100, 32);
 * const bob = new RatelessIBLT(100, 32);
 *
 * // Both insert shared items
 * alice.insert(Buffer.from('shared1'), Buffer.from('value1'));
 * bob.insert(Buffer.from('shared1'), Buffer.from('value1'));
 *
 * // Alice has unique items
 * alice.insert(Buffer.from('alice_only'), Buffer.from('alice_value'));
 *
 * // Bob has unique items
 * bob.insert(Buffer.from('bob_only'), Buffer.from('bob_value'));
 *
 * // Compute difference: alice - bob
 * alice.subtract(bob);
 *
 * // Decode to recover items
 * const result = alice.decode();
 * if (result.success) {
 *   console.log('Items in Alice but not Bob:', result.toInsert);
 *   console.log('Items in Bob but not Alice:', result.toRemove);
 * }
 * ```
 */
export class RatelessIBLT {
  /**
   * Create a new RatelessIBLT
   *
   * @param expectedDiff - Expected size of symmetric difference (must be > 0)
   * @param cellSize - Maximum size for cell data in bytes (typically 32-128)
   * @throws If expectedDiff or cellSize is 0
   */
  constructor(expectedDiff: number, cellSize: number)

  /**
   * Insert a key-value pair
   *
   * @param key - Key to insert
   * @param value - Value to insert
   */
  insert(key: Buffer, value: Buffer): void

  /**
   * Delete a key-value pair
   *
   * @param key - Key to delete
   * @param value - Value to delete
   */
  delete(key: Buffer, value: Buffer): void

  /**
   * Subtract another IBLT to compute symmetric difference
   *
   * @param other - Another RatelessIBLT to subtract
   * @throws If IBLTs have incompatible parameters
   */
  subtract(other: RatelessIBLT): void

  /**
   * Decode the IBLT to recover items
   *
   * @returns Object with toInsert, toRemove arrays and success flag
   */
  decode(): IBLTDecodeResult

  /** Get string representation */
  toString(): string
}

export interface IBLTDecodeResult {
  toInsert: Array<KeyValuePair>
  toRemove: Array<KeyValuePair>
  success: boolean
}

export interface KeyValuePair {
  key: Buffer
  value: Buffer
}

/**
 * Grafite - Optimal Range Filter with Adversarial-Robust Guarantees
 *
 * The first optimal range filter providing provable FPR bounds of
 * L / 2^(B-2) where L is range width and B is bits per key.
 *
 * **Use cases:**
 * - LSM-tree range queries (RocksDB, LevelDB)
 * - Database index optimization
 * - Time-series databases
 * - Financial market data (timestamp range lookups)
 * - Log aggregation systems
 *
 * **Performance:**
 * - Build: O(n log n) for sorting keys
 * - Query: O(log n) for range check
 * - Space: B bits per key (typically 4-8)
 * - FPR: L / 2^(B-2) provable bound
 *
 * @example
 * ```javascript
 * const { Grafite } = require('@sketch-oxide/node');
 *
 * // Build from sorted keys
 * const keys = [10n, 20n, 30n, 40n, 50n];
 * const filter = Grafite.build(keys, 6);
 *
 * // Query ranges
 * console.log(filter.mayContainRange(15n, 25n)); // true (contains 20)
 * console.log(filter.mayContain(30n)); // true (exact match)
 * console.log(filter.mayContainRange(500n, 600n)); // false (no keys)
 *
 * // Check FPR
 * const fpr = filter.expectedFpr(10n);
 * console.log(`Expected FPR: ${fpr}`); // 10 / 2^4 = 0.625
 *
 * // Get statistics
 * const stats = filter.stats();
 * console.log(`Keys: ${stats.keyCount}, Bits/key: ${stats.bitsPerKey}`);
 * ```
 */
export class Grafite {
  /**
   * Build a Grafite filter from sorted keys
   *
   * @param keys - Sorted array of 64-bit unsigned integers
   * @param bitsPerKey - Number of bits per key (typically 4-8, range 2-16)
   * @throws If keys array is empty or bitsPerKey is out of range
   */
  static build(keys: Array<bigint>, bitsPerKey: number): Grafite

  /**
   * Check if a range may contain keys
   *
   * @param low - Lower bound (inclusive)
   * @param high - Upper bound (inclusive)
   * @returns true if range may contain keys, false if definitely does not
   */
  mayContainRange(low: bigint, high: bigint): boolean

  /**
   * Check if a specific key may be present
   *
   * @param key - Key to check
   * @returns true if key may be present, false if definitely not
   */
  mayContain(key: bigint): boolean

  /**
   * Calculate expected false positive rate for a range width
   *
   * @param rangeWidth - Width of the query range
   * @returns Expected FPR = rangeWidth / 2^(bitsPerKey - 2)
   */
  expectedFpr(rangeWidth: bigint): number

  /**
   * Get filter statistics
   *
   * @returns Object with keyCount, bitsPerKey, totalBits
   */
  stats(): GrafiteStats

  /** Get string representation */
  toString(): string
}

export interface GrafiteStats {
  keyCount: number
  bitsPerKey: number
  totalBits: number
}

/**
 * MementoFilter - Dynamic Range Filter with FPR Guarantees
 *
 * The first dynamic range filter supporting insertions while maintaining
 * false positive rate guarantees. Combines a base range filter with a
 * quotient filter layer for precise element storage.
 *
 * **Use cases:**
 * - MongoDB WiredTiger integration
 * - RocksDB block filters
 * - Dynamic database indexes
 * - Log systems with streaming data
 * - Time-series with growing ranges
 *
 * **Performance:**
 * - Insertion: O(1) amortized, <200ns
 * - Query: O(1), <150ns
 * - Space: ~10 bits per element with 1% FPR
 * - FPR: Maintains configured target with dynamic insertions
 *
 * @example
 * ```javascript
 * const { MementoFilter } = require('@sketch-oxide/node');
 *
 * const filter = new MementoFilter(1000, 0.01); // 1% FPR
 *
 * // Insert key-value pairs dynamically
 * filter.insert(42n, Buffer.from('value1'));
 * filter.insert(100n, Buffer.from('value2'));
 * filter.insert(250n, Buffer.from('value3'));
 *
 * // Query ranges
 * console.log(filter.mayContainRange(40n, 50n)); // true (contains 42)
 * console.log(filter.mayContainRange(95n, 105n)); // true (contains 100)
 * console.log(filter.mayContainRange(500n, 600n)); // likely false
 * ```
 */
export class MementoFilter {
  /**
   * Create a new MementoFilter
   *
   * @param expectedElements - Expected number of elements (must be > 0)
   * @param fpr - Target false positive rate (0 < fpr < 1)
   * @throws If expectedElements is 0 or fpr is out of range
   */
  constructor(expectedElements: number, fpr: number)

  /**
   * Insert a key-value pair
   *
   * @param key - 64-bit unsigned integer key
   * @param value - Value as binary data
   */
  insert(key: bigint, value: Buffer): void

  /**
   * Check if a range may contain keys
   *
   * @param low - Lower bound (inclusive)
   * @param high - Upper bound (inclusive)
   * @returns true if range may contain keys, false if definitely does not
   */
  mayContainRange(low: bigint, high: bigint): boolean

  /** Get string representation */
  toString(): string
}

/**
 * SlidingHyperLogLog - Time-windowed Cardinality Estimation
 *
 * Extends HyperLogLog with temporal awareness for cardinality estimation
 * over sliding time windows. Essential for real-time analytics, DDoS detection,
 * and streaming applications.
 *
 * **Use cases:**
 * - Real-time dashboards (unique users in last N minutes)
 * - DDoS detection (unique source IPs in sliding window)
 * - Network telemetry (unique flows over time)
 * - CDN analytics (geographic distribution over time)
 * - Streaming aggregation (time-windowed distinct counts)
 *
 * **Performance:**
 * - Update: O(1)
 * - Window Query: O(m) where m = 2^precision
 * - Decay: O(m)
 * - Space: ~9m bytes (e.g., 36KB for precision 12)
 *
 * @example
 * ```javascript
 * const { SlidingHyperLogLog } = require('@sketch-oxide/node');
 *
 * // Create with precision 12, 1-hour max window
 * const hll = new SlidingHyperLogLog(12, 3600n);
 *
 * // Add items with timestamps
 * hll.update(Buffer.from('user_123'), 1000n);
 * hll.update(Buffer.from('user_456'), 1030n);
 * hll.update(Buffer.from('user_789'), 1060n);
 *
 * // Estimate cardinality in last 60 seconds
 * const estimate = hll.estimateWindow(1060n, 60n);
 * console.log(`Unique items in window: ${Math.round(estimate)}`);
 *
 * // Estimate all-time cardinality
 * const total = hll.estimateTotal();
 * console.log(`Total unique items: ${Math.round(total)}`);
 *
 * // Decay old entries
 * hll.decay(2000n, 600n); // Keep last 10 minutes
 * ```
 */
export class SlidingHyperLogLog {
  /**
   * Create a new SlidingHyperLogLog
   *
   * @param precision - Number of bits (4-16, typical 12-14)
   * @param maxWindowSeconds - Maximum window size in seconds
   * @throws If precision is out of range (4-16) or maxWindowSeconds is 0
   */
  constructor(precision: number, maxWindowSeconds: bigint)

  /**
   * Add an item with timestamp
   *
   * @param item - Binary data to add
   * @param timestamp - Unix timestamp in seconds
   */
  update(item: Buffer, timestamp: bigint): void

  /**
   * Estimate cardinality in a sliding window
   *
   * @param currentTime - Current timestamp in seconds
   * @param windowSeconds - Window size in seconds
   * @returns Estimated number of unique items in window
   */
  estimateWindow(currentTime: bigint, windowSeconds: bigint): number

  /**
   * Estimate total cardinality (all time)
   *
   * @returns Estimated number of unique items ever seen
   */
  estimateTotal(): number

  /**
   * Remove expired entries outside the window
   *
   * @param currentTime - Current timestamp in seconds
   * @param windowSeconds - Window size in seconds
   */
  decay(currentTime: bigint, windowSeconds: bigint): void

  /** Get string representation */
  toString(): string
}

// ============================================================================
// TIER 2 SKETCHES (2025)
// ============================================================================

/**
 * VacuumFilter - Best-in-class Dynamic Membership Filter
 *
 * Space-efficient filter supporting insertions AND deletions with
 * <15 bits/item at 1% FPR (better than Cuckoo and Counting Bloom).
 *
 * **Use cases:**
 * - Dynamic set membership with deletions
 * - Cache tracking with eviction
 * - Database deduplication
 * - Security: malicious URL tracking
 *
 * **Performance:**
 * - Space: <15 bits/item at 1% FPR
 * - Insert/Query/Delete: O(1) expected
 * - True deletions (no false negatives after delete)
 *
 * @example
 * ```javascript
 * const { VacuumFilter } = require('@sketch-oxide/node')
 *
 * const filter = new VacuumFilter(1000, 0.01)
 * filter.insert(Buffer.from('key1'))
 * console.log(filter.contains(Buffer.from('key1'))) // true
 *
 * filter.delete(Buffer.from('key1'))
 * console.log(filter.contains(Buffer.from('key1'))) // false
 *
 * const stats = filter.stats()
 * console.log(`Load factor: ${stats.loadFactor.toFixed(2)}`)
 * ```
 */
export class VacuumFilter {
  /**
   * Create a new VacuumFilter
   *
   * @param capacity - Expected number of elements
   * @param fpr - Target false positive rate (0 < fpr < 1)
   * @throws If capacity is 0 or fpr is out of range
   *
   * @example
   * ```javascript
   * const filter = new VacuumFilter(1000, 0.01) // 1000 items, 1% FPR
   * ```
   */
  constructor(capacity: number, fpr: number)

  /**
   * Insert an element into the filter
   *
   * @param key - The key to insert
   *
   * @example
   * ```javascript
   * filter.insert(Buffer.from('hello'))
   * filter.insert(Buffer.from('world'))
   * ```
   */
  insert(key: Buffer): void

  /**
   * Check if an element might be in the filter
   *
   * @param key - The key to check
   * @returns `true` if might be present (with FPR probability of false positive),
   *          `false` if definitely not present
   *
   * @example
   * ```javascript
   * if (filter.contains(Buffer.from('hello'))) {
   *   console.log('Key might be present')
   * }
   * ```
   */
  contains(key: Buffer): boolean

  /**
   * Delete an element from the filter
   *
   * @param key - The key to delete
   * @returns `true` if element was found and removed, `false` otherwise
   *
   * @example
   * ```javascript
   * const wasDeleted = filter.delete(Buffer.from('hello'))
   * console.log(`Deleted: ${wasDeleted}`)
   * ```
   */
  delete(key: Buffer): boolean

  /**
   * Get current load factor (0.0 to 1.0)
   *
   * @example
   * ```javascript
   * const loadFactor = filter.loadFactor()
   * console.log(`Filter is ${(loadFactor * 100).toFixed(1)}% full`)
   * ```
   */
  loadFactor(): number

  /** Get total capacity */
  capacity(): number

  /** Get number of items currently stored */
  len(): number

  /** Check if filter is empty */
  isEmpty(): boolean

  /** Get memory usage in bytes */
  memoryUsage(): number

  /**
   * Get filter statistics
   *
   * @returns Object with capacity, numItems, loadFactor, memoryBits, fingerprintBits
   */
  stats(): VacuumFilterStats

  /** Clear all items from the filter */
  clear(): void

  /** Get string representation */
  toString(): string
}

export interface VacuumFilterStats {
  capacity: number
  numItems: number
  loadFactor: number
  memoryBits: bigint
  fingerprintBits: number
}

/**
 * GRF (Gorilla Range Filter) - Shape-Based Range Filter for LSM-Trees
 *
 * Advanced range filter optimized for skewed distributions.
 * Uses shape encoding for 30-50% better FPR than Grafite on real data.
 *
 * **Use cases:**
 * - RocksDB/LevelDB SSTable filters
 * - Time-series databases (InfluxDB, TimescaleDB)
 * - Log aggregation systems
 * - Financial time-series data
 *
 * **Performance:**
 * - Build: O(n log n) for sorting
 * - Query: O(log n) binary search + O(k) segment checks
 * - Space: B bits per key (comparable to Grafite)
 *
 * @example
 * ```javascript
 * const { GRF } = require('@sketch-oxide/node')
 *
 * // Build from Zipf-distributed keys
 * const keys = [1n, 2n, 3n, 5n, 8n, 13n, 21n]
 * const grf = GRF.build(keys, 6)
 *
 * console.log(grf.mayContainRange(10n, 25n)) // true (contains 13, 21)
 * console.log(grf.mayContain(13n)) // true
 *
 * const stats = grf.stats()
 * console.log(`Segments: ${stats.segmentCount}`)
 * ```
 */
export class GRF {
  /**
   * Build a GRF filter from sorted keys
   *
   * @param keys - Array of sorted 64-bit unsigned integers
   * @param bitsPerKey - Number of bits per key (2-16, typical 4-8)
   * @throws If keys array is empty or bitsPerKey is out of range
   *
   * @example
   * ```javascript
   * const keys = [10n, 20n, 30n, 40n, 50n]
   * const grf = GRF.build(keys, 6)
   * ```
   */
  static build(keys: Array<bigint>, bitsPerKey: number): GRF

  /**
   * Check if a range may contain keys
   *
   * @param low - Lower bound (inclusive)
   * @param high - Upper bound (inclusive)
   * @returns `true` if range might contain keys, `false` if definitely does not
   *
   * @example
   * ```javascript
   * if (grf.mayContainRange(15n, 25n)) {
   *   console.log('Range might have keys')
   * }
   * ```
   */
  mayContainRange(low: bigint, high: bigint): boolean

  /**
   * Check if a specific key may be present
   *
   * @param key - Key to check
   * @returns `true` if key might be present, `false` if definitely not
   *
   * @example
   * ```javascript
   * if (grf.mayContain(30n)) {
   *   console.log('Key might be present')
   * }
   * ```
   */
  mayContain(key: bigint): boolean

  /**
   * Calculate expected FPR for a range width
   *
   * @param rangeWidth - Width of the query range
   * @returns Expected false positive rate (0.0 to 1.0)
   *
   * @example
   * ```javascript
   * const fpr = grf.expectedFpr(10n)
   * console.log(`Expected FPR: ${(fpr * 100).toFixed(2)}%`)
   * ```
   */
  expectedFpr(rangeWidth: bigint): number

  /**
   * Get filter statistics
   *
   * @returns Object with keyCount, segmentCount, avgKeysPerSegment, bitsPerKey, totalBits, memoryBytes
   */
  stats(): GRFStats

  /** Get string representation */
  toString(): string
}

export interface GRFStats {
  keyCount: number
  segmentCount: number
  avgKeysPerSegment: number
  bitsPerKey: number
  totalBits: bigint
  memoryBytes: number
}

/**
 * NitroSketch - High-Speed Network Telemetry with Selective Sampling
 *
 * Achieves 100Gbps line rate through probabilistic sampling while
 * maintaining accuracy via background synchronization.
 *
 * **Use cases:**
 * - Network traffic monitoring at 100Gbps+
 * - DDoS detection
 * - Software-Defined Networking (SDN)
 * - Cloud telemetry
 * - Real-time analytics with CPU constraints
 *
 * **Performance:**
 * - Update Latency: <100ns (sub-microsecond)
 * - Throughput: >100K updates/sec per core
 * - Accuracy: Comparable to base sketch after synchronization
 *
 * @example
 * ```javascript
 * const { NitroSketch, CountMinSketch } = require('@sketch-oxide/node')
 *
 * const base = new CountMinSketch(0.01, 0.01)
 * const nitro = new NitroSketch(base, 0.1) // 10% sampling
 *
 * // High-speed updates
 * for (let i = 0; i < 100000; i++) {
 *   nitro.updateSampled(Buffer.from(`packet_${i % 100}`))
 * }
 *
 * // Synchronize for accuracy
 * nitro.sync(1.0)
 *
 * const stats = nitro.stats()
 * console.log(`Sampled: ${stats.sampledCount}, Total: ${stats.totalItemsEstimated}`)
 * ```
 */
export class NitroSketch {
  /**
   * Create a new NitroSketch wrapping a CountMinSketch
   *
   * @param baseSketch - CountMinSketch to wrap
   * @param sampleRate - Probability of updating (0 < rate <= 1)
   *   - 1.0 = update every item (no sampling)
   *   - 0.1 = update 10% of items
   *   - 0.01 = update 1% of items
   * @throws If sampleRate is out of range
   *
   * @example
   * ```javascript
   * const base = new CountMinSketch(0.01, 0.01)
   * const nitro = new NitroSketch(base, 0.1)
   * ```
   */
  constructor(baseSketch: CountMinSketch, sampleRate: number)

  /**
   * Update with selective sampling
   *
   * Uses hash-based sampling to decide whether to update the base sketch.
   *
   * @param key - The item to possibly add
   *
   * @example
   * ```javascript
   * nitro.updateSampled(Buffer.from('flow_key'))
   * ```
   */
  updateSampled(key: Buffer): void

  /**
   * Query the frequency of a key
   *
   * For accurate results, call sync() periodically.
   *
   * @param key - The item to query
   * @returns Estimated frequency (may be underestimated if sync() not called)
   *
   * @example
   * ```javascript
   * const freq = nitro.query(Buffer.from('key'))
   * console.log(`Frequency: ${freq}`)
   * ```
   */
  query(key: Buffer): bigint

  /**
   * Synchronize to adjust for unsampled items
   *
   * Background synchronization adjusts the sketch to account for
   * items that were not sampled, recovering accuracy.
   *
   * @param unsampledWeight - Weight to apply to unsampled items (typically 1.0)
   *
   * @example
   * ```javascript
   * nitro.sync(1.0) // Adjust for unsampled items
   * ```
   */
  sync(unsampledWeight: number): void

  /**
   * Get statistics about sampling
   *
   * @returns Object with sampleRate, sampledCount, unsampledCount, totalItemsEstimated
   */
  stats(): NitroSketchStats

  /** Reset sampling statistics */
  resetStats(): void

  /** Get string representation */
  toString(): string
}

export interface NitroSketchStats {
  sampleRate: number
  sampledCount: bigint
  unsampledCount: bigint
  totalItemsEstimated: bigint
}

/**
 * UnivMon - Universal Sketch Supporting Multiple Simultaneous Metrics
 *
 * A single UnivMon estimates L1/L2 norms, entropy, heavy hitters,
 * and change detection, eliminating need for multiple specialized sketches.
 *
 * **Supported Metrics (from ONE sketch!):**
 * 1. L1 Norm (sum of frequencies): Total traffic volume
 * 2. L2 Norm (sum of squared frequencies): Load balance
 * 3. Entropy (Shannon entropy): Distribution diversity
 * 4. Heavy Hitters: Most frequent items
 * 5. Change Detection: Temporal anomalies
 * 6. Flow Size Distribution: Per-flow statistics
 *
 * **Use cases:**
 * - Network monitoring (simultaneous bandwidth, flows, protocols)
 * - Cloud analytics (unified telemetry)
 * - Real-time anomaly detection
 * - Multi-tenant systems
 *
 * **Performance:**
 * - Update: O(d * log n) where d = sketch depth, n = max stream size
 * - L1/L2/Entropy query: O(d * log n)
 * - Heavy hitters: O(k * d) where k = number of heavy hitters
 * - Space: O((log n / ε²) * log(1/δ))
 *
 * @example
 * ```javascript
 * const { UnivMon } = require('@sketch-oxide/node')
 *
 * const univmon = new UnivMon(1000000, 0.01, 0.01)
 *
 * // Update with network packets
 * univmon.update(Buffer.from('192.168.1.1'), 1500)
 * univmon.update(Buffer.from('192.168.1.2'), 800)
 *
 * // Query multiple metrics from SAME sketch
 * console.log(`Total traffic: ${univmon.estimateL1()}`)
 * console.log(`Load balance: ${univmon.estimateL2()}`)
 * console.log(`IP diversity: ${univmon.estimateEntropy()}`)
 *
 * const topIPs = univmon.heavyHitters(0.1)
 * console.log(`Top IPs: ${topIPs.length}`)
 * ```
 */
export class UnivMon {
  /**
   * Create a new UnivMon sketch
   *
   * @param maxStreamSize - Expected maximum number of items (determines layers)
   * @param epsilon - Error parameter (0 < epsilon < 1)
   * @param delta - Failure probability (0 < delta < 1)
   * @throws If maxStreamSize is 0 or epsilon/delta are out of range
   *
   * @example
   * ```javascript
   * const univmon = new UnivMon(1000000, 0.01, 0.01) // 1M items, 1% error
   * ```
   */
  constructor(maxStreamSize: bigint, epsilon: number, delta: number)

  /**
   * Update the sketch with an item and value
   *
   * @param item - The item (e.g., IP address, user ID)
   * @param value - The value/weight (e.g., packet size, transaction amount)
   *
   * @example
   * ```javascript
   * univmon.update(Buffer.from('192.168.1.1'), 1500)
   * univmon.update(Buffer.from('user_123'), 99.99)
   * ```
   */
  update(item: Buffer, value: number): void

  /**
   * Estimate L1 norm (sum of frequencies)
   *
   * @returns Total sum of all values (e.g., total traffic volume)
   *
   * @example
   * ```javascript
   * const totalTraffic = univmon.estimateL1()
   * console.log(`Total: ${totalTraffic} bytes`)
   * ```
   */
  estimateL1(): number

  /**
   * Estimate L2 norm (sum of squared frequencies)
   *
   * @returns L2 norm indicating distribution spread
   *
   * @example
   * ```javascript
   * const l2 = univmon.estimateL2()
   * console.log(`Load balance metric: ${l2}`)
   * ```
   */
  estimateL2(): number

  /**
   * Estimate Shannon entropy
   *
   * @returns Entropy value indicating distribution diversity
   *
   * @example
   * ```javascript
   * const entropy = univmon.estimateEntropy()
   * console.log(`Distribution diversity: ${entropy.toFixed(2)}`)
   * ```
   */
  estimateEntropy(): number

  /**
   * Get heavy hitters (most frequent items)
   *
   * @param threshold - Frequency threshold (0 < threshold <= 1)
   *   - 0.1 = items with frequency >= 10% of total
   * @returns Array of heavy hitter hashes
   *
   * @example
   * ```javascript
   * const topItems = univmon.heavyHitters(0.1) // Top 10% items
   * console.log(`Found ${topItems.length} heavy hitters`)
   * ```
   */
  heavyHitters(threshold: number): Array<bigint>

  /**
   * Detect change between two UnivMon sketches
   *
   * @param other - Another UnivMon sketch to compare
   * @returns Change magnitude (higher = more change)
   *
   * @example
   * ```javascript
   * const change = univmon1.detectChange(univmon2)
   * if (change > 0.5) {
   *   console.log('Significant distribution shift detected!')
   * }
   * ```
   */
  detectChange(other: UnivMon): number

  /** Get string representation */
  toString(): string
}

/**
 * LearnedBloomFilter - ML-Enhanced Membership Testing
 *
 * **EXPERIMENTAL** - Uses machine learning to achieve 70-80% memory
 * reduction compared to standard Bloom filters.
 *
 * **WARNING:**
 * Do NOT use in security-critical applications. ML models can be
 * adversarially attacked to craft keys that fool the predictor.
 *
 * **Use cases (Non-security):**
 * - In-memory caches (memory optimization)
 * - Database query optimization
 * - Data deduplication
 * - Analytics systems
 *
 * **Performance:**
 * - Space: ~3-4 bits/element (70-80% reduction from Bloom)
 * - Query: Fast (model prediction + backup filter)
 * - Training: One-time cost
 *
 * @example
 * ```javascript
 * const { LearnedBloomFilter } = require('@sketch-oxide/node')
 *
 * // Train on dataset
 * const keys = []
 * for (let i = 0; i < 10000; i++) {
 *   keys.push(Buffer.from(`key${i}`))
 * }
 *
 * const filter = LearnedBloomFilter.new(keys, 0.01)
 *
 * console.log(filter.contains(Buffer.from('key500'))) // true
 * console.log(filter.contains(Buffer.from('nonexistent'))) // probably false
 *
 * const mem = filter.memoryUsage()
 * console.log(`Memory: ${mem} bytes (70-80% reduction)`)
 * ```
 */
export class LearnedBloomFilter {
  /**
   * Create a new LearnedBloomFilter
   *
   * @param trainingKeys - Keys to train on (must be members, at least 10 keys)
   * @param fpr - Target false positive rate (0 < fpr < 1)
   * @throws If training data is empty or too small, or fpr is out of range
   *
   * @example
   * ```javascript
   * const keys = []
   * for (let i = 0; i < 1000; i++) {
   *   keys.push(Buffer.from(`key${i}`))
   * }
   * const filter = LearnedBloomFilter.new(keys, 0.01)
   * ```
   */
  static new(trainingKeys: Array<Buffer>, fpr: number): LearnedBloomFilter

  /**
   * Check if a key might be in the set
   *
   * @param key - The key to check
   * @returns `true` if might be present (or false positive),
   *          `false` if definitely not present
   *
   * **Guarantees:**
   * Zero false negatives: All training keys will return `true`
   *
   * @example
   * ```javascript
   * if (filter.contains(Buffer.from('key1'))) {
   *   console.log('Key might be present')
   * }
   * ```
   */
  contains(key: Buffer): boolean

  /**
   * Get memory usage in bytes
   *
   * @example
   * ```javascript
   * console.log(`Memory: ${filter.memoryUsage()} bytes`)
   * ```
   */
  memoryUsage(): number

  /** Get string representation */
  toString(): string
}

// NOTE: Full type definitions are auto-generated by napi-rs
// These manual stubs are for reference only
