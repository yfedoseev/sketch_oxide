using Xunit;
using SketchOxide.Quantiles;
using SketchOxide.Streaming;
using SketchOxide.Similarity;
using SketchOxide.Sampling;
using System;

namespace SketchOxide.Tests
{
    // ============================================================================
    // Quantiles Tests
    // ============================================================================

    public class DDSketchTests : IDisposable
    {
        private DDSketch? _dd;

        public DDSketchTests()
        {
            _dd = new DDSketch(0.01);
        }

        public void Dispose()
        {
            _dd?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_dd);
        }

        [Fact]
        public void Add_WithValue_Succeeds()
        {
            _dd!.Add(42.5);
        }

        [Fact]
        public void Quantile_AfterAdds_ReturnsValue()
        {
            for (int i = 1; i <= 100; i++)
            {
                _dd!.Add(i);
            }
            var median = _dd.Quantile(0.5);
            Assert.True(median > 40 && median < 60);
        }

        [Fact]
        public void Count_ReturnsCorrectValue()
        {
            for (int i = 0; i < 100; i++)
            {
                _dd!.Add(i);
            }
            Assert.Equal(100uL, _dd.Count());
        }
    }

    public class KllSketchTests : IDisposable
    {
        private KllSketch? _kll;

        public KllSketchTests()
        {
            _kll = new KllSketch(256);
        }

        public void Dispose()
        {
            _kll?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_kll);
        }

        [Fact]
        public void Update_WithValue_Succeeds()
        {
            _kll!.Update(42.5);
        }

        [Fact]
        public void Quantile_AfterUpdates_ReturnsValue()
        {
            for (int i = 1; i <= 100; i++)
            {
                _kll!.Update(i);
            }
            var median = _kll.Quantile(0.5);
            Assert.True(median > 40 && median < 60);
        }

        [Fact]
        public void Count_ReturnsCorrectValue()
        {
            for (int i = 0; i < 100; i++)
            {
                _kll!.Update(i);
            }
            Assert.Equal(100uL, _kll.Count());
        }
    }

    public class ReqSketchTests : IDisposable
    {
        private ReqSketch? _req;

        public ReqSketchTests()
        {
            _req = new ReqSketch(256, false); // false = LowRankAccuracy
        }

        public void Dispose()
        {
            _req?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_req);
        }

        [Fact]
        public void Update_WithValue_Succeeds()
        {
            _req!.Update(42.5);
        }

        [Fact]
        public void Quantile_AfterUpdates_ReturnsValue()
        {
            for (int i = 1; i <= 100; i++)
            {
                _req!.Update(i);
            }
            var median = _req.Quantile(0.5);
            Assert.True(median > 40 && median < 60);
        }
    }

    public class SplineSketchTests : IDisposable
    {
        private SplineSketch? _spline;

        public SplineSketchTests()
        {
            _spline = new SplineSketch(1000);
        }

        public void Dispose()
        {
            _spline?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_spline);
        }

        [Fact]
        public void Update_WithValueAndWeight_Succeeds()
        {
            _spline!.Update(42, 1.0);
        }

        [Fact]
        public void Query_AfterUpdates_ReturnsValue()
        {
            for (ulong i = 1; i <= 100; i++)
            {
                _spline!.Update(i, 1.0);
            }
            var median = _spline.Query(0.5);
            Assert.True(median > 40 && median < 60);
        }
    }

    public class TDigestTests : IDisposable
    {
        private TDigest? _td;

        public TDigestTests()
        {
            _td = new TDigest(100);
        }

        public void Dispose()
        {
            _td?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_td);
        }

        [Fact]
        public void Update_WithValue_Succeeds()
        {
            _td!.Update(42.5);
        }

        [Fact]
        public void Quantile_AfterUpdates_ReturnsValue()
        {
            for (int i = 1; i <= 100; i++)
            {
                _td!.Update(i);
            }
            var median = _td.Quantile(0.5);
            Assert.True(median > 40 && median < 60);
        }

        [Fact]
        public void Count_ReturnsCorrectValue()
        {
            for (int i = 0; i < 100; i++)
            {
                _td!.Update(i);
            }
            Assert.Equal(100, _td.Count());
        }
    }

    // ============================================================================
    // Streaming Tests
    // ============================================================================

    public class SlidingWindowCounterTests : IDisposable
    {
        private SlidingWindowCounter? _swc;

        public SlidingWindowCounterTests()
        {
            _swc = new SlidingWindowCounter(3600, 0.01);
        }

        public void Dispose()
        {
            _swc?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_swc);
        }

        [Fact]
        public void Increment_AtTimestamp_Succeeds()
        {
            _swc!.Increment(100);
        }

        [Fact]
        public void Count_WithinWindow_ReturnsValue()
        {
            _swc!.Increment(100);
            _swc.Increment(200);
            var count = _swc.Count(300);
            Assert.Equal(2uL, count);
        }
    }

    public class ExponentialHistogramTests : IDisposable
    {
        private ExponentialHistogram? _eh;

        public ExponentialHistogramTests()
        {
            _eh = new ExponentialHistogram(3600, 0.01);
        }

        public void Dispose()
        {
            _eh?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_eh);
        }

        [Fact]
        public void Insert_AtTimestamp_Succeeds()
        {
            _eh!.Insert(100, 1);
        }

        [Fact]
        public void Count_WithinWindow_ReturnsValue()
        {
            _eh!.Insert(100, 1);
            _eh.Insert(200, 2);
            var count = _eh.Count(300);
            Assert.Equal(3uL, count);
        }
    }

    public class SlidingHyperLogLogTests : IDisposable
    {
        private SlidingHyperLogLog? _shll;

        public SlidingHyperLogLogTests()
        {
            _shll = new SlidingHyperLogLog(12, 3600);
        }

        public void Dispose()
        {
            _shll?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_shll);
        }

        [Fact]
        public void Update_WithBytesAndTimestamp_Succeeds()
        {
            _shll!.Update("test".GetBytes(), 100);
        }

        [Fact]
        public void EstimateWindow_WithinWindow_ReturnsValue()
        {
            _shll!.Update("item1".GetBytes(), 100);
            _shll.Update("item2".GetBytes(), 200);
            var estimate = _shll.EstimateWindow(300, 3600);
            Assert.True(estimate >= 1);
        }
    }

    // ============================================================================
    // Similarity Tests
    // ============================================================================

    public class MinHashTests : IDisposable
    {
        private MinHash? _mh1;
        private MinHash? _mh2;

        public MinHashTests()
        {
            _mh1 = new MinHash(128);
            _mh2 = new MinHash(128);
        }

        public void Dispose()
        {
            _mh1?.Dispose();
            _mh2?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_mh1);
        }

        [Fact]
        public void Update_WithBytes_Succeeds()
        {
            _mh1!.Update("test".GetBytes());
        }

        [Fact]
        public void JaccardSimilarity_WithIdenticalSets_ReturnsOne()
        {
            for (int i = 0; i < 100; i++)
            {
                _mh1!.Update($"item-{i}".GetBytes());
                _mh2!.Update($"item-{i}".GetBytes());
            }

            var similarity = _mh1.JaccardSimilarity(_mh2);
            Assert.True(similarity > 0.95);
        }
    }

    public class SimHashTests : IDisposable
    {
        private SimHash? _sh1;
        private SimHash? _sh2;

        public SimHashTests()
        {
            _sh1 = new SimHash();
            _sh2 = new SimHash();
        }

        public void Dispose()
        {
            _sh1?.Dispose();
            _sh2?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_sh1);
        }

        [Fact]
        public void Update_WithBytes_Succeeds()
        {
            _sh1!.Update("test".GetBytes());
        }

        [Fact]
        public void Fingerprint_AfterUpdates_ReturnsValue()
        {
            _sh1!.Update("test".GetBytes());
            var fp = _sh1.Fingerprint();
            Assert.True(fp > 0);
        }

        [Fact]
        public void Similarity_WithIdenticalFeatures_ReturnsOne()
        {
            for (int i = 0; i < 10; i++)
            {
                _sh1!.Update($"feature-{i}".GetBytes());
                _sh2!.Update($"feature-{i}".GetBytes());
            }

            var similarity = _sh1.Similarity(_sh2);
            Assert.True(similarity > 0.9);
        }
    }

    // ============================================================================
    // Sampling Tests
    // ============================================================================

    public class ReservoirSamplingTests : IDisposable
    {
        private ReservoirSampling? _rs;

        public ReservoirSamplingTests()
        {
            _rs = new ReservoirSampling(100);
        }

        public void Dispose()
        {
            _rs?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_rs);
        }

        [Fact]
        public void Update_WithItem_Succeeds()
        {
            _rs!.Update(42);
        }

        [Fact]
        public void Count_AfterUpdates_ReturnsCorrectValue()
        {
            for (ulong i = 0; i < 100; i++)
            {
                _rs!.Update(i);
            }
            Assert.Equal(100uL, _rs.Count());
        }

        [Fact]
        public void Len_ReturnsSampleSize()
        {
            for (ulong i = 0; i < 50; i++)
            {
                _rs!.Update(i);
            }
            Assert.True(_rs.Len() <= 100);
        }
    }

    public class VarOptSamplingTests : IDisposable
    {
        private VarOptSampling? _vo;

        public VarOptSamplingTests()
        {
            _vo = new VarOptSampling(100);
        }

        public void Dispose()
        {
            _vo?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_vo);
        }

        [Fact]
        public void Update_WithItemAndWeight_Succeeds()
        {
            _vo!.Update(42, 1.5);
        }

        [Fact]
        public void Count_AfterUpdates_ReturnsCorrectValue()
        {
            for (ulong i = 0; i < 100; i++)
            {
                _vo!.Update(i, 1.0);
            }
            Assert.Equal(100uL, _vo.Count());
        }

        [Fact]
        public void TotalWeight_ReturnsCorrectValue()
        {
            for (ulong i = 0; i < 10; i++)
            {
                _vo!.Update(i, 2.0);
            }
            Assert.True(_vo.TotalWeight() >= 20.0);
        }
    }
}
