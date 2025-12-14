using Xunit;
using SketchOxide.RangeFilters;
using SketchOxide.Reconciliation;
using SketchOxide.Universal;
using System;

namespace SketchOxide.Tests
{
    // ============================================================================
    // Range Filters Tests
    // ============================================================================

    public class GrafiteTests
    {
        [Fact]
        public void Build_WithSortedKeys_Succeeds()
        {
            var keys = new ulong[] { 1, 2, 3, 4, 5 };
            using (var grf = Grafite.Build(keys, 8))
            {
                Assert.NotNull(grf);
            }
        }

        [Fact]
        public void MayContain_WithInsertedKey_ReturnsTrue()
        {
            var keys = new ulong[] { 10, 20, 30, 40, 50 };
            using (var grf = Grafite.Build(keys, 8))
            {
                Assert.True(grf!.MayContain(10));
                Assert.True(grf.MayContain(30));
            }
        }

        [Fact]
        public void MayContainRange_WithRange_Returns()
        {
            var keys = new ulong[] { 10, 20, 30, 40, 50 };
            using (var grf = Grafite.Build(keys, 8))
            {
                Assert.True(grf!.MayContainRange(10, 50));
            }
        }

        [Fact]
        public void KeyCount_ReturnsCorrectValue()
        {
            var keys = new ulong[] { 1, 2, 3, 4, 5 };
            using (var grf = Grafite.Build(keys, 8))
            {
                Assert.True(grf!.KeyCount() > 0);
            }
        }
    }

    public class GRFTests
    {
        [Fact]
        public void Build_WithSortedKeys_Succeeds()
        {
            var keys = new ulong[] { 1, 2, 3, 4, 5 };
            using (var grf = GRF.Build(keys, 4))
            {
                Assert.NotNull(grf);
            }
        }

        [Fact]
        public void MayContainRange_WithRange_Returns()
        {
            var keys = new ulong[] { 1, 2, 3, 4, 5 };
            using (var grf = GRF.Build(keys, 4))
            {
                Assert.True(grf!.MayContainRange(1, 5));
            }
        }

        [Fact]
        public void RangeCount_ReturnsNonZero()
        {
            var keys = new ulong[] { 1, 2, 3, 4, 5 };
            using (var grf = GRF.Build(keys, 4))
            {
                var count = grf!.RangeCount(1, 5);
                Assert.True(count > 0);
            }
        }
    }

    public class MementoFilterTests : IDisposable
    {
        private MementoFilter? _mf;

        public MementoFilterTests()
        {
            _mf = new MementoFilter(1000, 0.01, 0);
        }

        public void Dispose()
        {
            _mf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_mf);
        }

        [Fact]
        public void Insert_WithBytesAndTimestamp_Succeeds()
        {
            _mf!.Insert("test".GetBytes(), 0);
        }

        [Fact]
        public void Contains_AfterInsert_ReturnsTrue()
        {
            _mf!.Insert("test".GetBytes(), 0);
            Assert.True(_mf.Contains("test".GetBytes(), 0));
        }

