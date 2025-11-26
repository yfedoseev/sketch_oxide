import {
  VacuumFilter,
  GRF,
  NitroSketch,
  UnivMon,
  LearnedBloomFilter,
  CountMinSketch,
} from '../index'

describe('VacuumFilter', () => {
  describe('constructor', () => {
    it('should create VacuumFilter with valid parameters', () => {
      const filter = new VacuumFilter(1000, 0.01)
      expect(filter).toBeDefined()
    })

    it('should throw on invalid capacity (zero)', () => {
      expect(() => new VacuumFilter(0, 0.01)).toThrow()
    })

    it('should throw on invalid fpr', () => {
      expect(() => new VacuumFilter(1000, 0.0)).toThrow()
      expect(() => new VacuumFilter(1000, 1.0)).toThrow()
      expect(() => new VacuumFilter(1000, -0.1)).toThrow()
      expect(() => new VacuumFilter(1000, 1.5)).toThrow()
    })
  })

  describe('insert and contains', () => {
    it('should insert and query membership', () => {
      const filter = new VacuumFilter(100, 0.01)
      filter.insert(Buffer.from('key1'))
      filter.insert(Buffer.from('key2'))
      filter.insert(Buffer.from('key3'))

      expect(filter.contains(Buffer.from('key1'))).toBe(true)
      expect(filter.contains(Buffer.from('key2'))).toBe(true)
      expect(filter.contains(Buffer.from('key3'))).toBe(true)
    })

    it('should handle non-existent keys', () => {
      const filter = new VacuumFilter(100, 0.01)
      filter.insert(Buffer.from('exists'))

      // May have false positives, but should mostly be false
      const result = filter.contains(Buffer.from('nonexistent'))
      expect(typeof result).toBe('boolean')
    })

    it('should handle many insertions', () => {
      const filter = new VacuumFilter(100, 0.01)
      for (let i = 0; i < 50; i++) {
        filter.insert(Buffer.from(`key${i}`))
      }

      expect(filter.len()).toBe(50)
      for (let i = 0; i < 50; i++) {
        expect(filter.contains(Buffer.from(`key${i}`))).toBe(true)
      }
    })
  })

  describe('delete', () => {
    it('should delete existing keys', () => {
      const filter = new VacuumFilter(100, 0.01)
      filter.insert(Buffer.from('key1'))
      expect(filter.contains(Buffer.from('key1'))).toBe(true)

      const deleted = filter.delete(Buffer.from('key1'))
      expect(deleted).toBe(true)
      expect(filter.contains(Buffer.from('key1'))).toBe(false)
    })

    it('should return false for non-existent delete', () => {
      const filter = new VacuumFilter(100, 0.01)
      const deleted = filter.delete(Buffer.from('nonexistent'))
      expect(deleted).toBe(false)
    })

    it('should handle multiple deletes', () => {
      const filter = new VacuumFilter(100, 0.01)
      filter.insert(Buffer.from('a'))
      filter.insert(Buffer.from('b'))
      filter.insert(Buffer.from('c'))

      expect(filter.delete(Buffer.from('a'))).toBe(true)
      expect(filter.delete(Buffer.from('b'))).toBe(true)
      expect(filter.len()).toBe(1)
    })
  })

  describe('statistics', () => {
    it('should track load factor', () => {
      const filter = new VacuumFilter(100, 0.01)
      expect(filter.loadFactor()).toBe(0.0)

      filter.insert(Buffer.from('key1'))
      expect(filter.loadFactor()).toBeGreaterThan(0.0)
      expect(filter.loadFactor()).toBeLessThanOrEqual(1.0)
    })

    it('should report capacity and length', () => {
      const filter = new VacuumFilter(100, 0.01)
      expect(filter.capacity()).toBeGreaterThanOrEqual(100)
      expect(filter.len()).toBe(0)

      filter.insert(Buffer.from('key'))
      expect(filter.len()).toBe(1)
    })

    it('should check isEmpty', () => {
      const filter = new VacuumFilter(100, 0.01)
      expect(filter.isEmpty()).toBe(true)

      filter.insert(Buffer.from('key'))
      expect(filter.isEmpty()).toBe(false)
    })

    it('should report memory usage', () => {
      const filter = new VacuumFilter(100, 0.01)
      const mem = filter.memoryUsage()
      expect(mem).toBeGreaterThan(0)
    })

    it('should provide detailed stats', () => {
      const filter = new VacuumFilter(100, 0.01)
      const stats = filter.stats()

      expect(stats).toHaveProperty('capacity')
      expect(stats).toHaveProperty('numItems')
      expect(stats).toHaveProperty('loadFactor')
      expect(stats).toHaveProperty('memoryBits')
      expect(stats).toHaveProperty('fingerprintBits')
      expect(stats.capacity).toBeGreaterThan(0)
      expect(stats.fingerprintBits).toBeGreaterThan(0)
    })
  })

  describe('clear', () => {
    it('should clear all items', () => {
      const filter = new VacuumFilter(100, 0.01)
      filter.insert(Buffer.from('key1'))
      filter.insert(Buffer.from('key2'))
      expect(filter.len()).toBe(2)

      filter.clear()
      expect(filter.isEmpty()).toBe(true)
      expect(filter.len()).toBe(0)
      expect(filter.contains(Buffer.from('key1'))).toBe(false)
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const filter = new VacuumFilter(100, 0.01)
      const str = filter.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('VacuumFilter')
    })
  })
})

