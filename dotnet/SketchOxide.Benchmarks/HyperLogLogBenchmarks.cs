using BenchmarkDotNet.Attributes;
using BenchmarkDotNet.Running;
using SketchOxide.Cardinality;
using System;
using System.Diagnostics;

namespace SketchOxide.Benchmarks
{
    /// <summary>
    /// Benchmark comparing SketchOxide HyperLogLog with alternative C# implementations.
    ///
    /// Note: For fair comparison, ideally also include:
    /// - Probably (https://www.nuget.org/packages/Probably/)
    /// - BloomFilter.NetCore
    ///
    /// Run with: dotnet run -c Release --project SketchOxide.Benchmarks
    /// Or:       dotnet run --project SketchOxide.Benchmarks -- --job short
    /// </summary>
    [SimpleJob(warmupCount: 5, targetCount: 10)]
    [MemoryDiagnoser]
    [PlainExporter]
    public class HyperLogLogBenchmarks
    {
        private const int Precision = 14;
        private byte[] _testData = new byte[1024];
        private HyperLogLog _sketchOxideHll;
        private Random _random;

        [GlobalSetup]
        public void Setup()
        {
            _random = new Random(12345);
            _random.NextBytes(_testData);
            _sketchOxideHll = new HyperLogLog(Precision);
        }

        [Benchmark(Description = "SketchOxide HyperLogLog - Single Update")]
        public void SingleUpdate()
        {
            _random.NextBytes(_testData);
            _sketchOxideHll.Update(_testData);
        }

        [Benchmark(Description = "SketchOxide HyperLogLog - Estimate")]
        public double Estimate()
        {
            return _sketchOxideHll.Estimate();
        }

        [Benchmark(Description = "SketchOxide HyperLogLog - Serialization")]
        public byte[] Serialize()
        {
            return _sketchOxideHll.Serialize();
        }

        [Benchmark(Description = "SketchOxide HyperLogLog - Deserialization")]
        public HyperLogLog Deserialize()
        {
            byte[] serialized = _sketchOxideHll.Serialize();
            return HyperLogLog.Deserialize(serialized);
        }

        [Benchmark(Description = "SketchOxide HyperLogLog - Bulk Insert (100)")]
        public void BulkInsert()
        {
            for (int i = 0; i < 100; i++)
            {
                _random.NextBytes(_testData);
                _sketchOxideHll.Update(_testData);
            }
        }

        [Benchmark(Description = "SketchOxide HyperLogLog - Merge")]
        public void Merge()
        {
            using (var hll1 = new HyperLogLog(Precision))
            using (var hll2 = new HyperLogLog(Precision))
            {
                hll1.Update("data1"u8);
                hll2.Update("data2"u8);
                hll1.Merge(hll2);
            }
        }

        [Benchmark(Description = "SketchOxide HyperLogLog - Memory Footprint")]
        public int MemoryFootprint()
        {
            using (var hll = new HyperLogLog(Precision))
            {
                byte[] serialized = hll.Serialize();
                return serialized.Length;
            }
        }

        [Benchmark(Description = "SketchOxide HyperLogLog - Accuracy Test")]
        public double AccuracyTest()
        {
            using (var hll = new HyperLogLog(Precision))
            {
                // Add 10,000 unique items
                for (int i = 0; i < 10000; i++)
                {
                    hll.Update($"item_{i}"u8.ToArray());
                }

                double estimate = hll.Estimate();
                double error = Math.Abs(estimate - 10000.0) / 10000.0;
                return error;
            }
        }
    }

    /// <summary>
    /// Comparison benchmark - SketchOxide vs Probably (alternative library)
    ///
    /// To use this, add NuGet package: Probably
    /// Then uncomment the code below and implement the Probably-based benchmarks.
    ///
    /// // [MemoryDiagnoser]
    /// // public class HyperLogLogComparison
    /// // {
    /// //     private HyperLogLog _sketchOxideHll;
    /// //     // private ProbablyHll _probablyHll; // After adding Probably NuGet
    /// //
    /// //     [GlobalSetup]
    /// //     public void Setup()
    /// //     {
    /// //         _sketchOxideHll = new HyperLogLog(14);
    /// //         // _probablyHll = new ProbablyHll(14);
    /// //     }
    /// //
    /// //     [Benchmark]
    /// //     public double SketchOxideEstimate()
    /// //     {
    /// //         return _sketchOxideHll.Estimate();
    /// //     }
    /// //
    /// //     [Benchmark]
    /// //     public long ProbablyEstimate()
    /// //     {
    /// //         // return _probablyHll.Count();
    /// //         return 0;
    /// //     }
    /// // }
    /// </summary>

    // Program.cs integration
    internal class Program
    {
        private static void Main(string[] args)
        {
            var summary = BenchmarkRunner.Run<HyperLogLogBenchmarks>(args);
        }
    }
}
