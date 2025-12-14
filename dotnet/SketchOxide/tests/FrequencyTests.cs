using Xunit;
using SketchOxide.Frequency;
using System;

namespace SketchOxide.Tests
{
    public class CountMinSketchTests : IDisposable
    {
        private CountMinSketch? _cms;

        public CountMinSketchTests()
        {
            _cms = new CountMinSketch(0.01, 0.01);
        }

        public void Dispose()
        {
            _cms?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_cms);
        }

        [Fact]
        public void Update_WithBytes_Succeeds()
        {
            _cms!.Update("test".GetBytes());
        }

        [Fact]
        public void Estimate_SingleItem_ReturnsOne()
        {
            _cms!.Update("item".GetBytes());
            var estimate = _cms.Estimate("item".GetBytes());
            Assert.True(estimate >= 1);
        }

        [Fact]
        public void Estimate_WithMultipleUpdates_IsAccurate()
        {
            for (int i = 0; i < 100; i++)
            {
                _cms!.Update("item".GetBytes());
            }
            var estimate = _cms.Estimate("item".GetBytes());
            Assert.True(estimate >= 100);
            Assert.True(estimate <= 110);
        }

        [Fact]
        public void Merge_CombinesSketches()
        {
            var cms2 = new CountMinSketch(0.01, 0.01);
            try
            {
                for (int i = 0; i < 50; i++)
                {
                    _cms!.Update($"first".GetBytes());
                    cms2.Update($"second".GetBytes());
                }

                var estimate1Before = _cms.Estimate("first".GetBytes());
                _cms.Merge(cms2);
                var estimate1After = _cms.Estimate("first".GetBytes());

                Assert.True(estimate1After >= estimate1Before);
            }
            finally
            {
                cms2.Dispose();
            }
        }
    }

    public class CountSketchTests : IDisposable
    {
        private CountSketch? _cs;

        public CountSketchTests()
        {
            _cs = new CountSketch(0.01, 0.01);
        }

        public void Dispose()
        {
            _cs?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_cs);
        }

        [Fact]
        public void Update_WithBytesAndWeight_Succeeds()
        {
            _cs!.Update("test".GetBytes(), 1);
        }

        [Fact]
        public void Estimate_WithUpdates_ReturnsValue()
        {
            _cs!.Update("item".GetBytes(), 1);
            var estimate = _cs.Estimate("item".GetBytes());
            Assert.True(estimate >= 0);
        }
    }

    public class ConservativeCountMinTests : IDisposable
    {
        private ConservativeCountMin? _ccm;

        public ConservativeCountMinTests()
        {
            _ccm = new ConservativeCountMin(0.01, 0.01);
        }

        public void Dispose()
        {
            _ccm?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_ccm);
        }

        [Fact]
        public void Update_WithBytes_Succeeds()
        {
            _ccm!.Update("test".GetBytes());
        }

        [Fact]
        public void Estimate_WithUpdates_ReturnsValue()
        {
            _ccm!.Update("item".GetBytes());
            var estimate = _ccm.Estimate("item".GetBytes());
            Assert.True(estimate >= 1);
        }
    }

    public class SpaceSavingTests : IDisposable
    {
        private SpaceSaving? _ss;

        public SpaceSavingTests()
        {
            _ss = new SpaceSaving(0.01);
        }

        public void Dispose()
        {
            _ss?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_ss);
        }

        [Fact]
        public void Update_WithItem_Succeeds()
        {
            _ss!.Update(1000);
        }

        [Fact]
        public void Update_MultipleItems_Succeeds()
        {
            for (int i = 0; i < 100; i++)
            {
                _ss!.Update((ulong)i);
            }
        }
    }

    public class FrequentItemsTests : IDisposable
    {
        private FrequentItems? _fi;

        public FrequentItemsTests()
        {
            _fi = new FrequentItems(100);
        }

        public void Dispose()
        {
            _fi?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_fi);
        }

        [Fact]
        public void Update_WithItem_Succeeds()
        {
            _fi!.Update(1000);
        }

        [Fact]
        public void Update_MultipleItems_Succeeds()
        {
            for (int i = 0; i < 100; i++)
            {
                _fi!.Update((ulong)i);
            }
        }
    }

    public class ElasticSketchTests : IDisposable
    {
        private ElasticSketch? _es;

        public ElasticSketchTests()
        {
            _es = new ElasticSketch(1024, 3);
        }

        public void Dispose()
        {
            _es?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_es);
        }

        [Fact]
        public void Update_WithBytesAndCount_Succeeds()
        {
            _es!.Update("item".GetBytes(), 1);
        }

        [Fact]
        public void Estimate_WithUpdates_ReturnsValue()
        {
            _es!.Update("item".GetBytes(), 1);
            var estimate = _es.Estimate("item".GetBytes());
            Assert.True(estimate >= 1);
        }
    }

    public class SALSATests : IDisposable
    {
        private SALSA? _salsa;

        public SALSATests()
        {
            _salsa = new SALSA(0.01, 0.01);
        }

        public void Dispose()
        {
            _salsa?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_salsa);
        }

        [Fact]
        public void Update_WithBytesAndWeight_Succeeds()
        {
            _salsa!.Update("test".GetBytes(), 1);
        }

        [Fact]
        public void Estimate_WithUpdates_ReturnsValue()
        {
            _salsa!.Update("item".GetBytes(), 1);
            var estimate = _salsa.Estimate("item".GetBytes());
            Assert.True(estimate >= 1);
        }
    }

    public class RemovableUniversalSketchTests : IDisposable
    {
        private RemovableUniversalSketch? _rus;

        public RemovableUniversalSketchTests()
        {
            _rus = new RemovableUniversalSketch(0.01, 0.01);
        }

        public void Dispose()
        {
            _rus?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidSketch()
        {
            Assert.NotNull(_rus);
        }

        [Fact]
        public void Update_WithBytesAndDelta_Succeeds()
        {
            _rus!.Update("test".GetBytes(), 1);
        }

        [Fact]
        public void Estimate_WithUpdates_ReturnsValue()
        {
            _rus!.Update("item".GetBytes(), 1);
            var estimate = _rus.Estimate("item".GetBytes());
            Assert.True(estimate >= 1);
        }
    }
}