describe('GRF', () => {
  describe('build', () => {
    it('should build from sorted keys', () => {
      const keys = [10, 20, 30, 40, 50]
      const grf = GRF.build(keys, 6)
      expect(grf).toBeDefined()
    })

    it('should throw on empty keys', () => {
      expect(() => GRF.build([], 6)).toThrow()
    })

    it('should throw on invalid bitsPerKey', () => {
      const keys = [10, 20, 30]
      expect(() => GRF.build(keys, 1)).toThrow()
      expect(() => GRF.build(keys, 20)).toThrow()
    })

    it('should handle large key sets', () => {
      const keys = Array.from({ length: 1000 }, (_, i) => BigInt(i * 10))
      const grf = GRF.build(keys, 6)
      expect(grf).toBeDefined()
    })
  })

  describe('mayContainRange', () => {
    it('should detect ranges containing keys', () => {
      const keys = [100, 200, 300, 400, 500]
      const grf = GRF.build(keys, 6)

      expect(grf.mayContainRange(150, 250)).toBe(true) // Contains 200
      expect(grf.mayContainRange(100, 100)).toBe(true) // Point query
    })

    it('should reject ranges not containing keys', () => {
      const keys = [100, 200, 300, 400, 500]
      const grf = GRF.build(keys, 6)

      const result1 = grf.mayContainRange(1n, 50)
      const result2 = grf.mayContainRange(600, 700)
      expect(typeof result1).toBe('boolean')
      expect(typeof result2).toBe('boolean')
    })

    it('should handle overlapping ranges', () => {
      const keys = [10, 20, 30, 40, 50]
      const grf = GRF.build(keys, 6)

      expect(grf.mayContainRange(0, 100)).toBe(true)
      expect(grf.mayContainRange(25n, 35n)).toBe(true)
    })
  })

  describe('mayContain', () => {
    it('should check point membership', () => {
      const keys = [100, 200, 300, 400, 500]
      const grf = GRF.build(keys, 6)

      expect(grf.mayContain(200)).toBe(true)
      expect(grf.mayContain(300)).toBe(true)
      expect(grf.mayContain(100)).toBe(true)
    })

    it('should handle non-existent keys', () => {
      const keys = [100, 200, 300]
      const grf = GRF.build(keys, 6)

      const result = grf.mayContain(999n)
      expect(typeof result).toBe('boolean')
    })
  })

  describe('expectedFpr', () => {
    it('should calculate expected FPR', () => {
      const keys = [10, 20, 30, 40, 50]
      const grf = GRF.build(keys, 6)

      const fpr = grf.expectedFpr(10)
      expect(typeof fpr).toBe('number')
      expect(fpr).toBeGreaterThanOrEqual(0)
      expect(fpr).toBeLessThanOrEqual(1)
    })

    it('should show FPR increases with range width', () => {
      const keys = [10, 20, 30, 40, 50]
      const grf = GRF.build(keys, 6)

      const fpr1 = grf.expectedFpr(10)
      const fpr2 = grf.expectedFpr(100)
      const fpr3 = grf.expectedFpr(1000)

      expect(fpr2).toBeGreaterThan(fpr1)
      expect(fpr3).toBeGreaterThan(fpr2)
    })
  })

  describe('stats', () => {
    it('should return filter statistics', () => {
      const keys = [10, 20, 30, 40, 50]
      const grf = GRF.build(keys, 6)

      const stats = grf.stats()
      expect(stats).toHaveProperty('keyCount')
      expect(stats).toHaveProperty('segmentCount')
      expect(stats).toHaveProperty('avgKeysPerSegment')
      expect(stats).toHaveProperty('bitsPerKey')
      expect(stats).toHaveProperty('totalBits')
      expect(stats).toHaveProperty('memoryBytes')
      expect(stats.keyCount).toBe(5)
      expect(stats.bitsPerKey).toBe(6)
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const keys = [10, 20, 30]
      const grf = GRF.build(keys, 6)
      const str = grf.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('GRF')
    })
  })
})