        [Fact]
        public void Merge_CombinesFilters()
        {
            var mf2 = new MementoFilter(1000, 0.01, 0);
            try
            {
                _mf!.Insert("first".GetBytes(), 0);
                mf2.Insert("second".GetBytes(), 0);
                _mf.Merge(mf2);
                Assert.NotNull(_mf);
            }
            finally
            {
                mf2.Dispose();
            }
        }
    }

    // ============================================================================
    // Reconciliation Tests
    // ============================================================================

    public class RatelessIBLTTests : IDisposable
    {
        private RatelessIBLT? _iblt;

        public RatelessIBLTTests()
        {
            _iblt = new RatelessIBLT(100, 3);
        }

        public void Dispose()
        {
            _iblt?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_iblt);
        }

        [Fact]
        public void Insert_WithBytes_Succeeds()
        {
            _iblt!.Insert("item".GetBytes());
        }

        [Fact]
        public void MultipleInserts_Succeed()
        {
            for (int i = 0; i < 10; i++)
            {
                _iblt!.Insert($"item-{i}".GetBytes());
            }
        }

        [Fact]
        public void TryWithResources_Works()
        {
            using (var iblt = new RatelessIBLT(100, 3))
            {
                iblt.Insert("data".GetBytes());
                Assert.NotNull(iblt);
            }
        }
    }

    // ============================================================================
    // Streaming Tests (Alternative)
    // ============================================================================

    public class SlidingHyperLogLogAltTests : IDisposable
    {
        private SlidingHyperLogLog? _shll;

        public SlidingHyperLogLogAltTests()
        {
            _shll = new SlidingHyperLogLog(14, 3600);
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
        public void Update_WithData_Succeeds()
        {
            _shll!.Update("test".GetBytes(), 100);
        }

        [Fact]
        public void EstimateWindow_WithWindow_ReturnsValue()
        {
            _shll!.Update("item".GetBytes(), 100);
            var estimate = _shll.EstimateWindow(200, 3600);
            Assert.True(estimate >= 0);
        }
    }

    // ============================================================================
    // Universal Streaming Tests
    // ============================================================================

    public class UnivMonTests : IDisposable
    {
        private UnivMon? _um;

        public UnivMonTests()
        {
            _um = new UnivMon(0.01, 0.001);
        }

        public void Dispose()
        {
            _um?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_um);
        }

        [Fact]
        public void Update_WithBytesAndWeight_Succeeds()
        {
            _um!.Update("item".GetBytes(), 1);
        }

        [Fact]
        public void WeightedUpdates_Succeed()
        {
            _um!.Update("heavy".GetBytes(), 100);
            _um.Update("light".GetBytes(), 1);
            Assert.NotNull(_um);
        }

        [Fact]
        public void Merge_CombinesSketches()
        {
            var um2 = new UnivMon(0.01, 0.001);
            try
            {
                _um!.Update("item".GetBytes(), 1);
                um2.Update("item".GetBytes(), 1);
                _um.Merge(um2);
            }
            finally
            {
                um2.Dispose();
            }
        }

        [Fact]
        public void TryWithResources_Works()
        {
            using (var test = new UnivMon(0.01, 0.001))
            {
                test.Update("data".GetBytes(), 1);
                Assert.NotNull(test);
            }
        }
    }

    // ============================================================================
    // NitroSketch Tests
    // ============================================================================

    public class NitroSketchTests : IDisposable
    {
        private NitroSketch? _ns;

        public NitroSketchTests()
        {
            _ns = new NitroSketch(0.01, 0.01);
        }

        public void Dispose()
        {
            _ns?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_ns);
        }

        [Fact]
        public void Update_WithBytesAndCount_Succeeds()
        {
            _ns!.Update("item".GetBytes(), 1);
        }

        [Fact]
        public void Estimate_WithUpdates_ReturnsValue()
        {
            _ns!.Update("item".GetBytes(), 1);
            var estimate = _ns.Estimate("item".GetBytes());
            Assert.True(estimate >= 1);
        }
    }

    // ============================================================================
    // HeavyKeeper Tests
    // ============================================================================

    public class HeavyKeeperTests : IDisposable
    {
        private HeavyKeeper? _hk;

        public HeavyKeeperTests()
        {
            _hk = new HeavyKeeper(0.01, 0.01);
        }

        public void Dispose()
        {
            _hk?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_hk);
        }

        [Fact]
        public void Insert_WithBytesAndCount_Succeeds()
        {
            _hk!.Insert("item".GetBytes(), 1);
        }

        [Fact]
        public void Query_WithBytes_ReturnsValue()
        {
            _hk!.Insert("item".GetBytes(), 1);
            var count = _hk.Query("item".GetBytes());
            Assert.True(count >= 1);
        }
    }

    // ============================================================================
    // LearnedBloomFilter Tests
    // ============================================================================

    public class LearnedBloomFilterTests : IDisposable
    {
        private LearnedBloomFilter? _lbf;

        public LearnedBloomFilterTests()
        {
            _lbf = new LearnedBloomFilter(1000, 0.01);
        }

        public void Dispose()
        {
            _lbf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_lbf);
        }

        [Fact]
        public void Contains_WithItem_Returns()
        {
            var result = _lbf!.Contains(42);
            Assert.False(result); // Not inserted, should be false
        }

        [Fact]
        public void MemoryUsage_ReturnsValue()
        {
            var memory = _lbf!.MemoryUsage();
            Assert.True(memory > 0);
        }
    }

    // ============================================================================
    // VacuumFilter Tests
    // ============================================================================

    public class VacuumFilterTests : IDisposable
    {
        private VacuumFilter? _vf;

        public VacuumFilterTests()
        {
            _vf = new VacuumFilter(1000, 0.01);
        }

        public void Dispose()
        {
            _vf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_vf);
        }

        [Fact]
        public void Insert_WithBytes_Succeeds()
        {
            _vf!.Insert("test".GetBytes());
        }

        [Fact]
        public void Contains_AfterInsert_ReturnsTrue()
        {
            _vf!.Insert("test".GetBytes());
            Assert.True(_vf.Contains("test".GetBytes()));
        }

        [Fact]
        public void Delete_AfterInsert_Works()
        {
            _vf!.Insert("test".GetBytes());
            _vf.Delete("test".GetBytes());
            Assert.False(_vf.Contains("test".GetBytes()));
        }

        [Fact]
        public void Clear_RemovesAllItems()
        {
            _vf!.Insert("test".GetBytes());
            _vf.Clear();
            Assert.False(_vf.Contains("test".GetBytes()));
        }
    }
}
