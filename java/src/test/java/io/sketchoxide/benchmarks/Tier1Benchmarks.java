package io.sketchoxide.benchmarks;

import com.sketches_oxide.frequency.HeavyKeeper;
import com.sketches_oxide.reconciliation.RatelessIBLT;
import com.sketches_oxide.range_filters.Grafite;
import com.sketches_oxide.range_filters.MementoFilter;
import com.sketches_oxide.streaming.SlidingHyperLogLog;
import org.openjdk.jmh.annotations.*;
import java.util.concurrent.TimeUnit;

@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
@Warmup(iterations = 3, time = 1)
@Measurement(iterations = 5, time = 1)
@Fork(1)
@State(Scope.Benchmark)
public class Tier1Benchmarks {

    private HeavyKeeper heavyKeeper;
    private RatelessIBLT ratelessIBLT;
    private Grafite grafite;
    private MementoFilter mementoFilter;
    private SlidingHyperLogLog slidingHLL;

    private byte[] testItem = "benchmark_item".getBytes();
    private long timestamp = 1000;

    @Setup
    public void setup() {
        heavyKeeper = new HeavyKeeper(100, 0.001, 0.01);
        ratelessIBLT = new RatelessIBLT(100, 32);

        long[] keys = new long[100];
        for (int i = 0; i < 100; i++) keys[i] = i * 10;
        grafite = Grafite.build(keys, 6);

        mementoFilter = new MementoFilter(1000, 0.01);
        slidingHLL = new SlidingHyperLogLog(12, 3600);
    }

    @TearDown
    public void teardown() {
        if (heavyKeeper != null) heavyKeeper.close();
        if (ratelessIBLT != null) ratelessIBLT.close();
        if (grafite != null) grafite.close();
        if (mementoFilter != null) mementoFilter.close();
        if (slidingHLL != null) slidingHLL.close();
    }

    @Benchmark
    public void heavyKeeperUpdate() {
        heavyKeeper.update(testItem);
    }

    @Benchmark
    public int heavyKeeperEstimate() {
        return heavyKeeper.estimate(testItem);
    }

    @Benchmark
    public void ratelessIBLTInsert() {
        ratelessIBLT.insert(testItem, testItem);
    }

    @Benchmark
    public boolean grafiteMayContain() {
        return grafite.mayContain(500);
    }

    @Benchmark
    public boolean grafiteMayContainRange() {
        return grafite.mayContainRange(100, 200);
    }

    @Benchmark
    public void mementoFilterInsert() {
        mementoFilter.insert(timestamp++, testItem);
    }

    @Benchmark
    public boolean mementoFilterQuery() {
        return mementoFilter.mayContainRange(100, 200);
    }

    @Benchmark
    public void slidingHLLUpdate() {
        slidingHLL.update(testItem, timestamp++);
    }

    @Benchmark
    public double slidingHLLEstimateWindow() {
        return slidingHLL.estimateWindow(timestamp, 600);
    }

    @Benchmark
    public double slidingHLLEstimateTotal() {
        return slidingHLL.estimateTotal();
    }
}
