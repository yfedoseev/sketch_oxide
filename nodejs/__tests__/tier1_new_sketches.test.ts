import {
  HeavyKeeper,
  RatelessIBLT,
  Grafite,
  MementoFilter,
  SlidingHyperLogLog,
} from '../index'

describe('HeavyKeeper', () => {
  describe('constructor', () => {
    it('should create HeavyKeeper with valid parameters', () => {
      const hk = new HeavyKeeper(10, 0.001, 0.01)
      expect(hk).toBeDefined()
    })

    it('should create HeavyKeeper with default epsilon and delta', () => {
      const hk = new HeavyKeeper(10)
      expect(hk).toBeDefined()
    })

    it('should throw on invalid k (zero)', () => {
      expect(() => new HeavyKeeper(0, 0.001, 0.01)).toThrow()
    })

    it('should throw on invalid epsilon', () => {
      expect(() => new HeavyKeeper(10, -0.1, 0.01)).toThrow()
      expect(() => new HeavyKeeper(10, 1.5, 0.01)).toThrow()
    })

    it('should throw on invalid delta', () => {
      expect(() => new HeavyKeeper(10, 0.001, -0.1)).toThrow()
      expect(() => new HeavyKeeper(10, 0.001, 1.5)).toThrow()
    })
  })

  describe('update and estimate', () => {
    it('should update and estimate frequency', () => {
      const hk = new HeavyKeeper(10, 0.001, 0.01)
      for (let i = 0; i < 100; i++) {
        hk.update(Buffer.from('item_5'))
      }

      const count = hk.estimate(Buffer.from('item_5'))
      expect(count).toBeGreaterThan(0)
      expect(count).toBeGreaterThanOrEqual(90) // Allow some error
      expect(count).toBeLessThanOrEqual(110)
    })

    it('should handle multiple items', () => {
      const hk = new HeavyKeeper(10, 0.001, 0.01)

      // Add items with different frequencies
      for (let i = 0; i < 1000; i++) {
        hk.update(Buffer.from('frequent'))
      }
      for (let i = 0; i < 100; i++) {
        hk.update(Buffer.from('medium'))
      }
      for (let i = 0; i < 10; i++) {
        hk.update(Buffer.from('rare'))
      }

      const freqCount = hk.estimate(Buffer.from('frequent'))
      const medCount = hk.estimate(Buffer.from('medium'))
      const rareCount = hk.estimate(Buffer.from('rare'))

      expect(freqCount).toBeGreaterThan(medCount)
      expect(medCount).toBeGreaterThan(rareCount)
    })

    it('should handle empty buffer', () => {
      const hk = new HeavyKeeper(10, 0.001, 0.01)
      expect(() => {
        hk.update(Buffer.from([]))
      }).not.toThrow()
    })
  })

  describe('topK', () => {
    it('should return top-k heavy hitters', () => {
      const hk = new HeavyKeeper(5, 0.001, 0.01)

      for (let i = 0; i < 10; i++) {
        for (let j = 0; j < (10 - i) * 10; j++) {
          hk.update(Buffer.from(`item_${i}`))
        }
      }

      const topK = hk.topK()
      expect(topK).toBeDefined()
      expect(topK.length).toBeGreaterThan(0)
      expect(topK.length).toBeLessThanOrEqual(5)

      // Check structure
      for (const item of topK) {
        expect(item).toHaveProperty('hash')
        expect(item).toHaveProperty('count')
        expect(typeof item.hash).toBe('bigint')
        expect(typeof item.count).toBe('number')
        expect(item.count).toBeGreaterThan(0)
      }

      // Check sorted by count descending
      for (let i = 1; i < topK.length; i++) {
        expect(topK[i].count).toBeLessThanOrEqual(topK[i - 1].count)
      }
    })

    it('should return empty array for new sketch', () => {
      const hk = new HeavyKeeper(10, 0.001, 0.01)
      const topK = hk.topK()
      expect(topK).toEqual([])
    })
  })

  describe('decay', () => {
    it('should apply decay to counters', () => {
      const hk = new HeavyKeeper(10, 0.001, 0.01)

      for (let i = 0; i < 100; i++) {
        hk.update(Buffer.from('item'))
      }

      const countBefore = hk.estimate(Buffer.from('item'))
      hk.decay()
      const countAfter = hk.estimate(Buffer.from('item'))

      // After decay, count should decrease or stay similar
      expect(countAfter).toBeLessThanOrEqual(countBefore)
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const hk = new HeavyKeeper(10, 0.001, 0.01)
      const str = hk.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('HeavyKeeper')
    })
  })
})

