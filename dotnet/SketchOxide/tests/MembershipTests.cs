using Xunit;
using SketchOxide.Membership;
using System;

namespace SketchOxide.Tests
{
    public class BloomFilterTests : IDisposable
    {
        private BloomFilter? _bf;

        public BloomFilterTests()
        {
            _bf = new BloomFilter(1000, 0.01);
        }

        public void Dispose()
        {
            _bf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_bf);
        }

        [Fact]
        public void Insert_WithBytes_Succeeds()
        {
            _bf!.Insert("test".GetBytes());
        }

        [Fact]
        public void Contains_AfterInsert_ReturnsTrue()
        {
            _bf!.Insert("test".GetBytes());
            Assert.True(_bf.Contains("test".GetBytes()));
        }

        [Fact]
        public void Contains_NotInserted_ReturnsFalse()
        {
            Assert.False(_bf!.Contains("notinserted".GetBytes()));
        }

        [Fact]
        public void MultipleInserts_AllContained()
        {
            for (int i = 0; i < 100; i++)
            {
                _bf!.Insert($"item-{i}".GetBytes());
            }

            for (int i = 0; i < 100; i++)
            {
                Assert.True(_bf.Contains($"item-{i}".GetBytes()));
            }
        }
    }

    public class BlockedBloomFilterTests : IDisposable
    {
        private BlockedBloomFilter? _bbf;

        public BlockedBloomFilterTests()
        {
            _bbf = new BlockedBloomFilter(1000, 0.01);
        }

        public void Dispose()
        {
            _bbf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_bbf);
        }

        [Fact]
        public void Insert_WithBytes_Succeeds()
        {
            _bbf!.Insert("test".GetBytes());
        }

        [Fact]
        public void Contains_AfterInsert_ReturnsTrue()
        {
            _bbf!.Insert("test".GetBytes());
            Assert.True(_bbf.Contains("test".GetBytes()));
        }

        [Fact]
        public void MultipleInserts_AllContained()
        {
            for (int i = 0; i < 100; i++)
            {
                _bbf!.Insert($"item-{i}".GetBytes());
            }

            for (int i = 0; i < 100; i++)
            {
                Assert.True(_bbf.Contains($"item-{i}".GetBytes()));
            }
        }
    }

    public class CountingBloomFilterTests : IDisposable
    {
        private CountingBloomFilter? _cbf;

        public CountingBloomFilterTests()
        {
            _cbf = new CountingBloomFilter(1000, 0.01);
        }

        public void Dispose()
        {
            _cbf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_cbf);
        }

        [Fact]
        public void Insert_WithBytes_Succeeds()
        {
            _cbf!.Insert("test".GetBytes());
        }

        [Fact]
        public void Contains_AfterInsert_ReturnsTrue()
        {
            _cbf!.Insert("test".GetBytes());
            Assert.True(_cbf.Contains("test".GetBytes()));
        }

        [Fact]
        public void Remove_AfterInsert_Works()
        {
            _cbf!.Insert("test".GetBytes());
            Assert.True(_cbf.Contains("test".GetBytes()));
            _cbf.Remove("test".GetBytes());
            Assert.False(_cbf.Contains("test".GetBytes()));
        }
    }

    public class CuckooFilterTests : IDisposable
    {
        private CuckooFilter? _cf;

        public CuckooFilterTests()
        {
            _cf = new CuckooFilter(1000);
        }

        public void Dispose()
        {
            _cf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_cf);
        }

        [Fact]
        public void Insert_WithBytes_Succeeds()
        {
            _cf!.Insert("test".GetBytes());
        }

        [Fact]
        public void Contains_AfterInsert_ReturnsTrue()
        {
            _cf!.Insert("test".GetBytes());
            Assert.True(_cf.Contains("test".GetBytes()));
        }

        [Fact]
        public void Remove_AfterInsert_Works()
        {
            _cf!.Insert("test".GetBytes());
            _cf.Remove("test".GetBytes());
            Assert.False(_cf.Contains("test".GetBytes()));
        }
    }

    public class RibbonFilterTests : IDisposable
    {
        private RibbonFilter? _rf;

        public RibbonFilterTests()
        {
            _rf = new RibbonFilter(1000, 0.01);
        }

        public void Dispose()
        {
            _rf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_rf);
        }

        [Fact]
        public void Insert_WithBytes_Succeeds()
        {
            _rf!.Insert("test".GetBytes());
        }

        [Fact]
        public void Finalize_Succeeds()
        {
            _rf!.Insert("test".GetBytes());
            _rf.Finalize();
        }

        [Fact]
        public void Contains_AfterFinalizeAndInsert_ReturnsTrue()
        {
            _rf!.Insert("test".GetBytes());
            _rf.Finalize();
            Assert.True(_rf.Contains("test".GetBytes()));
        }
    }

    public class StableBloomFilterTests : IDisposable
    {
        private StableBloomFilter? _sbf;

        public StableBloomFilterTests()
        {
            _sbf = new StableBloomFilter(1000, 0.01);
        }

        public void Dispose()
        {
            _sbf?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_sbf);
        }

        [Fact]
        public void Insert_WithBytes_Succeeds()
        {
            _sbf!.Insert("test".GetBytes());
        }

        [Fact]
        public void Contains_AfterInsert_ReturnsTrue()
        {
            _sbf!.Insert("test".GetBytes());
            Assert.True(_sbf.Contains("test".GetBytes()));
        }
    }

    public class BinaryFuseFilterTests : IDisposable
    {
        private BinaryFuseFilter? _bff;

        public BinaryFuseFilterTests()
        {
            var items = new ulong[] { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 };
            _bff = new BinaryFuseFilter(items, 8);
        }

        public void Dispose()
        {
            _bff?.Dispose();
        }

        [Fact]
        public void Constructor_CreatesValidFilter()
        {
            Assert.NotNull(_bff);
        }

        [Fact]
        public void Contains_WithInsertedItem_ReturnsTrue()
        {
            Assert.True(_bff!.Contains(1));
        }

        [Fact]
        public void Contains_WithNotInsertedItem_ReturnsFalse()
        {
            Assert.False(_bff!.Contains(999));
        }
    }
}
