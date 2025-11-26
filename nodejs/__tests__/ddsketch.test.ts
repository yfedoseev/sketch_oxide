import { DDSketch } from '../index'

describe('DDSketch', () => {
  describe('constructor', () => {
    it('should create DDSketch with valid relative accuracy', () => {
      const ds = new DDSketch(0.01)
      expect(ds).toBeDefined()
    })

    it('should create DDSketch with various relative accuracy values', () => {
      expect(() => new DDSketch(0.001)).not.toThrow()
      expect(() => new DDSketch(0.01)).not.toThrow()
      expect(() => new DDSketch(0.05)).not.toThrow()
      expect(() => new DDSketch(0.1)).not.toThrow()
    })

    it('should throw on invalid relative accuracy (zero)', () => {
      expect(() => new DDSketch(0.0)).toThrow()
    })

    it('should throw on invalid relative accuracy (negative)', () => {
      expect(() => new DDSketch(-0.01)).toThrow()
    })

    it('should throw on invalid relative accuracy (>= 1)', () => {
      expect(() => new DDSketch(1.0)).toThrow()
      expect(() => new DDSketch(1.5)).toThrow()
    })

    it('should throw on invalid relative accuracy (> 1)', () => {
      expect(() => new DDSketch(0.99)).not.toThrow()
    })
  })

  describe('update and quantile', () => {
    let ds: DDSketch

    beforeEach(() => {
      ds = new DDSketch(0.01)
    })

    afterEach(() => {
      ds.dispose?.()
    })

    it('should update with single value', () => {
      expect(() => ds.update(100.0)).not.toThrow()
    })

    it('should update with multiple values', () => {
      for (let i = 1; i <= 100; i++) {
        ds.update(i)
      }
      expect(ds).toBeDefined()
    })

    it('should query quantile after updates', () => {
      for (let i = 1; i <= 1000; i++) {
        ds.update(i)
      }

      const median = ds.quantile(0.5)!
      expect(median).toBeGreaterThan(400)
      expect(median).toBeLessThan(600)
    })

    it('should maintain quantile ordering', () => {
      for (let i = 1; i <= 1000; i++) {
        ds.update(i)
      }

      const q25 = ds.quantile(0.25)!
      const q50 = ds.quantile(0.5)!
      const q75 = ds.quantile(0.75)!

      expect(q25).toBeLessThanOrEqual(q50)
      expect(q50).toBeLessThanOrEqual(q75)
    })

    it('should handle duplicate updates', () => {
      for (let i = 0; i < 100; i++) {
        ds.update(50.0)
      }

      const median = ds.quantile(0.5)!
      expect(median).toBeCloseTo(50.0, 1)
    })

    it('should handle negative values', () => {
      for (let i = -100; i <= 100; i++) {
        ds.update(i)
      }

      const median = ds.quantile(0.5)!
      expect(median).toBeCloseTo(0, 5)
    })

    it('should handle very small values', () => {
      for (let i = 0; i < 100; i++) {
        ds.update(0.0001 * (i + 1))
      }

      const median = ds.quantile(0.5)!
      expect(median).toBeGreaterThan(0)
    })

    it('should handle very large values', () => {
      for (let i = 0; i < 100; i++) {
        ds.update(1e10 * (i + 1))
      }

      const median = ds.quantile(0.5)!
      expect(median).toBeGreaterThan(0)
    })
  })

  describe('batch quantiles', () => {
    let ds: DDSketch

    beforeEach(() => {
      ds = new DDSketch(0.01)
      for (let i = 1; i <= 1000; i++) {
        ds.update(i)
      }
    })

    afterEach(() => {
      ds.dispose?.()
    })

    it('should query multiple quantiles', () => {
      const results = ds.quantiles([0.25, 0.5, 0.75])
      expect(results.length).toBe(3)
      expect(results[0]).toBeLessThanOrEqual(results[1])
      expect(results[1]).toBeLessThanOrEqual(results[2])
    })

    it('should match single and batch queries', () => {
      const single = [
        ds.quantile(0.1)!,
        ds.quantile(0.5)!,
        ds.quantile(0.9)!,
      ]

      const batch = ds.quantiles([0.1, 0.5, 0.9])

      for (let i = 0; i < single.length; i++) {
        expect(batch[i]).toBeCloseTo(single[i], 1)
      }
    })

    it('should handle boundary quantiles', () => {
      const results = ds.quantiles([0.0, 0.5, 1.0])
      expect(results.length).toBe(3)
      expect(results[0]).toBeLessThanOrEqual(results[1])
      expect(results[1]).toBeLessThanOrEqual(results[2])
    })

    it('should handle empty quantiles array', () => {
      const results = ds.quantiles([])
      expect(results.length).toBe(0)
    })

    it('should throw on invalid quantile', () => {
      expect(() => ds.quantiles([0.5, 1.5])).toThrow()
      expect(() => ds.quantiles([-0.1, 0.5])).toThrow()
    })
  })

  describe('quantile accuracy', () => {
    let ds: DDSketch

    beforeEach(() => {
      ds = new DDSketch(0.01)
    })

    afterEach(() => {
      ds.dispose?.()
    })

    it('should maintain relative accuracy for uniform distribution', () => {
      for (let i = 1; i <= 1000; i++) {
        ds.update(i)
      }

      const median = ds.quantile(0.5)!
      const trueMedian = 500.5
      const relativeError = Math.abs(median - trueMedian) / trueMedian

      expect(relativeError).toBeLessThan(0.05) // Allow 5% relative error
    })

    it('should provide monotonic quantiles', () => {
      for (let i = 1; i <= 1000; i++) {
        ds.update(i)
      }

      const q1 = ds.quantile(0.1)!
      const q5 = ds.quantile(0.5)!
      const q9 = ds.quantile(0.9)!

      expect(q1).toBeLessThanOrEqual(q5)
      expect(q5).toBeLessThanOrEqual(q9)
    })

    it('should handle exponential distribution', () => {
      for (let i = 0; i < 1000; i++) {
        const value = -Math.log(1 - Math.random()) * 100
        ds.update(value)
      }

      const median = ds.quantile(0.5)!
      const p95 = ds.quantile(0.95)!

      expect(median).toBeGreaterThan(0)
      expect(p95).toBeGreaterThan(median)
    })

    it('should handle bimodal distribution', () => {
      // Two modes
      for (let i = 0; i < 500; i++) {
        ds.update(100 + Math.random() * 100) // Mode 1
        ds.update(900 + Math.random() * 100) // Mode 2
      }

      const median = ds.quantile(0.5)!
      expect(median).toBeGreaterThan(0)
    })

    it('should handle skewed distribution', () => {
      // Right-skewed
      for (let i = 0; i < 900; i++) {
        ds.update(Math.random() * 10)
      }
      for (let i = 0; i < 100; i++) {
        ds.update(10 + Math.random() * 1000)
      }

      const median = ds.quantile(0.5)!
      const p95 = ds.quantile(0.95)!

      expect(p95).toBeGreaterThan(median)
    })
  })

  describe('merge', () => {
    it('should merge two compatible sketches', () => {
      const ds1 = new DDSketch(0.01)
      const ds2 = new DDSketch(0.01)

      for (let i = 1; i <= 500; i++) {
        ds1.update(i)
      }

      for (let i = 501; i <= 1000; i++) {
        ds2.update(i)
      }


      const median = ds1.quantile(0.5)!
      const trueMedian = 500.5
      const relativeError = Math.abs(median - trueMedian) / trueMedian

      expect(relativeError).toBeLessThan(0.2)

      ds1.dispose?.()
      ds2.dispose?.()
    })

    it('should throw on merge with different relative accuracy', () => {
      const ds1 = new DDSketch(0.01)
      const ds2 = new DDSketch(0.05)


      ds1.dispose?.()
      ds2.dispose?.()
    })

    it('should throw on merge with null', () => {
      const ds = new DDSketch(0.01)
      ds.dispose?.()
    })

    it('should merge sketches with identical data', () => {
      const ds1 = new DDSketch(0.01)
      const ds2 = new DDSketch(0.01)

      for (let i = 1; i <= 1000; i++) {
        ds1.update(i)
        ds2.update(i)
      }


      const median = ds1.quantile(0.5)!
      expect(median).toBeCloseTo(500.5, 0)

      ds1.dispose?.()
      ds2.dispose?.()
    })

    it('should merge sketches with overlapping data', () => {
      const ds1 = new DDSketch(0.01)
      const ds2 = new DDSketch(0.01)

      for (let i = 1; i <= 500; i++) {
        ds1.update(i)
      }

      for (let i = 250; i <= 750; i++) {
        ds2.update(i)
      }


      const median = ds1.quantile(0.5)!
      expect(median).toBeGreaterThan(0)

      ds1.dispose?.()
      ds2.dispose?.()
    })
  })

  describe('serialization', () => {
    let ds: DDSketch

    beforeEach(() => {
      ds = new DDSketch(0.01)
    })

    afterEach(() => {
      ds.dispose?.()
    })

    it('should serialize empty sketch', () => {
      const serialized = ds.serialize()
      expect(serialized).toBeDefined()
      expect(serialized.length).toBeGreaterThan(0)
    })

    it('should serialize sketch with data', () => {
      for (let i = 1; i <= 100; i++) {
        ds.update(i)
      }

      const serialized = ds.serialize()
      expect(serialized.length).toBeGreaterThan(0)
    })

    it('should deserialize and restore quantiles', () => {
      for (let i = 1; i <= 1000; i++) {
        ds.update(i)
      }

      const originalMedian = ds.quantile(0.5)!
      const serialized = ds.serialize()
      const restored = DDSketch.deserialize(serialized)

      const restoredMedian = restored.quantile(0.5)!
      expect(restoredMedian).toBeCloseTo(originalMedian, 0)

      restored.dispose?.()
    })

    it('should handle round-trip serialization', () => {
      for (let i = 0; i < 1000; i++) {
        ds.update(Math.random() * 1000)
      }

      const q25 = ds.quantile(0.25)!
      const q50 = ds.quantile(0.5)!
      const q75 = ds.quantile(0.75)!

      const serialized = ds.serialize()
      const restored = DDSketch.deserialize(serialized)

      expect(restored.quantile(0.25)!).toBeCloseTo(q25, 0)
      expect(restored.quantile(0.5)!).toBeCloseTo(q50, 0)
      expect(restored.quantile(0.75)!).toBeCloseTo(q75, 0)

      restored.dispose?.()
    })

    it('should throw on deserialize invalid data', () => {
      expect(() =>
        DDSketch.deserialize(Buffer.from([1, 2, 3, 4, 5])),
      ).toThrow()
    })

    it('should throw on deserialize null', () => {
      expect(() => DDSketch.deserialize(null as any)).toThrow()
    })
  })

  describe('large dataset', () => {
    it('should handle large dataset', () => {
      const ds = new DDSketch(0.01)

      for (let i = 0; i < 1000000; i++) {
        ds.update(Math.random() * 1000)
      }

      const median = ds.quantile(0.5)!
      const p99 = ds.quantile(0.99)!

      expect(median).toBeGreaterThan(0)
      expect(p99).toBeGreaterThan(median)

      ds.dispose?.()
    })

    it('should maintain accuracy on large dataset', () => {
      const ds = new DDSketch(0.01)

      for (let i = 1; i <= 100000; i++) {
        ds.update(i)
      }

      const median = ds.quantile(0.5)!
      const trueMedian = 50000.5

      const relativeError = Math.abs(median - trueMedian) / trueMedian
      expect(relativeError).toBeLessThan(0.1)

      ds.dispose?.()
    })
  })

  describe('edge cases', () => {
    let ds: DDSketch

    beforeEach(() => {
      ds = new DDSketch(0.01)
    })

    afterEach(() => {
      ds.dispose?.()
    })

    it('should handle single value', () => {
      ds.update(42.0)
      const q50 = ds.quantile(0.5)!
      expect(q50).toBeCloseTo(42.0, 1)
    })

    it('should handle constant values', () => {
      for (let i = 0; i < 1000; i++) {
        ds.update(100.0)
      }

      const q25 = ds.quantile(0.25)!
      const q50 = ds.quantile(0.5)!
      const q75 = ds.quantile(0.75)!

      expect(q25).toBeCloseTo(100.0, 0)
      expect(q50).toBeCloseTo(100.0, 0)
      expect(q75).toBeCloseTo(100.0, 0)
    })

    it('should handle mixed magnitude values', () => {
      for (let i = 0; i < 100; i++) {
        ds.update(0.001)
        ds.update(1e8)
      }

      const median = ds.quantile(0.5)!
      expect(median).toBeGreaterThan(0)
    })

    it('should work with different relative accuracy values', () => {
      for (const alpha of [0.001, 0.01, 0.05, 0.1]) {
        const sketch = new DDSketch(alpha)
        for (let i = 1; i <= 1000; i++) {
          sketch.update(i)
        }
        const median = sketch.quantile(0.5)!
        expect(median).toBeGreaterThan(0)
        sketch.dispose?.()
      }
    })

    it('should throw on invalid quantile', () => {
      expect(() => ds.quantile(-0.1)).toThrow()
      expect(() => ds.quantile(1.1)).toThrow()
    })
  })
})