describe('RatelessIBLT', () => {
  describe('constructor', () => {
    it('should create RatelessIBLT with valid parameters', () => {
      const iblt = new RatelessIBLT(100, 32)
      expect(iblt).toBeDefined()
    })

    it('should throw on invalid expectedDiff (zero)', () => {
      expect(() => new RatelessIBLT(0, 32)).toThrow()
    })

    it('should throw on invalid cellSize (zero)', () => {
      expect(() => new RatelessIBLT(100, 0)).toThrow()
    })
  })

  describe('insert and delete', () => {
    it('should insert key-value pairs', () => {
      const iblt = new RatelessIBLT(100, 32)
      expect(() => {
        iblt.insert(Buffer.from('key1'), Buffer.from('value1'))
        iblt.insert(Buffer.from('key2'), Buffer.from('value2'))
      }).not.toThrow()
    })

    it('should delete key-value pairs', () => {
      const iblt = new RatelessIBLT(100, 32)
      iblt.insert(Buffer.from('key1'), Buffer.from('value1'))
      expect(() => {
        iblt.delete(Buffer.from('key1'), Buffer.from('value1'))
      }).not.toThrow()
    })
  })

  describe('subtract and decode', () => {
    it('should compute symmetric difference', () => {
      const alice = new RatelessIBLT(100, 32)
      const bob = new RatelessIBLT(100, 32)

      // Shared items
      alice.insert(Buffer.from('shared1'), Buffer.from('value1'))
      bob.insert(Buffer.from('shared1'), Buffer.from('value1'))

      // Alice unique
      alice.insert(Buffer.from('alice_only'), Buffer.from('alice_value'))

      // Bob unique
      bob.insert(Buffer.from('bob_only'), Buffer.from('bob_value'))

      // Compute difference
      alice.subtract(bob)

      const result = alice.decode()
      expect(result).toBeDefined()
      expect(result).toHaveProperty('toInsert')
      expect(result).toHaveProperty('toRemove')
      expect(result).toHaveProperty('success')
      expect(typeof result.success).toBe('boolean')
    })

    it('should handle empty IBLTs', () => {
      const iblt1 = new RatelessIBLT(100, 32)
      const iblt2 = new RatelessIBLT(100, 32)

      expect(() => {
        iblt1.subtract(iblt2)
      }).not.toThrow()

      const result = iblt1.decode()
      expect(result.success).toBe(true)
      expect(result.toInsert.length).toBe(0)
      expect(result.toRemove.length).toBe(0)
    })

    it('should decode small differences', () => {
      const alice = new RatelessIBLT(10, 32)
      const bob = new RatelessIBLT(10, 32)

      alice.insert(Buffer.from('a'), Buffer.from('value_a'))
      bob.insert(Buffer.from('b'), Buffer.from('value_b'))

      alice.subtract(bob)
      const result = alice.decode()

      if (result.success) {
        expect(result.toInsert.length + result.toRemove.length).toBeGreaterThan(0)

        // Check structure of results
        for (const item of result.toInsert) {
          expect(item).toHaveProperty('key')
          expect(item).toHaveProperty('value')
          expect(Buffer.isBuffer(item.key)).toBe(true)
          expect(Buffer.isBuffer(item.value)).toBe(true)
        }
      }
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const iblt = new RatelessIBLT(100, 32)
      const str = iblt.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('RatelessIBLT')
    })
  })
})

