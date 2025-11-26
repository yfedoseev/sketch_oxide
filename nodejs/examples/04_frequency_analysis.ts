/**
 * Frequency Analysis - Stream Processing with CountMinSketch
 *
 * Demonstrates using CountMinSketch for real-time word frequency analysis
 * in text streams (similar to Twitter trending topics, log analysis).
 *
 * CountMinSketch provides frequency estimation with bounded error and
 * never underestimates - perfect for finding heavy hitters.
 *
 * Run: npx ts-node 04_frequency_analysis.ts
 */

import { CountMinSketch } from '..';
import * as crypto from 'crypto';

console.log('=== Real-Time Word Frequency Analysis ===\n');

// ============================================================================
// Stream Processing System
// ============================================================================

interface WordFrequency {
  word: string;
  estimatedCount: number;
}

class StreamProcessor {
  private sketch: CountMinSketch;
  private epsilon: number;
  private delta: number;

  constructor(epsilon: number = 0.01, delta: number = 0.01) {
    this.epsilon = epsilon;
    this.delta = delta;
    this.sketch = new CountMinSketch(epsilon, delta);
  }

  processWord(word: string, count: number = 1): void {
    this.sketch.update(Buffer.from(word.toLowerCase()), count);
  }

  getFrequency(word: string): number {
    return this.sketch.estimate(Buffer.from(word.toLowerCase()));
  }

  // Note: CMS doesn't have built-in top-k, so we track separately
  // In production, use HeavyKeeper or combine with a heap
}

// ============================================================================
// Example 1: Twitter-Style Trending Topics
// ============================================================================

console.log('Example 1: Trending Topics Analysis');
console.log('====================================\n');

interface Tweet {
  id: string;
  text: string;
  timestamp: number;
  hashtags: string[];
}

function generateTweets(count: number): Tweet[] {
  const trendingTopics = ['#AI', '#MachineLearning', '#CloudComputing', '#Cybersecurity'];
  const normalTopics = ['#technology', '#programming', '#datascience', '#webdev',
    '#javascript', '#python', '#devops', '#cloud'];

  const tweets: Tweet[] = [];

  for (let i = 0; i < count; i++) {
    const isTrending = Math.random() < 0.3; // 30% trending topics
    const topics = isTrending ? trendingTopics : normalTopics;

    const numHashtags = Math.floor(Math.random() * 3) + 1;
    const hashtags: string[] = [];

    for (let j = 0; j < numHashtags; j++) {
      hashtags.push(topics[Math.floor(Math.random() * topics.length)]);
    }

    tweets.push({
      id: crypto.randomUUID(),
      text: `Sample tweet ${i}`,
      timestamp: Date.now() + i * 1000,
      hashtags
    });
  }

  return tweets;
}

const tweetProcessor = new StreamProcessor(0.001, 0.01);
const hashtagCounts = new Map<string, number>(); // Ground truth

console.log('Processing tweet stream...');
const tweets = generateTweets(10000);

for (const tweet of tweets) {
  for (const hashtag of tweet.hashtags) {
    tweetProcessor.processWord(hashtag);

    // Track exact counts for comparison
    hashtagCounts.set(hashtag, (hashtagCounts.get(hashtag) || 0) + 1);
  }
}

console.log(`✓ Processed ${tweets.length.toLocaleString()} tweets\n`);

// Find trending topics
console.log('TRENDING TOPICS (Top 10):');
console.log('-------------------------');

const sortedHashtags = Array.from(hashtagCounts.entries())
  .sort((a, b) => b[1] - a[1])
  .slice(0, 10);

for (const [hashtag, actualCount] of sortedHashtags) {
  const estimated = tweetProcessor.getFrequency(hashtag);
  const error = Math.abs(estimated - actualCount);
  const errorPct = (error / actualCount * 100).toFixed(1);

  const bar = '█'.repeat(Math.floor(actualCount / 50));
  console.log(`${hashtag.padEnd(20)} Est: ${estimated.toString().padStart(5)} ` +
    `Actual: ${actualCount.toString().padStart(5)} ` +
    `Error: ${errorPct}% ${bar}`);
}

// ============================================================================
// Example 2: Log Analysis - Error Detection
// ============================================================================

console.log('\n\nExample 2: Log File Analysis');
console.log('=============================\n');