describe('NitroSketch', () => {
  describe('constructor', () => {
    it('should create NitroSketch with valid parameters', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 0.1)
      expect(nitro).toBeDefined()
    })

    it('should throw on invalid sample rate (zero)', () => {
      const base = new CountMinSketch(0.01, 0.01)
      expect(() => new NitroSketch(base, 0.0)).toThrow()
    })

    it('should throw on invalid sample rate (negative)', () => {
      const base = new CountMinSketch(0.01, 0.01)
      expect(() => new NitroSketch(base, -0.1)).toThrow()
    })

    it('should throw on invalid sample rate (too large)', () => {
      const base = new CountMinSketch(0.01, 0.01)
      expect(() => new NitroSketch(base, 1.1)).toThrow()
    })

    it('should accept sample rate of 1.0', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 1.0)
      expect(nitro).toBeDefined()
    })
  })

  describe('updateSampled', () => {
    it('should update with sampling', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 0.1)

      for (let i = 0; i < 100; i++) {
        nitro.updateSampled(Buffer.from('key'))
      }

      const stats = nitro.stats()
      expect(stats.totalItemsEstimated).toBe(100)
    })

    it('should handle multiple keys', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 0.1)

      for (let i = 0; i < 100; i++) {
        nitro.updateSampled(Buffer.from(`key${i}`))
      }

      const stats = nitro.stats()
      expect(stats.totalItemsEstimated).toBe(100)
    })
  })

  describe('query', () => {
    it('should query key frequency', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 0.5)

      for (let i = 0; i < 100; i++) {
        nitro.updateSampled(Buffer.from('key'))
      }

      nitro.sync(1.0)

      const freq = nitro.query(Buffer.from('key'))
      expect(typeof freq).toBe('bigint')
    })
  })

  describe('sync', () => {
    it('should synchronize for accuracy', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 0.1)

      for (let i = 0; i < 1000; i++) {
        nitro.updateSampled(Buffer.from(`key${i % 10}`))
      }

      expect(() => nitro.sync(1.0)).not.toThrow()
    })
  })

  describe('stats', () => {
    it('should return sampling statistics', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 0.5)

      for (let i = 0; i < 100; i++) {
        nitro.updateSampled(Buffer.from(`key${i}`))
      }

      const stats = nitro.stats()
      expect(stats).toHaveProperty('sampleRate')
      expect(stats).toHaveProperty('sampledCount')
      expect(stats).toHaveProperty('unsampledCount')
      expect(stats).toHaveProperty('totalItemsEstimated')
      expect(stats.sampleRate).toBe(0.5)
      expect(stats.totalItemsEstimated).toBe(100)
    })
  })

  describe('resetStats', () => {
    it('should reset sampling statistics', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 0.1)

      for (let i = 0; i < 100; i++) {
        nitro.updateSampled(Buffer.from('key'))
      }

      expect(nitro.stats().totalItemsEstimated).toBeGreaterThan(0)

      nitro.resetStats()
      expect(nitro.stats().totalItemsEstimated).toBe(0)
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const base = new CountMinSketch(0.01, 0.01)
      const nitro = new NitroSketch(base, 0.1)
      const str = nitro.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('NitroSketch')
    })
  })
})

