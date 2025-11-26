import { BloomFilter } from '../index'

describe('BloomFilter', () => {
  describe('constructor', () => {
    it('should create BloomFilter with valid parameters', () => {
      const bf = new BloomFilter(10000, 0.01)
      expect(bf).toBeDefined()
    })

    it('should create BloomFilter with different capacities', () => {
      expect(() => new BloomFilter(100, 0.01)).not.toThrow()
      expect(() => new BloomFilter(1000, 0.01)).not.toThrow()
      expect(() => new BloomFilter(100000, 0.01)).not.toThrow()
    })

    it('should create BloomFilter with different FPR values', () => {
      expect(() => new BloomFilter(1000, 0.001)).not.toThrow()
      expect(() => new BloomFilter(1000, 0.01)).not.toThrow()
      expect(() => new BloomFilter(1000, 0.1)).not.toThrow()
    })

    it('should throw on invalid n (negative)', () => {
      expect(() => new BloomFilter(-1000, 0.01)).toThrow()
    })

    it('should throw on invalid n (zero)', () => {
      expect(() => new BloomFilter(0, 0.01)).toThrow()
    })

    it('should throw on invalid fpr (too low)', () => {
      expect(() => new BloomFilter(1000, 0.0)).toThrow()
      expect(() => new BloomFilter(1000, -0.1)).toThrow()
    })

    it('should throw on invalid fpr (too high)', () => {
      expect(() => new BloomFilter(1000, 1.0)).toThrow()
      expect(() => new BloomFilter(1000, 1.5)).toThrow()
    })
  })

  describe('insert and contains', () => {
    let bf: BloomFilter

    beforeEach(() => {
      bf = new BloomFilter(10000, 0.01)
    })

    afterEach(() => {
      bf.dispose?.()
    })

    it('should find inserted items', () => {
      bf.insert(Buffer.from('test-item'))
      expect(bf.contains(Buffer.from('test-item'))).toBe(true)
    })

    it('should handle multiple inserts', () => {
      for (let i = 0; i < 100; i++) {
        bf.insert(Buffer.from(`item-${i}`))
      }

      for (let i = 0; i < 100; i++) {
        expect(bf.contains(Buffer.from(`item-${i}`))).toBe(true)
      }
    })

    it('should handle duplicate inserts', () => {
      bf.insert(Buffer.from('duplicate'))
      bf.insert(Buffer.from('duplicate'))
      bf.insert(Buffer.from('duplicate'))

      expect(bf.contains(Buffer.from('duplicate'))).toBe(true)
    })

    it('should guarantee no false negatives', () => {
      const items = ['apple', 'banana', 'cherry', 'date', 'elderberry']

      for (const item of items) {
        bf.insert(Buffer.from(item))
      }

      for (const item of items) {
        expect(bf.contains(Buffer.from(item))).toBe(true)
      }
    })

    it('should handle empty buffer', () => {
      bf.insert(Buffer.from([]))
      expect(bf.contains(Buffer.from([]))).toBe(true)
    })

    it('should handle binary data', () => {
      const binary = Buffer.from([0, 1, 2, 3, 127, 255])
      bf.insert(binary)
      expect(bf.contains(binary)).toBe(true)
    })

    it('should handle unicode strings', () => {
      const items = ['你好', 'مرحبا', 'Привет', 'こんにちは']

      for (const item of items) {
        bf.insert(Buffer.from(item))
      }

      for (const item of items) {
        expect(bf.contains(Buffer.from(item))).toBe(true)
      }
    })

    it('should distinguish similar items', () => {
      bf.insert(Buffer.from('test'))
      bf.insert(Buffer.from('tests'))
      bf.insert(Buffer.from('testing'))

      expect(bf.contains(Buffer.from('test'))).toBe(true)
      expect(bf.contains(Buffer.from('tests'))).toBe(true)
      expect(bf.contains(Buffer.from('testing'))).toBe(true)
      expect(bf.contains(Buffer.from('tester'))).toBe(false)
    })
  })

  describe('batch operations', () => {
    let bf: BloomFilter

    beforeEach(() => {
      bf = new BloomFilter(10000, 0.01)
    })

    afterEach(() => {
      bf.dispose?.()
    })

    it('should insert batch of items', () => {
      const items = [
        Buffer.from('item1'),
        Buffer.from('item2'),
        Buffer.from('item3'),
        Buffer.from('item4'),
        Buffer.from('item5'),
      ]

      bf.insertBatch(...items)

      for (const item of items) {
        expect(bf.contains(item)).toBe(true)
      }
    })

    it('should check batch of items', () => {
      bf.insert(Buffer.from('apple'))
      bf.insert(Buffer.from('banana'))
      bf.insert(Buffer.from('cherry'))

      const results = bf.containsBatch(
        Buffer.from('apple'),
        Buffer.from('banana'),
        Buffer.from('cherry'),
        Buffer.from('date'),
      )

      expect(results).toEqual([true, true, true, false])
    })

    it('should handle empty batch', () => {
      expect(() => bf.insertBatch()).not.toThrow()
      expect(bf.containsBatch()).toEqual([])
    })

    it('should handle large batch', () => {
      const items = Array.from({ length: 1000 }, (_, i) =>
        Buffer.from(`item-${i}`),
      )

      bf.insertBatch(...items)

      for (let i = 0; i < 1000; i += 100) {
        expect(bf.contains(Buffer.from(`item-${i}`))).toBe(true)
      }
    })
  })


  describe('serialization', () => {
    let bf: BloomFilter

    beforeEach(() => {
      bf = new BloomFilter(10000, 0.01)
    })

    afterEach(() => {
      bf.dispose?.()
    })

    it('should serialize empty filter', () => {
      const serialized = bf.serialize()
      expect(serialized).toBeDefined()
      expect(serialized.length).toBeGreaterThan(0)
    })

    it('should serialize filter with data', () => {
      bf.insert(Buffer.from('item1'))
      bf.insert(Buffer.from('item2'))
      bf.insert(Buffer.from('item3'))

      const serialized = bf.serialize()
      expect(serialized.length).toBeGreaterThan(0)
    })

    it('should deserialize and restore data', () => {
      bf.insert(Buffer.from('apple'))
      bf.insert(Buffer.from('banana'))
      bf.insert(Buffer.from('cherry'))

      const serialized = bf.serialize()
      const restored = BloomFilter.deserialize(serialized)

      expect(restored.contains(Buffer.from('apple'))).toBe(true)
      expect(restored.contains(Buffer.from('banana'))).toBe(true)
      expect(restored.contains(Buffer.from('cherry'))).toBe(true)

      restored.dispose?.()
    })

    it('should handle round-trip serialization', () => {
      for (let i = 0; i < 100; i++) {
        bf.insert(Buffer.from(`item-${i}`))
      }

      const serialized = bf.serialize()
      const restored = BloomFilter.deserialize(serialized)

      for (let i = 0; i < 100; i++) {
        expect(restored.contains(Buffer.from(`item-${i}`))).toBe(true)
      }

      restored.dispose?.()
    })

    it('should throw on deserialize invalid data', () => {
      expect(() =>
        BloomFilter.deserialize(Buffer.from([1, 2, 3, 4, 5])),
      ).toThrow()
    })

    it('should throw on deserialize null', () => {
      expect(() => BloomFilter.deserialize(null as any)).toThrow()
    })
  })

  describe('large dataset', () => {
    it('should handle large dataset', () => {
      const bf = new BloomFilter(100000, 0.01)

      for (let i = 0; i < 50000; i++) {
        bf.insert(Buffer.from(`item-${i}`))
      }

      // Sample verification
      for (let i = 0; i < 1000; i += 10) {
        expect(bf.contains(Buffer.from(`item-${i}`))).toBe(true)
      }

      bf.dispose?.()
    })

    it('should maintain false positive rate on large dataset', () => {
      const bf = new BloomFilter(100000, 0.01)

      // Insert items
      for (let i = 0; i < 50000; i++) {
        bf.insert(Buffer.from(`item-${i}`))
      }

      // Check false positive rate on non-existent items
      let falsePositives = 0
      const testSize = 10000
      for (let i = 50000; i < 50000 + testSize; i++) {
        if (bf.contains(Buffer.from(`item-${i}`))) {
          falsePositives++
        }
      }

      const observedFpr = falsePositives / testSize
      // Should be roughly within 2x of target FPR due to variance
      expect(observedFpr).toBeLessThan(0.05)

      bf.dispose?.()
    })
  })

  describe('edge cases', () => {
    let bf: BloomFilter

    beforeEach(() => {
      bf = new BloomFilter(10000, 0.01)
    })

    afterEach(() => {
      bf.dispose?.()
    })

    it('should handle very small filter', () => {
      const small = new BloomFilter(10, 0.1)
      small.insert(Buffer.from('item'))
      expect(small.contains(Buffer.from('item'))).toBe(true)
      small.dispose?.()
    })

    it('should handle very large items', () => {
      const large = Buffer.alloc(10000)
      large.fill('x')
      bf.insert(large)
      expect(bf.contains(large)).toBe(true)
    })

    it('should handle different FPR values', () => {
      const high = new BloomFilter(1000, 0.5)
      const low = new BloomFilter(1000, 0.001)

      high.insert(Buffer.from('test'))
      low.insert(Buffer.from('test'))

      expect(high.contains(Buffer.from('test'))).toBe(true)
      expect(low.contains(Buffer.from('test'))).toBe(true)

      high.dispose?.()
      low.dispose?.()
    })

    it('should handle many duplicates', () => {
      for (let i = 0; i < 1000; i++) {
        bf.insert(Buffer.from('same-item'))
      }

      expect(bf.contains(Buffer.from('same-item'))).toBe(true)
    })
  })

  describe('disposal', () => {
    it('should dispose filter cleanly', () => {
      const bf = new BloomFilter(1000, 0.01)
      bf.insert(Buffer.from('test'))
      expect(() => bf.dispose?.()).not.toThrow()
    })

    it('should allow multiple dispose calls', () => {
      const bf = new BloomFilter(1000, 0.01)
      expect(() => {
        bf.dispose?.()
        bf.dispose?.()
        bf.dispose?.()
      }).not.toThrow()
    })
  })
})