interface LogEntry {
  level: 'INFO' | 'WARN' | 'ERROR' | 'DEBUG';
  message: string;
  service: string;
  timestamp: number;
}

function generateLogs(count: number): LogEntry[] {
  const services = ['auth-service', 'api-gateway', 'database', 'cache', 'worker'];
  const errorTypes = [
    'ConnectionTimeout',
    'NullPointerException',
    'OutOfMemory',
    'DatabaseDeadlock',
    'RateLimitExceeded',
    'InvalidCredentials'
  ];

  const logs: LogEntry[] = [];

  for (let i = 0; i < count; i++) {
    // 10% errors, 20% warnings, 70% info
    const rand = Math.random();
    const level = rand < 0.1 ? 'ERROR' : rand < 0.3 ? 'WARN' : 'INFO';

    let message;
    if (level === 'ERROR') {
      message = errorTypes[Math.floor(Math.random() * errorTypes.length)];
    } else {
      message = `Routine ${level} message ${i}`;
    }

    logs.push({
      level,
      message,
      service: services[Math.floor(Math.random() * services.length)],
      timestamp: Date.now() + i
    });
  }

  return logs;
}

const logProcessor = new StreamProcessor(0.005, 0.01);
const errorCounts = new Map<string, number>();
const serviceCounts = new Map<string, number>();

console.log('Analyzing log stream...');
const logs = generateLogs(50000);

for (const log of logs) {
  // Track error types
  if (log.level === 'ERROR') {
    logProcessor.processWord(`error:${log.message}`);
    errorCounts.set(log.message, (errorCounts.get(log.message) || 0) + 1);
  }

  // Track service activity
  logProcessor.processWord(`service:${log.service}`);
  serviceCounts.set(log.service, (serviceCounts.get(log.service) || 0) + 1);
}

console.log(`✓ Processed ${logs.length.toLocaleString()} log entries\n`);

// Error frequency analysis
console.log('ERROR FREQUENCY ANALYSIS:');
console.log('------------------------');

const sortedErrors = Array.from(errorCounts.entries())
  .sort((a, b) => b[1] - a[1]);

for (const [error, actualCount] of sortedErrors) {
  const estimated = logProcessor.getFrequency(`error:${error}`);
  const bar = '█'.repeat(Math.floor(actualCount / 10));
  console.log(`${error.padEnd(25)} ${estimated.toString().padStart(4)} ${bar}`);
}

// Service load analysis
console.log('\nSERVICE REQUEST DISTRIBUTION:');
console.log('----------------------------');

const sortedServices = Array.from(serviceCounts.entries())
  .sort((a, b) => b[1] - a[1]);

for (const [service, actualCount] of sortedServices) {
  const estimated = logProcessor.getFrequency(`service:${service}`);
  const percentage = (actualCount / logs.length * 100).toFixed(1);
  const bar = '█'.repeat(Math.floor(actualCount / 500));
  console.log(`${service.padEnd(15)} ${percentage.padStart(5)}% Est: ${estimated.toString().padStart(6)} ${bar}`);
}

// ============================================================================
// Example 3: Real-Time Text Stream Processing
// ============================================================================

console.log('\n\nExample 3: Document Stream Word Frequency');
console.log('=========================================\n');

const textCorpus = `
In the field of data structures and algorithms, probabilistic data structures
offer a powerful tradeoff between memory efficiency and accuracy. These structures
use randomization and hashing to achieve space complexity that is orders of magnitude
better than deterministic alternatives. Count-Min Sketch, HyperLogLog, and Bloom Filters
are among the most widely used probabilistic structures in production systems.

Count-Min Sketch provides frequency estimation with provable error bounds. Unlike
exact counting which requires O(n) space, Count-Min Sketch uses O(1/ε × log(1/δ)) space.
The sketch never underestimates frequencies, making it ideal for heavy hitter detection.

Applications span multiple domains: network monitoring uses these structures for
traffic analysis, databases employ them for query optimization, and streaming platforms
leverage them for real-time analytics. The efficiency gains enable processing at
line rate for 100Gbps networks and beyond.
`;

const wordProcessor = new StreamProcessor(0.01, 0.01);
const wordCounts = new Map<string, number>();

// Process text
const words = textCorpus.toLowerCase()
  .replace(/[^a-z\s]/g, '')
  .split(/\s+/)
  .filter(w => w.length > 3); // Filter short words