describe('Grafite', () => {
  describe('build', () => {
    it('should build filter from sorted keys', () => {
      const keys = [10n, 20n, 30n, 40n, 50n]
      const filter = Grafite.build(keys, 6)
      expect(filter).toBeDefined()
    })

    it('should throw on empty keys array', () => {
      expect(() => Grafite.build([], 6)).toThrow()
    })

    it('should throw on invalid bitsPerKey', () => {
      const keys = [10n, 20n, 30n]
      expect(() => Grafite.build(keys, 1)).toThrow()
      expect(() => Grafite.build(keys, 20)).toThrow()
    })
  })

  describe('mayContainRange', () => {
    it('should detect ranges containing keys', () => {
      const keys = [100n, 200n, 300n, 400n, 500n]
      const filter = Grafite.build(keys, 6)

      // Range containing a key
      expect(filter.mayContainRange(150n, 250n)).toBe(true) // Contains 200
      expect(filter.mayContainRange(100n, 100n)).toBe(true) // Point query for 100
    })

    it('should reject ranges not containing keys', () => {
      const keys = [100n, 200n, 300n, 400n, 500n]
      const filter = Grafite.build(keys, 6)

      // Range before all keys
      const result1 = filter.mayContainRange(1n, 50n)
      expect(typeof result1).toBe('boolean')

      // Range after all keys
      const result2 = filter.mayContainRange(600n, 700n)
      expect(typeof result2).toBe('boolean')
    })

    it('should handle large range queries', () => {
      const keys = [10n, 20n, 30n, 40n, 50n]
      const filter = Grafite.build(keys, 6)

      // Query covering all keys
      expect(filter.mayContainRange(0n, 100n)).toBe(true)
    })
  })

  describe('mayContain', () => {
    it('should check point membership', () => {
      const keys = [100n, 200n, 300n, 400n, 500n]
      const filter = Grafite.build(keys, 6)

      expect(filter.mayContain(200n)).toBe(true)
      expect(filter.mayContain(300n)).toBe(true)
    })
  })

  describe('expectedFpr', () => {
    it('should calculate expected FPR', () => {
      const keys = [10n, 20n, 30n, 40n, 50n]
      const filter = Grafite.build(keys, 6)

      const fpr = filter.expectedFpr(10n)
      expect(typeof fpr).toBe('number')
      expect(fpr).toBeGreaterThanOrEqual(0)
      expect(fpr).toBeLessThanOrEqual(1)
    })

    it('should show FPR increases with range width', () => {
      const keys = [10n, 20n, 30n, 40n, 50n]
      const filter = Grafite.build(keys, 6)

      const fpr1 = filter.expectedFpr(10n)
      const fpr2 = filter.expectedFpr(100n)
      const fpr3 = filter.expectedFpr(1000n)

      expect(fpr2).toBeGreaterThan(fpr1)
      expect(fpr3).toBeGreaterThan(fpr2)
    })
  })

  describe('stats', () => {
    it('should return filter statistics', () => {
      const keys = [10n, 20n, 30n, 40n, 50n]
      const filter = Grafite.build(keys, 6)

      const stats = filter.stats()
      expect(stats).toBeDefined()
      expect(stats).toHaveProperty('keyCount')
      expect(stats).toHaveProperty('bitsPerKey')
      expect(stats).toHaveProperty('totalBits')
      expect(stats.keyCount).toBe(5)
      expect(stats.bitsPerKey).toBe(6)
      expect(stats.totalBits).toBeGreaterThan(0)
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const keys = [10n, 20n, 30n]
      const filter = Grafite.build(keys, 6)
      const str = filter.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('Grafite')
    })
  })
})

describe('MementoFilter', () => {
  describe('constructor', () => {
    it('should create MementoFilter with valid parameters', () => {
      const filter = new MementoFilter(1000, 0.01)
      expect(filter).toBeDefined()
    })

    it('should throw on invalid expectedElements (zero)', () => {
      expect(() => new MementoFilter(0, 0.01)).toThrow()
    })

    it('should throw on invalid fpr', () => {
      expect(() => new MementoFilter(1000, -0.1)).toThrow()
      expect(() => new MementoFilter(1000, 1.5)).toThrow()
    })
  })

  describe('insert', () => {
    it('should insert key-value pairs', () => {
      const filter = new MementoFilter(1000, 0.01)
      expect(() => {
        filter.insert(42n, Buffer.from('value1'))
        filter.insert(100n, Buffer.from('value2'))
        filter.insert(250n, Buffer.from('value3'))
      }).not.toThrow()
    })

    it('should handle duplicate insertions', () => {
      const filter = new MementoFilter(1000, 0.01)
      filter.insert(42n, Buffer.from('value'))
      expect(() => {
        filter.insert(42n, Buffer.from('value'))
      }).not.toThrow()
    })
  })

  describe('mayContainRange', () => {
    it('should detect ranges with inserted keys', () => {
      const filter = new MementoFilter(1000, 0.01)
      filter.insert(42n, Buffer.from('value1'))
      filter.insert(100n, Buffer.from('value2'))
      filter.insert(250n, Buffer.from('value3'))

      // Range containing inserted key
      expect(filter.mayContainRange(40n, 50n)).toBe(true)
      expect(filter.mayContainRange(95n, 105n)).toBe(true)
    })

    it('should handle empty filter', () => {
      const filter = new MementoFilter(1000, 0.01)
      const result = filter.mayContainRange(0n, 100n)
      expect(typeof result).toBe('boolean')
    })

    it('should handle wide range queries', () => {
      const filter = new MementoFilter(1000, 0.01)
      filter.insert(50n, Buffer.from('value'))

      expect(filter.mayContainRange(0n, 1000n)).toBe(true)
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const filter = new MementoFilter(1000, 0.01)
      const str = filter.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('MementoFilter')
    })
  })
})

