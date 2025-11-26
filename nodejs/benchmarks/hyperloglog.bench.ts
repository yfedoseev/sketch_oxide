/**
 * HyperLogLog Benchmark - SketchOxide vs npm packages
 *
 * Compares performance against:
 * - hyperloglog npm package
 * - Other Node.js implementations
 *
 * Run with: npm run benchmark -- hyperloglog
 */

import { HyperLogLog as SketchOxideHLL } from '../src';
import HyperLogLog from 'hyperloglog'; // npm package for comparison
import { performance } from 'perf_hooks';
import * as crypto from 'crypto';

interface BenchmarkResult {
    operation: string;
    sketchOxide: number;
    competitor: number;
    unit: string;
    difference: string;
}

class HyperLogLogBenchmark {
    private precision = 14;
    private iterations = 1000;
    private testData = Buffer.alloc(1024);
    private results: BenchmarkResult[] = [];

    constructor() {
        crypto.randomFillSync(this.testData);
    }

    /**
     * Benchmark: Single update operation
     */
    benchmarkSingleUpdate(): BenchmarkResult {
        const iterations = 10000;
        let randomBytes = Buffer.alloc(1024);

        // SketchOxide
        const soStart = performance.now();
        const soHLL = new SketchOxideHLL(this.precision);
        for (let i = 0; i < iterations; i++) {
            crypto.randomFillSync(randomBytes);
            soHLL.update(randomBytes);
        }
        const soTime = (performance.now() - soStart) / iterations;

        // Competitor
        const compStart = performance.now();
        const compHLL = new HyperLogLog(0.01); // 1% accuracy
        for (let i = 0; i < iterations; i++) {
            crypto.randomFillSync(randomBytes);
            compHLL.add(randomBytes.toString());
        }
        const compTime = (performance.now() - compStart) / iterations;

        return {
            operation: 'Single update (µs)',
            sketchOxide: soTime,
            competitor: compTime,
            unit: 'microseconds',
            difference: this.percentDiff(soTime, compTime)
        };
    }

    /**
     * Benchmark: Cardinality estimation
     */
    benchmarkEstimate(): BenchmarkResult {
        const soHLL = new SketchOxideHLL(this.precision);
        const compHLL = new HyperLogLog(0.01);

        // Populate sketches
        for (let i = 0; i < 10000; i++) {
            const data = Buffer.from(`item_${i}`);
            soHLL.update(data);
            compHLL.add(`item_${i}`);
        }

        // SketchOxide
        const soStart = performance.now();
        for (let i = 0; i < 100000; i++) {
            soHLL.estimate();
        }
        const soTime = (performance.now() - soStart) / 100000;

        // Competitor
        const compStart = performance.now();
        for (let i = 0; i < 100000; i++) {
            compHLL.count();
        }
        const compTime = (performance.now() - compStart) / 100000;

        return {
            operation: 'Estimate (ns)',
            sketchOxide: soTime * 1000, // Convert to nanoseconds
            competitor: compTime * 1000,
            unit: 'nanoseconds',
            difference: this.percentDiff(soTime, compTime)
        };
    }

    /**
     * Benchmark: Serialization
     */
    benchmarkSerialization(): BenchmarkResult {
        const soHLL = new SketchOxideHLL(this.precision);
        const compHLL = new HyperLogLog(0.01);

        // Populate
        for (let i = 0; i < 100000; i++) {
            soHLL.update(Buffer.from(`item_${i}`));
            compHLL.add(`item_${i}`);
        }

        // SketchOxide
        const soStart = performance.now();
        for (let i = 0; i < 1000; i++) {
            soHLL.serialize();
        }
        const soTime = (performance.now() - soStart) / 1000;

        // Competitor
        const compStart = performance.now();
        for (let i = 0; i < 1000; i++) {
            compHLL.toString();
        }
        const compTime = (performance.now() - compStart) / 1000;

        return {
            operation: 'Serialize (µs)',
            sketchOxide: soTime,
            competitor: compTime,
            unit: 'microseconds',
            difference: this.percentDiff(soTime, compTime)
        };
    }

    /**
     * Benchmark: Merge operations
     */
    benchmarkMerge(): BenchmarkResult {
        // SketchOxide merge
        const soStart = performance.now();
        for (let i = 0; i < 1000; i++) {
            const hll1 = new SketchOxideHLL(this.precision);
            const hll2 = new SketchOxideHLL(this.precision);
            hll1.update(Buffer.from('data1'));
            hll2.update(Buffer.from('data2'));
            hll1.merge(hll2);
        }
        const soTime = (performance.now() - soStart) / 1000;

        // Competitor merge
        const compStart = performance.now();
        for (let i = 0; i < 1000; i++) {
            const hll1 = new HyperLogLog(0.01);
            const hll2 = new HyperLogLog(0.01);
            hll1.add('data1');
            hll2.add('data2');
            // Note: hyperloglog npm doesn't support merge, so this is just creation time
        }
        const compTime = (performance.now() - compStart) / 1000;

        return {
            operation: 'Merge (µs)',
            sketchOxide: soTime,
            competitor: compTime,
            unit: 'microseconds',
            difference: 'N/A - no merge support in competitor'
        };
    }

    /**
     * Benchmark: Memory footprint
     */
    benchmarkMemory(): BenchmarkResult {
        const soHLL = new SketchOxideHLL(this.precision);
        const soSerialized = soHLL.serialize();

        const compHLL = new HyperLogLog(0.01);
        const compSerialized = compHLL.toString().length;

        return {
            operation: 'Memory (bytes)',
            sketchOxide: soSerialized.length,
            competitor: compSerialized,
            unit: 'bytes',
            difference: this.percentDiff(soSerialized.length, compSerialized)
        };
    }

    /**
     * Helper: Calculate percentage difference
     */
    private percentDiff(val1: number, val2: number): string {
        if (val2 === 0) return 'Infinite';
        const pct = ((val1 - val2) / val2) * 100;
        const winner = pct < 0 ? '✓ SO faster' : '✗ Slower';
        return `${pct.toFixed(1)}% ${winner}`;
    }

    /**
     * Run all benchmarks
     */
    async run(): Promise<void> {
        console.log('=== Node.js HyperLogLog Benchmark ===\n');
        console.log('SketchOxide vs hyperloglog npm package\n');

        this.results.push(this.benchmarkSingleUpdate());
        this.results.push(this.benchmarkEstimate());
        this.results.push(this.benchmarkSerialization());
        this.results.push(this.benchmarkMerge());
        this.results.push(this.benchmarkMemory());

        this.printResults();
    }

    /**
     * Print results in table format
     */
    private printResults(): void {
        console.log(
            'Operation'.padEnd(20),
            'SketchOxide'.padEnd(15),
            'Competitor'.padEnd(15),
            'Difference'
        );
        console.log('-'.repeat(65));

        for (const result of this.results) {
            console.log(
                result.operation.padEnd(20),
                `${result.sketchOxide.toFixed(2)}`.padEnd(15),
                `${result.competitor.toFixed(2)}`.padEnd(15),
                result.difference.padEnd(20)
            );
        }

        console.log('\nNote: Lower times are better. Times in µs (microseconds) or ns (nanoseconds)');
    }
}

// Run benchmark
if (require.main === module) {
    const bench = new HyperLogLogBenchmark();
    bench.run().catch(console.error);
}

export { HyperLogLogBenchmark };