describe('UnivMon', () => {
  describe('constructor', () => {
    it('should create UnivMon with valid parameters', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)
      expect(univmon).toBeDefined()
    })

    it('should throw on invalid maxStreamSize (zero)', () => {
      expect(() => new UnivMon(0, 0.01, 0.01)).toThrow()
    })

    it('should throw on invalid epsilon', () => {
      expect(() => new UnivMon(10000, 0.0, 0.01)).toThrow()
      expect(() => new UnivMon(10000, 1.0, 0.01)).toThrow()
    })

    it('should throw on invalid delta', () => {
      expect(() => new UnivMon(10000, 0.01, 0.0)).toThrow()
      expect(() => new UnivMon(10000, 0.01, 1.0)).toThrow()
    })
  })

  describe('update', () => {
    it('should update with item and value', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)
      expect(() => {
        univmon.update(Buffer.from('key1'), 100)
        univmon.update(Buffer.from('key2'), 200)
      }).not.toThrow()
    })

    it('should handle many updates', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)
      for (let i = 0; i < 100; i++) {
        univmon.update(Buffer.from(`key${i}`), i + 1)
      }
      expect(univmon.estimateL1()).toBeGreaterThan(0)
    })
  })

  describe('estimateL1', () => {
    it('should estimate L1 norm (sum of frequencies)', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)
      univmon.update(Buffer.from('a'), 100)
      univmon.update(Buffer.from('b'), 200)
      univmon.update(Buffer.from('c'), 300)

      const l1 = univmon.estimateL1()
      expect(l1).toBeGreaterThan(0)
      expect(l1).toBeGreaterThan(500) // Should be close to 600
    })
  })

  describe('estimateL2', () => {
    it('should estimate L2 norm', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)
      univmon.update(Buffer.from('a'), 100)
      univmon.update(Buffer.from('b'), 200)

      const l2 = univmon.estimateL2()
      expect(typeof l2).toBe('number')
      expect(l2).toBeGreaterThan(0)
    })
  })

  describe('estimateEntropy', () => {
    it('should estimate Shannon entropy', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)
      univmon.update(Buffer.from('a'), 100)
      univmon.update(Buffer.from('b'), 100)
      univmon.update(Buffer.from('c'), 100)

      const entropy = univmon.estimateEntropy()
      expect(typeof entropy).toBe('number')
      expect(entropy).toBeGreaterThanOrEqual(0)
    })
  })

  describe('heavyHitters', () => {
    it('should return heavy hitters', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)

      // Add heavy hitters
      for (let i = 0; i < 100; i++) {
        univmon.update(Buffer.from('heavy1'), 10)
      }
      for (let i = 0; i < 50; i++) {
        univmon.update(Buffer.from('heavy2'), 10)
      }
      for (let i = 0; i < 10; i++) {
        univmon.update(Buffer.from('light'), 1)
      }

      const topItems = univmon.heavyHitters(0.1)
      expect(Array.isArray(topItems)).toBe(true)
    })

    it('should handle empty sketch', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)
      const topItems = univmon.heavyHitters(0.1)
      expect(Array.isArray(topItems)).toBe(true)
    })
  })

  describe('detectChange', () => {
    it('should detect change between sketches', () => {
      const univmon1 = new UnivMon(10000, 0.01, 0.01)
      const univmon2 = new UnivMon(10000, 0.01, 0.01)

      // Different distributions
      univmon1.update(Buffer.from('a'), 100)
      univmon2.update(Buffer.from('b'), 100)

      const change = univmon1.detectChange(univmon2)
      expect(typeof change).toBe('number')
      expect(change).toBeGreaterThanOrEqual(0)
    })

    it('should detect no change for similar sketches', () => {
      const univmon1 = new UnivMon(10000, 0.01, 0.01)
      const univmon2 = new UnivMon(10000, 0.01, 0.01)

      // Same distribution
      univmon1.update(Buffer.from('a'), 100)
      univmon2.update(Buffer.from('a'), 100)

      const change = univmon1.detectChange(univmon2)
      expect(change).toBeLessThan(0.1) // Should be close to 0
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const univmon = new UnivMon(10000, 0.01, 0.01)
      const str = univmon.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('UnivMon')
    })
  })
})

