/**
 * Benchmarks for Tier 2 Sketches
 *
 * Performance benchmarks for:
 * - VacuumFilter (dynamic membership with deletions)
 * - GRF (Gorilla Range Filter for LSM-trees)
 * - NitroSketch (high-speed network telemetry)
 * - UnivMon (universal multi-metric sketch)
 * - LearnedBloomFilter (ML-enhanced membership)
 */

import {
  VacuumFilter,
  GRF,
  NitroSketch,
  UnivMon,
  LearnedBloomFilter,
  CountMinSketch,
} from '../index'

// ============================================================================
// VACUUMFILTER BENCHMARKS
// ============================================================================

console.log('='.repeat(80))
console.log('VACUUMFILTER BENCHMARKS')
console.log('='.repeat(80))

{
  const filter = new VacuumFilter(10000, 0.01)
  const key = Buffer.from('test_key')

  // Benchmark: insert
  const insertStart = Date.now()
  for (let i = 0; i < 10000; i++) {
    filter.insert(Buffer.from(`key${i}`))
  }
  const insertTime = Date.now() - insertStart
  console.log(
    `VacuumFilter#insert: ${insertTime}ms for 10K ops (${(10000 / insertTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: contains
  const containsStart = Date.now()
  for (let i = 0; i < 100000; i++) {
    filter.contains(key)
  }
  const containsTime = Date.now() - containsStart
  console.log(
    `VacuumFilter#contains: ${containsTime}ms for 100K ops (${(100000 / containsTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: delete
  const deleteStart = Date.now()
  for (let i = 0; i < 5000; i++) {
    filter.delete(Buffer.from(`key${i}`))
  }
  const deleteTime = Date.now() - deleteStart
  console.log(
    `VacuumFilter#delete: ${deleteTime}ms for 5K ops (${(5000 / deleteTime * 1000).toFixed(0)} ops/sec)`,
  )

  const stats = filter.stats()
  console.log(`  Load factor: ${stats.loadFactor.toFixed(3)}`)
  console.log(`  Memory: ${stats.memoryBits} bits (${Math.round(Number(stats.memoryBits) / 8)} bytes)`)
}

{
  // Benchmark: Mixed workload
  console.log('\nVacuumFilter - Mixed Workload (insert/contains/delete):')
  const filter = new VacuumFilter(10000, 0.01)

  const mixedStart = Date.now()
  for (let i = 0; i < 10000; i++) {
    const key = Buffer.from(`key${i % 1000}`)
    filter.insert(key)
    filter.contains(key)
    if (i % 10 === 0) {
      filter.delete(key)
    }
  }
  const mixedTime = Date.now() - mixedStart
  console.log(`  Mixed: ${mixedTime}ms for 10K ops (${(10000 / mixedTime * 1000).toFixed(0)} ops/sec)`)
}

// ============================================================================
// GRF BENCHMARKS
// ============================================================================

console.log('\n' + '='.repeat(80))
console.log('GRF BENCHMARKS')
console.log('='.repeat(80))

{
  // Benchmark: build
  const keys = Array.from({ length: 10000 }, (_, i) => BigInt(i * 10))
  const buildStart = Date.now()
  const grf = GRF.build(keys, 6)
  const buildTime = Date.now() - buildStart
  console.log(`GRF#build: ${buildTime}ms for 10K keys`)

  // Benchmark: mayContainRange
  const rangeQueryStart = Date.now()
  for (let i = 0; i < 100000; i++) {
    grf.mayContainRange(100n, 200n)
  }
  const rangeQueryTime = Date.now() - rangeQueryStart
  console.log(
    `GRF#mayContainRange: ${rangeQueryTime}ms for 100K ops (${(100000 / rangeQueryTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: mayContain (point query)
  const pointQueryStart = Date.now()
  for (let i = 0; i < 100000; i++) {
    grf.mayContain(500n)
  }
  const pointQueryTime = Date.now() - pointQueryStart
  console.log(
    `GRF#mayContain: ${pointQueryTime}ms for 100K ops (${(100000 / pointQueryTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: expectedFpr
  const fprStart = Date.now()
  for (let i = 0; i < 10000; i++) {
    grf.expectedFpr(10n)
  }
  const fprTime = Date.now() - fprStart
  console.log(
    `GRF#expectedFpr: ${fprTime}ms for 10K ops (${(10000 / fprTime * 1000).toFixed(0)} ops/sec)`,
  )

  const stats = grf.stats()
  console.log(`  Keys: ${stats.keyCount}, Segments: ${stats.segmentCount}`)
  console.log(`  Memory: ${stats.totalBits} bits (${stats.memoryBytes} bytes)`)
}

{
  // Benchmark: Skewed distribution (Zipf)
  console.log('\nGRF - Skewed Distribution (Zipf):')
  const keys: bigint[] = []
  for (let i = 1; i <= 1000; i++) {
    const count = Math.floor(1000 / i)
    for (let j = 0; j < count; j++) {
      keys.push(BigInt(i))
    }
  }
  keys.sort((a, b) => (a < b ? -1 : 1))

  const zipfBuildStart = Date.now()
  const zipfGrf = GRF.build(keys, 6)
  const zipfBuildTime = Date.now() - zipfBuildStart
  console.log(`  Build (Zipf): ${zipfBuildTime}ms for ${keys.length} keys`)

  // Query performance on skewed data
  const zipfQueryStart = Date.now()
  for (let i = 0; i < 10000; i++) {
    zipfGrf.mayContainRange(1n, 100n)
  }
  const zipfQueryTime = Date.now() - zipfQueryStart
  console.log(
    `  Range query (Zipf): ${zipfQueryTime}ms for 10K ops (${(10000 / zipfQueryTime * 1000).toFixed(0)} ops/sec)`,
  )
}

// ============================================================================
// NITROSKETCH BENCHMARKS
// ============================================================================

console.log('\n' + '='.repeat(80))
console.log('NITROSKETCH BENCHMARKS')
console.log('='.repeat(80))

{
  const base = new CountMinSketch(0.01, 0.01)
  const nitro = new NitroSketch(base, 0.1)
  const key = Buffer.from('test_key')

  // Benchmark: updateSampled (10% sampling)
  const updateStart = Date.now()
  for (let i = 0; i < 100000; i++) {
    nitro.updateSampled(key)
  }
  const updateTime = Date.now() - updateStart
  console.log(
    `NitroSketch#updateSampled (10%): ${updateTime}ms for 100K ops (${(100000 / updateTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: query
  const queryStart = Date.now()
  for (let i = 0; i < 100000; i++) {
    nitro.query(key)
  }
  const queryTime = Date.now() - queryStart
  console.log(
    `NitroSketch#query: ${queryTime}ms for 100K ops (${(100000 / queryTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: sync
  const syncStart = Date.now()
  for (let i = 0; i < 1000; i++) {
    nitro.sync(1.0)
  }
  const syncTime = Date.now() - syncStart
  console.log(
    `NitroSketch#sync: ${syncTime}ms for 1K ops (${(1000 / syncTime * 1000).toFixed(0)} ops/sec)`,
  )

  const stats = nitro.stats()
  console.log(`  Sample rate: ${stats.sampleRate}`)
  console.log(`  Sampled: ${stats.sampledCount}, Unsampled: ${stats.unsampledCount}`)
}

{
  // Compare sampling rates
  console.log('\nNitroSketch - Sampling Rate Comparison:')
  const sampleRates = [1.0, 0.5, 0.1, 0.01]

  for (const rate of sampleRates) {
    const base = new CountMinSketch(0.01, 0.01)
    const nitro = new NitroSketch(base, rate)

    const start = Date.now()
    for (let i = 0; i < 100000; i++) {
      nitro.updateSampled(Buffer.from(`key${i % 1000}`))
    }
    const time = Date.now() - start

    const stats = nitro.stats()
    console.log(
      `  Rate ${rate.toFixed(2)}: ${time}ms for 100K ops (${(100000 / time * 1000).toFixed(0)} ops/sec), sampled ${stats.sampledCount}`,
    )
  }
}

// ============================================================================
// UNIVMON BENCHMARKS
// ============================================================================

console.log('\n' + '='.repeat(80))
console.log('UNIVMON BENCHMARKS')
console.log('='.repeat(80))

{
  const univmon = new UnivMon(100000n, 0.01, 0.01)
  const key = Buffer.from('test_key')

  // Benchmark: update
  const updateStart = Date.now()
  for (let i = 0; i < 10000; i++) {
    univmon.update(Buffer.from(`key${i % 100}`), i + 1)
  }
  const updateTime = Date.now() - updateStart
  console.log(
    `UnivMon#update: ${updateTime}ms for 10K ops (${(10000 / updateTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: estimateL1
  const l1Start = Date.now()
  for (let i = 0; i < 10000; i++) {
    univmon.estimateL1()
  }
  const l1Time = Date.now() - l1Start
  console.log(
    `UnivMon#estimateL1: ${l1Time}ms for 10K ops (${(10000 / l1Time * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: estimateL2
  const l2Start = Date.now()
  for (let i = 0; i < 10000; i++) {
    univmon.estimateL2()
  }
  const l2Time = Date.now() - l2Start
  console.log(
    `UnivMon#estimateL2: ${l2Time}ms for 10K ops (${(10000 / l2Time * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: estimateEntropy
  const entropyStart = Date.now()
  for (let i = 0; i < 10000; i++) {
    univmon.estimateEntropy()
  }
  const entropyTime = Date.now() - entropyStart
  console.log(
    `UnivMon#estimateEntropy: ${entropyTime}ms for 10K ops (${(10000 / entropyTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: heavyHitters
  const hhStart = Date.now()
  for (let i = 0; i < 1000; i++) {
    univmon.heavyHitters(0.1)
  }
  const hhTime = Date.now() - hhStart
  console.log(
    `UnivMon#heavyHitters: ${hhTime}ms for 1K ops (${(1000 / hhTime * 1000).toFixed(0)} ops/sec)`,
  )

  console.log(`  L1: ${univmon.estimateL1().toFixed(0)}`)
  console.log(`  L2: ${univmon.estimateL2().toFixed(0)}`)
  console.log(`  Entropy: ${univmon.estimateEntropy().toFixed(2)}`)
}

{
  // Multi-metric scenario
  console.log('\nUnivMon - Multi-Metric Scenario (6 metrics from ONE sketch):')
  const univmon = new UnivMon(100000n, 0.01, 0.01)

  // Simulate network traffic
  const ips = ['192.168.1.1', '192.168.1.2', '192.168.1.3', '10.0.0.1', '10.0.0.2']

  const multiStart = Date.now()
  for (let i = 0; i < 10000; i++) {
    const ip = ips[i % ips.length]
    const packetSize = 500 + Math.floor(Math.random() * 1500)
    univmon.update(Buffer.from(ip), packetSize)
  }

  // Query all metrics
  const l1 = univmon.estimateL1()
  const l2 = univmon.estimateL2()
  const entropy = univmon.estimateEntropy()
  const heavyHitters = univmon.heavyHitters(0.1)

  const multiTime = Date.now() - multiStart
  console.log(`  Total time: ${multiTime}ms for 10K updates + 4 queries`)
  console.log(`  Total traffic: ${l1.toFixed(0)} bytes`)
  console.log(`  Load balance (L2): ${l2.toFixed(0)}`)
  console.log(`  IP diversity (entropy): ${entropy.toFixed(2)}`)
  console.log(`  Heavy hitters: ${heavyHitters.length}`)
}

// ============================================================================
// LEARNEDBLOOMFILTER BENCHMARKS
// ============================================================================

console.log('\n' + '='.repeat(80))
console.log('LEARNEDBLOOMFILTER BENCHMARKS')
console.log('='.repeat(80))

{
  // Benchmark: training
  const keys: Buffer[] = []
  for (let i = 0; i < 10000; i++) {
    keys.push(Buffer.from(`key${i}`))
  }

  const trainStart = Date.now()
  const filter = LearnedBloomFilter.new(keys, 0.01)
  const trainTime = Date.now() - trainStart
  console.log(`LearnedBloomFilter#new (training): ${trainTime}ms for 10K keys`)

  // Benchmark: contains (positive queries)
  const containsPositiveStart = Date.now()
  for (let i = 0; i < 100000; i++) {
    filter.contains(Buffer.from(`key${i % 10000}`))
  }
  const containsPositiveTime = Date.now() - containsPositiveStart
  console.log(
    `LearnedBloomFilter#contains (positive): ${containsPositiveTime}ms for 100K ops (${(100000 / containsPositiveTime * 1000).toFixed(0)} ops/sec)`,
  )

  // Benchmark: contains (negative queries)
  const containsNegativeStart = Date.now()
  for (let i = 0; i < 100000; i++) {
    filter.contains(Buffer.from(`nonexistent${i}`))
  }
  const containsNegativeTime = Date.now() - containsNegativeStart
  console.log(
    `LearnedBloomFilter#contains (negative): ${containsNegativeTime}ms for 100K ops (${(100000 / containsNegativeTime * 1000).toFixed(0)} ops/sec)`,
  )

  const mem = filter.memoryUsage()
  console.log(`  Memory: ${mem} bytes (${(mem * 8 / 10000).toFixed(2)} bits/key)`)
  console.log(`  Memory savings: ~${((1 - mem * 8 / 10000 / 10) * 100).toFixed(0)}% vs standard Bloom`)
}

{
  // Compare sizes
  console.log('\nLearnedBloomFilter - Size Comparison:')
  const sizes = [1000, 5000, 10000, 50000]

  for (const size of sizes) {
    const keys: Buffer[] = []
    for (let i = 0; i < size; i++) {
      keys.push(Buffer.from(`key${i}`))
    }

    const start = Date.now()
    const filter = LearnedBloomFilter.new(keys, 0.01)
    const time = Date.now() - start

    const mem = filter.memoryUsage()
    const bitsPerKey = (mem * 8 / size).toFixed(2)

    console.log(
      `  ${size} keys: ${time}ms training, ${mem} bytes (${bitsPerKey} bits/key)`,
    )
  }
}

// ============================================================================
// CROSS-SKETCH COMPARISON
// ============================================================================

console.log('\n' + '='.repeat(80))
console.log('CROSS-SKETCH COMPARISON')
console.log('='.repeat(80))

{
  console.log('\nMembership Testing (10K inserts, 100K queries):')

  // VacuumFilter
  {
    const filter = new VacuumFilter(10000, 0.01)
    const start = Date.now()
    for (let i = 0; i < 10000; i++) {
      filter.insert(Buffer.from(`key${i}`))
    }
    for (let i = 0; i < 100000; i++) {
      filter.contains(Buffer.from(`key${i % 10000}`))
    }
    const time = Date.now() - start
    const mem = filter.memoryUsage()
    console.log(`  VacuumFilter: ${time}ms, ${mem} bytes`)
  }

  // LearnedBloomFilter
  {
    const keys: Buffer[] = []
    for (let i = 0; i < 10000; i++) {
      keys.push(Buffer.from(`key${i}`))
    }
    const start = Date.now()
    const filter = LearnedBloomFilter.new(keys, 0.01)
    for (let i = 0; i < 100000; i++) {
      filter.contains(Buffer.from(`key${i % 10000}`))
    }
    const time = Date.now() - start
    const mem = filter.memoryUsage()
    console.log(`  LearnedBloomFilter: ${time}ms (incl. training), ${mem} bytes`)
  }
}

console.log('\n' + '='.repeat(80))
console.log('BENCHMARKS COMPLETE')
console.log('='.repeat(80))
