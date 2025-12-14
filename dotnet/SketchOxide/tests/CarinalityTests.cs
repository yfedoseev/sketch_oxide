using Xunit;
using SketchOxide.Cardinality;
using System;

namespace SketchOxide.Tests
{
    public class HyperLogLogTests : IDisposable
    {
        private HyperLogLog? _hll;

        public HyperLogLogTests()
        {
            _hll = new HyperLogLog(14);
        }

        public void Dispose()
        {
            _hll?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_hll);
            Assert.Equal(14, _hll!.Precision);
        }

        [Fact]
        public void Update_WithBytes_Succeeds()
        {
            _hll!.Update("test".GetBytes());
            var estimate = _hll.Estimate();
            Assert.True(estimate >= 0);
        }

        [Fact]
        public void Estimate_OnEmptySketch_ReturnsZero()
        {
            var estimate = _hll!.Estimate();
            Assert.True(estimate == 0 || estimate < 1.0);
        }

        [Fact]
        public void Estimate_WithMultipleItems_ReturnsReasonableValue()
        {
            for (int i = 0; i < 1000; i++)
            {
                _hll!.Update($"item-{i}".GetBytes());
            }
            var estimate = _hll.Estimate();
            var error = Math.Abs(estimate - 1000) / 1000;
            Assert.True(error < 0.05); // Allow 5% error
        }

        [Fact]
        public void Duplicates_AreHandledCorrectly()
        {
            for (int i = 0; i < 100; i++)
            {
                _hll!.Update("same".GetBytes());
            }
            var estimate = _hll.Estimate();
            Assert.True(estimate < 5);
        }

        [Fact]
        public void Merge_CombinesSketches()
        {
            var hll2 = new HyperLogLog(14);
            try
            {
                for (int i = 0; i < 500; i++)
                {
                    _hll!.Update($"first-{i}".GetBytes());
                    hll2.Update($"second-{i}".GetBytes());
                }

                var estimate1 = _hll.Estimate();
                _hll.Merge(hll2);
                var estimateMerged = _hll.Estimate();

                Assert.True(estimateMerged > estimate1);
            }
            finally
            {
                hll2.Dispose();
            }
        }

        [Fact]
        public void Precision_ReturnsCorrectValue()
        {
            Assert.Equal(14, _hll!.Precision);
        }

        [Fact]
        public void TryWithResources_WorksCorrectly()
        {
            using (var hll = new HyperLogLog(12))
            {
                hll.Update("test".GetBytes());
                var estimate = hll.Estimate();
                Assert.True(estimate >= 0);
            }
        }
    }

    public class UltraLogLogTests : IDisposable
    {
        private UltraLogLog? _ull;

        public UltraLogLogTests()
        {
            _ull = new UltraLogLog(14);
        }

        public void Dispose()
        {
            _ull?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_ull);
        }

        [Fact]
        public void Update_WithBytes_Succeeds()
        {
            _ull!.Update("test".GetBytes());
            var estimate = _ull.Estimate();
            Assert.True(estimate >= 0);
        }

        [Fact]
        public void Estimate_WithMultipleItems_IsAccurate()
        {
            for (int i = 0; i < 1000; i++)
            {
                _ull!.Update($"item-{i}".GetBytes());
            }
            var estimate = _ull.Estimate();
            Assert.True(estimate > 900 && estimate < 1100);
        }

        [Fact]
        public void Merge_CombinesSketches()
        {
            var ull2 = new UltraLogLog(14);
            try
            {
                for (int i = 0; i < 500; i++)
                {
                    _ull!.Update($"first-{i}".GetBytes());
                    ull2.Update($"second-{i}".GetBytes());
                }
                _ull.Merge(ull2);
                var estimateMerged = _ull.Estimate();
                Assert.True(estimateMerged > 500);
            }
            finally
            {
                ull2.Dispose();
            }
        }
    }

    public class CpcSketchTests : IDisposable
    {
        private CpcSketch? _cpc;

        public CpcSketchTests()
        {
            _cpc = new CpcSketch(12);
        }

        public void Dispose()
        {
            _cpc?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_cpc);
        }

        [Fact]
        public void Update_WithBytes_Succeeds()
        {
            _cpc!.Update("test".GetBytes());
            var estimate = _cpc.Estimate();
            Assert.True(estimate >= 0);
        }

        [Fact]
        public void Estimate_WithMultipleItems_IsAccurate()
        {
            for (int i = 0; i < 1000; i++)
            {
                _cpc!.Update($"item-{i}".GetBytes());
            }
            var estimate = _cpc.Estimate();
            Assert.True(estimate > 900 && estimate < 1100);
        }
    }

    public class QSketchTests : IDisposable
    {
        private QSketch? _qs;

        public QSketchTests()
        {
            _qs = new QSketch(64);
        }

        public void Dispose()
        {
            _qs?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_qs);
        }

        [Fact]
        public void Update_WithBytesAndWeight_Succeeds()
        {
            _qs!.Update("test".GetBytes(), 1.0);
            var estimate = _qs.Estimate();
            Assert.True(estimate >= 0);
        }

        [Fact]
        public void Estimate_WithMultipleItems_IsAccurate()
        {
            for (int i = 0; i < 1000; i++)
            {
                _qs!.Update($"item-{i}".GetBytes(), 1.0);
            }
            var estimate = _qs.Estimate();
            Assert.True(estimate > 900 && estimate < 1100);
        }
    }

    public class ThetaSketchTests : IDisposable
    {
        private ThetaSketch? _theta;

        public ThetaSketchTests()
        {
            _theta = new ThetaSketch(12);
        }

        public void Dispose()
        {
            _theta?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_theta);
        }

        [Fact]
        public void Update_WithBytes_Succeeds()
        {
            _theta!.Update("test".GetBytes());
            var estimate = _theta.Estimate();
            Assert.True(estimate >= 0);
        }

        [Fact]
        public void Estimate_WithMultipleItems_IsAccurate()
        {
            for (int i = 0; i < 1000; i++)
            {
                _theta!.Update($"item-{i}".GetBytes());
            }
            var estimate = _theta.Estimate();
            Assert.True(estimate > 900 && estimate < 1100);
        }
    }
}
