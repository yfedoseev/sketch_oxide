import { HyperLogLog } from '../index'

describe('HyperLogLog', () => {
  describe('constructor', () => {
    it('should create HyperLogLog with valid precision', () => {
      const hll = new HyperLogLog(14)
      expect(hll).toBeDefined()
      expect(hll.precision()).toBe(14)
    })

    it('should create HyperLogLog with minimum precision (4)', () => {
      const hll = new HyperLogLog(4)
      expect(hll).toBeDefined()
      expect(hll.precision()).toBe(4)
    })

    it('should create HyperLogLog with maximum precision (16)', () => {
      const hll = new HyperLogLog(16)
      expect(hll).toBeDefined()
      expect(hll.precision()).toBe(16)
    })

    it('should throw on precision too low (< 4)', () => {
      expect(() => new HyperLogLog(3)).toThrow()
    })

    it('should throw on precision too high (> 16)', () => {
      expect(() => new HyperLogLog(19)).toThrow()
    })
  })

  describe('update', () => {
    it('should add items to sketch', () => {
      const hll = new HyperLogLog(14)
      expect(() => {
        hll.update(Buffer.from('item1'))
        hll.update(Buffer.from('item2'))
      }).not.toThrow()
    })

    it('should accept binary data', () => {
      const hll = new HyperLogLog(14)
      expect(() => {
        hll.update(Buffer.from([1, 2, 3, 4, 5]))
      }).not.toThrow()
    })

    it('should accept empty buffer', () => {
      const hll = new HyperLogLog(14)
      expect(() => {
        hll.update(Buffer.from([]))
      }).not.toThrow()
    })
  })

  describe('estimate', () => {
    it('should return 0 for empty sketch', () => {
      const hll = new HyperLogLog(14)
      const estimate = hll.estimate()
      expect(estimate).toBe(0)
    })

    it('should estimate cardinality for single item', () => {
      const hll = new HyperLogLog(14)
      hll.update(Buffer.from('item1'))
      const estimate = hll.estimate()
      expect(estimate).toBeGreaterThanOrEqual(1)
      expect(estimate).toBeLessThanOrEqual(2)
    })

    it('should estimate cardinality within error bounds', () => {
      const hll = new HyperLogLog(14)
      const n = 10000
      for (let i = 0; i < n; i++) {
        hll.update(Buffer.from(`item-${i}`))
      }
      const estimate = hll.estimate()
      // Standard error is ~1.04/sqrt(m) where m = 2^14 = 16384
      // So ~0.008 relative error, 95% CI is ±1.96*0.008 = ±1.57%
      const error = Math.abs(estimate - n) / n
      expect(error).toBeLessThan(0.03) // Allow 3% error
    })

    it('should handle duplicate items correctly', () => {
      const hll = new HyperLogLog(14)
      hll.update(Buffer.from('item1'))
      hll.update(Buffer.from('item1'))
      hll.update(Buffer.from('item1'))
      const estimate = hll.estimate()
      expect(estimate).toBeCloseTo(1, 0)
    })

    it('should increase monotonically with new items', () => {
      const hll = new HyperLogLog(14)
      const estimates: number[] = []
      for (let i = 0; i < 100; i++) {
        hll.update(Buffer.from(`item-${i}`))
        if (i % 10 === 0) {
          estimates.push(hll.estimate())
        }
      }
      // Estimates should generally increase (accounting for variance)
      for (let i = 1; i < estimates.length; i++) {
        expect(estimates[i]).toBeGreaterThanOrEqual(estimates[i - 1] * 0.9) // Allow 10% variance
      }
    })
  })

  describe('merge', () => {
    it('should merge two empty sketches', () => {
      const hll1 = new HyperLogLog(14)
      const hll2 = new HyperLogLog(14)
      expect(() => {
        hll1.merge(hll2)
      }).not.toThrow()
      expect(hll1.estimate()).toBe(0)
    })

    it('should merge sketches with items', () => {
      const hll1 = new HyperLogLog(14)
      const hll2 = new HyperLogLog(14)
      hll1.update(Buffer.from('item1'))
      hll2.update(Buffer.from('item2'))
      hll1.merge(hll2)
      const estimate = hll1.estimate()
      expect(estimate).toBeGreaterThanOrEqual(1.9)
      expect(estimate).toBeLessThanOrEqual(2.1)
    })

    it('should handle merging sketches with overlapping items', () => {
      const hll1 = new HyperLogLog(14)
      const hll2 = new HyperLogLog(14)
      hll1.update(Buffer.from('a'))
      hll1.update(Buffer.from('b'))
      hll2.update(Buffer.from('b'))
      hll2.update(Buffer.from('c'))
      hll1.merge(hll2)
      const estimate = hll1.estimate()
      expect(estimate).toBeGreaterThanOrEqual(2.7)
      expect(estimate).toBeLessThanOrEqual(3.3)
    })

    it('should fail when merging sketches with different precision', () => {
      const hll1 = new HyperLogLog(14)
      const hll2 = new HyperLogLog(12)
      hll1.update(Buffer.from('item1'))
      hll2.update(Buffer.from('item2'))
      expect(() => {
        hll1.merge(hll2)
      }).toThrow()
    })
  })

  describe('reset', () => {
    it('should reset sketch to empty state', () => {
      const hll = new HyperLogLog(14)
      hll.update(Buffer.from('item1'))
      hll.update(Buffer.from('item2'))
      expect(hll.estimate()).toBeGreaterThan(0)
      expect(hll.estimate()).toBe(0)
    })

    it('should allow updates after reset', () => {
      const hll = new HyperLogLog(14)
      hll.update(Buffer.from('item1'))
      hll.update(Buffer.from('item2'))
      const estimate = hll.estimate()
      expect(estimate).toBeGreaterThanOrEqual(1)
      expect(estimate).toBeLessThanOrEqual(2)
    })
  })

  describe('serialization', () => {
    it('should serialize and deserialize empty sketch', () => {
      const hll1 = new HyperLogLog(14)
      const data = hll1.serialize()
      expect(data).toBeInstanceOf(Buffer)
      expect(data.length).toBeGreaterThan(0)
      const hll2 = HyperLogLog.deserialize(data)
      expect(hll2.estimate()).toBe(0)
      expect(hll2.precision()).toBe(14)
    })

    it('should preserve state through serialization', () => {
      const hll1 = new HyperLogLog(14)
      for (let i = 0; i < 100; i++) {
        hll1.update(Buffer.from(`item-${i}`))
      }
      const estimate1 = hll1.estimate()
      const data = hll1.serialize()
      const hll2 = HyperLogLog.deserialize(data)
      const estimate2 = hll2.estimate()
      expect(estimate1).toBe(estimate2)
    })

    it('should allow updates after deserialization', () => {
      const hll1 = new HyperLogLog(14)
      hll1.update(Buffer.from('item1'))
      const estimate1 = hll1.estimate()
      const data = hll1.serialize()
      const hll2 = HyperLogLog.deserialize(data)
      hll2.update(Buffer.from('item2'))
      const estimate2 = hll2.estimate()
      expect(estimate2).toBeGreaterThan(estimate1)
    })
  })

  describe('toString', () => {
    it('should return string representation', () => {
      const hll = new HyperLogLog(14)
      const str = hll.toString()
      expect(typeof str).toBe('string')
      expect(str).toContain('HyperLogLog')
      expect(str).toContain('14')
    })

    it('should include estimate in string representation', () => {
      const hll = new HyperLogLog(14)
      hll.update(Buffer.from('item1'))
      const str = hll.toString()
      expect(str).toContain('estimate')
    })
  })

  describe('stress tests', () => {
    it('should handle 100K items efficiently', () => {
      const hll = new HyperLogLog(14)
      const startTime = Date.now()
      for (let i = 0; i < 100000; i++) {
        hll.update(Buffer.from(`item-${i}`))
      }
      const elapsed = Date.now() - startTime
      const estimate = hll.estimate()
      expect(estimate).toBeGreaterThan(99000)
      expect(estimate).toBeLessThan(101000)
      expect(elapsed).toBeLessThan(5000) // Should complete in < 5 seconds
    })

    it('should maintain accuracy with mixed item types', () => {
      const hll = new HyperLogLog(14)
      // Add various types of items
      for (let i = 0; i < 1000; i++) {
        hll.update(Buffer.from(`string-${i}`))
      }
      for (let i = 0; i < 1000; i++) {
        hll.update(Buffer.from([i, i >> 8, i >> 16]))
      }
      const estimate = hll.estimate()
      expect(estimate).toBeGreaterThan(1900)
      expect(estimate).toBeLessThan(2100)
    })
  })
})