console.log(`Processing ${words.length} words...`);

for (const word of words) {
  wordProcessor.processWord(word);
  wordCounts.set(word, (wordCounts.get(word) || 0) + 1);
}

console.log('✓ Processing complete\n');

console.log('WORD FREQUENCY (Top 15):');
console.log('-----------------------');

const sortedWords = Array.from(wordCounts.entries())
  .sort((a, b) => b[1] - a[1])
  .slice(0, 15);

for (const [word, actualCount] of sortedWords) {
  const estimated = wordProcessor.getFrequency(word);
  const bar = '█'.repeat(actualCount);
  console.log(`${word.padEnd(20)} ${estimated.toString().padStart(2)} ${bar}`);
}

// ============================================================================
// Performance Characteristics
// ============================================================================

console.log('\n\n=== CountMinSketch Performance ===\n');

console.log('Space Complexity:');
console.log('  Formula: O(1/ε × log(1/δ))');
console.log('  ε=0.01, δ=0.01: ~460 counters = ~1.8 KB');
console.log('  ε=0.001, δ=0.01: ~4600 counters = ~18 KB');
console.log('  ε=0.0001, δ=0.01: ~46000 counters = ~180 KB\n');

console.log('Time Complexity:');
console.log('  Update: O(log(1/δ)) - typically 5-7 hash operations');
console.log('  Query: O(log(1/δ)) - same as update');
console.log('  Both are constant time in practice\n');

console.log('Accuracy Guarantees:');
console.log('  Never underestimates (estimate ≥ true count)');
console.log('  Overestimation bounded: estimate ≤ true + ε × N');
console.log('  Where N = total number of items processed');
console.log('  Confidence: 1 - δ (e.g., 99% for δ=0.01)\n');

console.log('When to Use CountMinSketch:');
console.log('  ✓ Heavy hitter detection (top-K frequent items)');
console.log('  ✓ Frequency estimation in streams');
console.log('  ✓ When you can tolerate overestimation but not underestimation');
console.log('  ✓ Space-constrained environments');
console.log('  ✓ Need for merge across distributed systems\n');

console.log('Production Use Cases:');
console.log('  • Network traffic monitoring (packet/flow counting)');
console.log('  • Log analysis (error frequency, service load)');
console.log('  • Text processing (word frequency, n-grams)');
console.log('  • Trending topics detection (social media)');
console.log('  • Database query optimization (selectivity estimation)');
console.log('  • Ad impression frequency capping\n');

console.log('Comparison with Alternatives:');
console.log('  vs Exact Counting: 100x+ space savings');
console.log('  vs HeavyKeeper: CMS simpler, HK better for top-k');
console.log('  vs Count Sketch: CMS never underestimates');
console.log('  vs Sampling: CMS provides deterministic bounds\n');

// ============================================================================
// Advanced: Distributed Merging
// ============================================================================

console.log('=== Distributed Processing Example ===\n');

// Simulate 3 worker nodes
const worker1 = new CountMinSketch(0.01, 0.01);
const worker2 = new CountMinSketch(0.01, 0.01);
const worker3 = new CountMinSketch(0.01, 0.01);

// Split words across workers
const wordsPerWorker = Math.floor(words.length / 3);
words.slice(0, wordsPerWorker).forEach(w => worker1.update(Buffer.from(w)));
words.slice(wordsPerWorker, wordsPerWorker * 2).forEach(w => worker2.update(Buffer.from(w)));
words.slice(wordsPerWorker * 2).forEach(w => worker3.update(Buffer.from(w)));

console.log('Processing across 3 worker nodes...');
console.log(`Worker 1: ${wordsPerWorker} words`);
console.log(`Worker 2: ${wordsPerWorker} words`);
console.log(`Worker 3: ${words.length - wordsPerWorker * 2} words`);

// Merge into worker1
worker1.merge(worker2);
worker1.merge(worker3);

console.log('\n✓ Merged results from all workers');

// Verify merged results
const testWords = ['structures', 'sketch', 'frequency'];
console.log('\nVerifying merged counts:');
for (const word of testWords) {
  const mergedCount = worker1.estimate(Buffer.from(word));
  const actualCount = wordCounts.get(word) || 0;
  console.log(`  ${word}: merged=${mergedCount}, actual=${actualCount}`);
}

console.log('\n=== Example Complete ===\n');