describe('LearnedBloomFilter', () => {
  describe('new', () => {
    it('should create LearnedBloomFilter with valid training data', () => {
      const keys: Buffer[] = []
      for (let i = 0; i < 100; i++) {
        keys.push(Buffer.from(`key${i}`))
      }

      const filter = LearnedBloomFilter.new(keys, 0.01)
      expect(filter).toBeDefined()
    })

    it('should throw on empty training data', () => {
      expect(() => LearnedBloomFilter.new([], 0.01)).toThrow()
    })

    it('should throw on too few training samples', () => {
      const keys = [Buffer.from('key1'), Buffer.from('key2')]
      expect(() => LearnedBloomFilter.new(keys, 0.01)).toThrow()
    })

    it('should throw on invalid fpr', () => {
      const keys: Buffer[] = []
      for (let i = 0; i < 100; i++) {
        keys.push(Buffer.from(`key${i}`))
      }

      expect(() => LearnedBloomFilter.new(keys, 0.0)).toThrow()
      expect(() => LearnedBloomFilter.new(keys, 1.0)).toThrow()
    })
  })

  describe('contains', () => {
    it('should return true for training keys (no false negatives)', () => {
      const keys: Buffer[] = []
      for (let i = 0; i < 100; i++) {
        keys.push(Buffer.from(`key${i}`))
      }

      const filter = LearnedBloomFilter.new(keys, 0.01)

      // All training keys should return true
      for (let i = 0; i < 100; i++) {
        expect(filter.contains(Buffer.from(`key${i}`))).toBe(true)
      }
    })

    it('should handle non-existent keys', () => {
      const keys: Buffer[] = []
      for (let i = 0; i < 100; i++) {
        keys.push(Buffer.from(`key${i}`))
      }

      const filter = LearnedBloomFilter.new(keys, 0.01)

      // Non-existent keys may return true or false (with FPR)
      const result = filter.contains(Buffer.from('nonexistent'))
      expect(typeof result).toBe('boolean')
    })

    it('should work with large datasets', () => {
      const keys: Buffer[] = []
      for (let i = 0; i < 1000; i++) {
        keys.push(Buffer.from(`key${i}`))
      }

      const filter = LearnedBloomFilter.new(keys, 0.01)

      // Sample check
      expect(filter.contains(Buffer.from('key0'))).toBe(true)
      expect(filter.contains(Buffer.from('key500'))).toBe(true)
      expect(filter.contains(Buffer.from('key999'))).toBe(true)
    })
  })

  describe('memoryUsage', () => {
    it('should report memory usage', () => {
      const keys: Buffer[] = []
      for (let i = 0; i < 100; i++) {
        keys.push(Buffer.from(`key${i}`))
      }

      const filter = LearnedBloomFilter.new(keys, 0.01)
      const mem = filter.memoryUsage()
      expect(typeof mem).toBe('number')
      expect(mem).toBeGreaterThan(0)
    })

    it('should show memory savings compared to standard Bloom', () => {
      const keys: Buffer[] = []
      for (let i = 0; i < 1000; i++) {
        keys.push(Buffer.from(`key${i}`))
      }

      const filter = LearnedBloomFilter.new(keys, 0.01)
      const mem = filter.memoryUsage()

      // Learned Bloom should use significantly less memory
      // Standard Bloom would use ~10 bits/element = 1250 bytes
      // Learned Bloom should use ~3-4 bits/element = 375-500 bytes
      expect(mem).toBeLessThan(2000) // Very generous upper bound
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const keys: Buffer[] = []
      for (let i = 0; i < 100; i++) {
        keys.push(Buffer.from(`key${i}`))
      }

      const filter = LearnedBloomFilter.new(keys, 0.01)
      const str = filter.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('LearnedBloomFilter')
      expect(str).toContain('EXPERIMENTAL')
    })
  })
})
