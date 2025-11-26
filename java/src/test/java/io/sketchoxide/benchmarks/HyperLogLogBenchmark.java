package io.sketchoxide.benchmarks;

import io.sketchoxide.cardinality.HyperLogLog;
import org.apache.datasketches.hll.HllSketch;
import org.apache.datasketches.hll.TgtHllType;
import org.openjdk.jmh.annotations.*;
import org.openjdk.jmh.infra.Blackhole;

import java.util.Random;
import java.util.concurrent.TimeUnit;

/**
 * Benchmark comparing SketchOxide HyperLogLog with Apache DataSketches HyperLogLog.
 *
 * Run with: mvn clean test -Dtest=HyperLogLogBenchmark -DargLine="-XX:+UnlockDiagnosticVMOptions -XX:+TraceClassLoading"
 * Or: mvn jmh:benchmark -Djmh.mainClass=io.sketchoxide.benchmarks.HyperLogLogBenchmark
 */
@State(Scope.Thread)
@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.MICROSECONDS)
@Warmup(iterations = 5, time = 1, timeUnit = TimeUnit.SECONDS)
@Measurement(iterations = 10, time = 2, timeUnit = TimeUnit.SECONDS)
@Fork(value = 2, jvmArgs = {"-Xms1g", "-Xmx1g"})
public class HyperLogLogBenchmark {

    // Precision level - used by both libraries for fair comparison
    private static final int PRECISION = 14;

    @State(Scope.Benchmark)
    public static class BenchmarkData {
        public byte[] testData;
        public HyperLogLog sketchOxideHll;
        public HllSketch apacheDataSketchesHll;
        public Random random;

        @Setup(Level.Trial)
        public void setup() {
            random = new Random(12345L);
            testData = new byte[1024]; // 1KB test data

            // Initialize SketchOxide HyperLogLog
            try {
                sketchOxideHll = new HyperLogLog(PRECISION);
            } catch (Exception e) {
                throw new RuntimeException(e);
            }

            // Initialize Apache DataSketches HyperLogLog
            apacheDataSketchesHll = new HllSketch(PRECISION, TgtHllType.HLL_8);
        }
    }

    @Benchmark
    @Description("SketchOxide HyperLogLog - single update")
    public void sketchOxideHllUpdate(BenchmarkData data, Blackhole bh) {
        data.random.nextBytes(data.testData);
        data.sketchOxideHll.update(data.testData);
        bh.consume(data.sketchOxideHll);
    }

    @Benchmark
    @Description("Apache DataSketches HyperLogLog - single update")
    public void apacheDataSketchesHllUpdate(BenchmarkData data, Blackhole bh) {
        data.random.nextBytes(data.testData);
        data.apacheDataSketchesHll.update(data.testData);
        bh.consume(data.apacheDataSketchesHll);
    }

    @Benchmark
    @Description("SketchOxide HyperLogLog - estimate cardinality")
    public double sketchOxideHllEstimate(BenchmarkData data) {
        return data.sketchOxideHll.estimate();
    }

    @Benchmark
    @Description("Apache DataSketches HyperLogLog - estimate cardinality")
    public double apacheDataSketchesHllEstimate(BenchmarkData data) {
        return data.apacheDataSketchesHll.getEstimate();
    }

    @Benchmark
    @Description("SketchOxide HyperLogLog - serialization")
    public byte[] sketchOxideHllSerialize(BenchmarkData data) {
        return data.sketchOxideHll.serialize();
    }

    @Benchmark
    @Description("Apache DataSketches HyperLogLog - serialization")
    public byte[] apacheDataSketchesHllSerialize(BenchmarkData data) {
        return data.apacheDataSketchesHll.toCompactByteArray();
    }

    @Benchmark
    @Description("SketchOxide HyperLogLog - bulk insert (100 items)")
    public void sketchOxideHllBulkInsert(BenchmarkData data, Blackhole bh) {
        for (int i = 0; i < 100; i++) {
            data.random.nextBytes(data.testData);
            data.sketchOxideHll.update(data.testData);
        }
        bh.consume(data.sketchOxideHll.estimate());
    }

    @Benchmark
    @Description("Apache DataSketches HyperLogLog - bulk insert (100 items)")
    public void apacheDataSketchesHllBulkInsert(BenchmarkData data, Blackhole bh) {
        for (int i = 0; i < 100; i++) {
            data.random.nextBytes(data.testData);
            data.apacheDataSketchesHll.update(data.testData);
        }
        bh.consume(data.apacheDataSketchesHll.getEstimate());
    }

    @Benchmark
    @Description("SketchOxide HyperLogLog - memory footprint (precision 14)")
    public int sketchOxideHllMemory(BenchmarkData data, Blackhole bh) throws Exception {
        HyperLogLog hll = new HyperLogLog(PRECISION);
        byte[] serialized = hll.serialize();
        bh.consume(hll);
        return serialized.length;
    }

    @Benchmark
    @Description("Apache DataSketches HyperLogLog - memory footprint (precision 14)")
    public int apacheDataSketchesHllMemory(BenchmarkData data, Blackhole bh) {
        HllSketch hll = new HllSketch(PRECISION, TgtHllType.HLL_8);
        byte[] serialized = hll.toCompactByteArray();
        bh.consume(hll);
        return serialized.length;
    }

    @Benchmark
    @Description("SketchOxide HyperLogLog - deserialization")
    public HyperLogLog sketchOxideHllDeserialize(BenchmarkData data) throws Exception {
        byte[] serialized = data.sketchOxideHll.serialize();
        return HyperLogLog.deserialize(serialized);
    }

    @Benchmark
    @Description("Apache DataSketches HyperLogLog - deserialization")
    public HllSketch apacheDataSketchesHllDeserialize(BenchmarkData data) {
        byte[] serialized = data.apacheDataSketchesHll.toCompactByteArray();
        return HllSketch.heapify(serialized);
    }

    @Benchmark
    @Description("SketchOxide HyperLogLog - merge operation")
    public HyperLogLog sketchOxideHllMerge(BenchmarkData data) throws Exception {
        HyperLogLog hll1 = new HyperLogLog(PRECISION);
        HyperLogLog hll2 = new HyperLogLog(PRECISION);
        hll1.update("data1".getBytes());
        hll2.update("data2".getBytes());
        hll1.merge(hll2);
        return hll1;
    }

    @Benchmark
    @Description("Apache DataSketches HyperLogLog - merge operation")
    public HllSketch apacheDataSketchesHllMerge(BenchmarkData data) {
        HllSketch hll1 = new HllSketch(PRECISION, TgtHllType.HLL_8);
        HllSketch hll2 = new HllSketch(PRECISION, TgtHllType.HLL_8);
        hll1.update("data1".getBytes());
        hll2.update("data2".getBytes());
        hll1.union(hll2);
        return hll1;
    }
}