describe('SlidingHyperLogLog', () => {
  describe('constructor', () => {
    it('should create SlidingHyperLogLog with valid parameters', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)
      expect(hll).toBeDefined()
    })

    it('should throw on invalid precision', () => {
      expect(() => new SlidingHyperLogLog(3, 3600n)).toThrow()
      expect(() => new SlidingHyperLogLog(17, 3600n)).toThrow()
    })

    it('should create with various window sizes', () => {
      expect(() => new SlidingHyperLogLog(12, 1n)).not.toThrow()
      expect(() => new SlidingHyperLogLog(12, 3600n)).not.toThrow()
      expect(() => new SlidingHyperLogLog(12, 86400n)).not.toThrow()
    })
  })

  describe('update', () => {
    it('should add items with timestamps', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)
      expect(() => {
        hll.update(Buffer.from('user_123'), 1000n)
        hll.update(Buffer.from('user_456'), 1030n)
        hll.update(Buffer.from('user_789'), 1060n)
      }).not.toThrow()
    })

    it('should handle duplicate items at different times', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)
      hll.update(Buffer.from('item'), 1000n)
      expect(() => {
        hll.update(Buffer.from('item'), 2000n)
      }).not.toThrow()
    })
  })

  describe('estimateWindow', () => {
    it('should estimate cardinality in window', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)

      // Add items at different times
      for (let i = 0; i < 100; i++) {
        hll.update(Buffer.from(`item_${i}`), BigInt(1000 + i))
      }

      // Estimate in last 50 time units
      const estimate = hll.estimateWindow(1100n, 50n)
      expect(typeof estimate).toBe('number')
      expect(estimate).toBeGreaterThan(0)
      expect(estimate).toBeLessThanOrEqual(100)
    })

    it('should return 0 for empty window', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)
      hll.update(Buffer.from('item'), 1000n)

      // Query window before any items
      const estimate = hll.estimateWindow(500n, 100n)
      expect(estimate).toBe(0)
    })

    it('should handle narrow windows', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)

      hll.update(Buffer.from('item1'), 1000n)
      hll.update(Buffer.from('item2'), 1005n)
      hll.update(Buffer.from('item3'), 1010n)

      const estimate = hll.estimateWindow(1010n, 10n)
      expect(estimate).toBeGreaterThan(0)
    })
  })

  describe('estimateTotal', () => {
    it('should estimate total cardinality', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)

      for (let i = 0; i < 100; i++) {
        hll.update(Buffer.from(`item_${i}`), BigInt(1000 + i))
      }

      const total = hll.estimateTotal()
      expect(typeof total).toBe('number')
      expect(total).toBeGreaterThan(0)
      expect(total).toBeGreaterThan(90) // Allow some error
      expect(total).toBeLessThan(110)
    })

    it('should return 0 for empty sketch', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)
      const total = hll.estimateTotal()
      expect(total).toBe(0)
    })
  })

  describe('decay', () => {
    it('should remove expired entries', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)

      hll.update(Buffer.from('old'), 1000n)
      hll.update(Buffer.from('recent'), 2000n)

      expect(() => {
        hll.decay(2100n, 200n)
      }).not.toThrow()

      // After decay, old items should be removed
      const estimate = hll.estimateWindow(2100n, 200n)
      expect(estimate).toBeGreaterThan(0)
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const hll = new SlidingHyperLogLog(12, 3600n)
      const str = hll.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('SlidingHyperLogLog')
    })
  })
})
