import { MinHash } from '../index'

describe('MinHash', () => {
  describe('constructor', () => {
    it('should create MinHash with valid numPerm', () => {
      const mh = new MinHash(128)
      expect(mh).toBeDefined()
    })

    it('should create MinHash with various numPerm values', () => {
      expect(() => new MinHash(8)).not.toThrow()
      expect(() => new MinHash(64)).not.toThrow()
      expect(() => new MinHash(256)).not.toThrow()
      expect(() => new MinHash(512)).not.toThrow()
    })

    it('should throw on zero numPerm', () => {
      expect(() => new MinHash(0)).toThrow()
    })

    it('should throw on negative numPerm', () => {
      expect(() => new MinHash(-128)).toThrow()
    })
  })

  describe('update', () => {
    let mh: MinHash

    beforeEach(() => {
      mh = new MinHash(128)
    })

    afterEach(() => {
    })

    it('should add elements to MinHash', () => {
      expect(() => mh.update(Buffer.from('element'))).not.toThrow()
    })

    it('should handle multiple updates', () => {
      for (let i = 0; i < 100; i++) {
        mh.update(Buffer.from(`element-${i}`))
      }
      expect(mh).toBeDefined()
    })

    it('should handle duplicate elements', () => {
      mh.update(Buffer.from('element'))
      mh.update(Buffer.from('element'))
      mh.update(Buffer.from('element'))
      expect(mh).toBeDefined()
    })

    it('should handle binary data', () => {
      const binary = Buffer.from([0, 1, 2, 3, 127, 255])
      mh.update(binary)
      expect(mh).toBeDefined()
    })

    it('should handle empty buffer', () => {
      mh.update(Buffer.from([]))
      expect(mh).toBeDefined()
    })

    it('should handle unicode strings', () => {
      mh.update(Buffer.from('你好'))
      mh.update(Buffer.from('مرحبا'))
      mh.update(Buffer.from('Привет'))
      mh.update(Buffer.from('こんにちは'))
      expect(mh).toBeDefined()
    })
  })

  describe('jaccard similarity', () => {
    it('should compute similarity of identical sets', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      const items = ['apple', 'banana', 'cherry', 'date', 'elderberry']

      for (const item of items) {
        mh1.update(Buffer.from(item))
        mh2.update(Buffer.from(item))
      }

      const similarity = mh1.jaccardSimilarity(mh2)
      expect(similarity).toBeCloseTo(1.0, 1)

    })

    it('should compute similarity of disjoint sets', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      const set1 = ['a', 'b', 'c']
      const set2 = ['d', 'e', 'f']

      for (const item of set1) {
        mh1.update(Buffer.from(item))
      }
      for (const item of set2) {
        mh2.update(Buffer.from(item))
      }

      const similarity = mh1.jaccardSimilarity(mh2)
      expect(similarity).toBeCloseTo(0.0, 1)

    })

    it('should compute similarity of partially overlapping sets', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      // Set 1: {a, b, c, d}
      // Set 2: {c, d, e, f}
      // Intersection: {c, d} (2 elements)
      // Union: {a, b, c, d, e, f} (6 elements)
      // Expected Jaccard: 2/6 = 0.333

      for (const item of ['a', 'b', 'c', 'd']) {
        mh1.update(Buffer.from(item))
      }
      for (const item of ['c', 'd', 'e', 'f']) {
        mh2.update(Buffer.from(item))
      }

      const similarity = mh1.jaccardSimilarity(mh2)
      expect(similarity).toBeGreaterThan(0.25)
      expect(similarity).toBeLessThan(0.5)

    })

    it('should compute similarity of subset', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      // Set 1: {a, b}
      // Set 2: {a, b, c, d}
      // Intersection: {a, b} (2 elements)
      // Union: {a, b, c, d} (4 elements)
      // Expected Jaccard: 2/4 = 0.5

      for (const item of ['a', 'b']) {
        mh1.update(Buffer.from(item))
      }
      for (const item of ['a', 'b', 'c', 'd']) {
        mh2.update(Buffer.from(item))
      }

      const similarity = mh1.jaccardSimilarity(mh2)
      expect(similarity).toBeGreaterThan(0.4)
      expect(similarity).toBeLessThan(0.6)

    })

    it('should be symmetric', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      for (const item of ['a', 'b', 'c']) {
        mh1.update(Buffer.from(item))
      }
      for (const item of ['b', 'c', 'd']) {
        mh2.update(Buffer.from(item))
      }

      const sim12 = mh1.jaccardSimilarity(mh2)
      const sim21 = mh2.jaccardSimilarity(mh1)

      expect(sim12).toBeCloseTo(sim21, 1)

    })

    it('should return similarity in valid range', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      mh1.update(Buffer.from('a'))
      mh1.update(Buffer.from('b'))
      mh2.update(Buffer.from('c'))

      const similarity = mh1.jaccardSimilarity(mh2)
      expect(similarity).toBeGreaterThanOrEqual(0.0)
      expect(similarity).toBeLessThanOrEqual(1.0)

    })

    it('should throw on merge with null', () => {
      const mh = new MinHash(128)
      expect(() => mh.jaccardSimilarity(null as any)).toThrow()
    })

    it('should throw on similarity with different numPerm', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(256)

      expect(() => mh1.jaccardSimilarity(mh2)).toThrow()

    })
  })

  describe('merge', () => {
    it('should merge two compatible MinHash sketches', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      for (const item of ['a', 'b']) {
        mh1.update(Buffer.from(item))
      }
      for (const item of ['c', 'd']) {
        mh2.update(Buffer.from(item))
      }

      mh1.merge(mh2)

      // After merge, mh1 should represent {a, b, c, d}
      const reference = new MinHash(128)
      for (const item of ['a', 'b', 'c', 'd']) {
        reference.update(Buffer.from(item))
      }

      const mergedSim = mh1.jaccardSimilarity(reference)
      expect(mergedSim).toBeGreaterThan(0.9)

    })

    it('should throw on merge with different numPerm', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(256)

      expect(() => mh1.merge(mh2)).toThrow()

    })

    it('should throw on merge with null', () => {
      const mh = new MinHash(128)
      expect(() => mh.merge(null as any)).toThrow()
    })

    it('should merge sketches with identical data', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      for (const item of ['a', 'b', 'c']) {
        mh1.update(Buffer.from(item))
        mh2.update(Buffer.from(item))
      }

      mh1.merge(mh2)

      const reference = new MinHash(128)
      for (const item of ['a', 'b', 'c']) {
        reference.update(Buffer.from(item))
      }

      const sim = mh1.jaccardSimilarity(reference)
      expect(sim).toBeCloseTo(1.0, 0)

    })

    it('should merge sketches with overlapping data', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      for (let i = 0; i < 500; i++) {
        mh1.update(Buffer.from(`item-${i}`))
      }

      for (let i = 250; i < 750; i++) {
        mh2.update(Buffer.from(`item-${i}`))
      }

      mh1.merge(mh2)

      const reference = new MinHash(128)
      for (let i = 0; i < 750; i++) {
        reference.update(Buffer.from(`item-${i}`))
      }

      const mergedSim = mh1.jaccardSimilarity(reference)
      expect(mergedSim).toBeGreaterThan(0.85)

    })
  })

  describe('serialization', () => {
    let mh: MinHash

    beforeEach(() => {
      mh = new MinHash(128)
    })

    afterEach(() => {
    })

    it('should serialize empty MinHash', () => {
      const serialized = mh.serialize()
      expect(serialized).toBeDefined()
      expect(serialized.length).toBeGreaterThan(0)
    })

    it('should serialize MinHash with data', () => {
      mh.update(Buffer.from('item1'))
      mh.update(Buffer.from('item2'))
      mh.update(Buffer.from('item3'))

      const serialized = mh.serialize()
      expect(serialized.length).toBeGreaterThan(0)
    })

    it('should deserialize and restore data', () => {
      mh.update(Buffer.from('apple'))
      mh.update(Buffer.from('banana'))

      const serialized = mh.serialize()
      const restored = MinHash.deserialize(serialized)

      const testSet = new MinHash(128)
      testSet.update(Buffer.from('apple'))
      testSet.update(Buffer.from('banana'))

      const simOriginal = mh.jaccardSimilarity(testSet)
      const simRestored = restored.jaccardSimilarity(testSet)

      expect(simRestored).toBeCloseTo(simOriginal, 1)

    })

    it('should handle round-trip serialization', () => {
      for (let i = 0; i < 100; i++) {
        mh.update(Buffer.from(`item-${i}`))
      }

      const serialized = mh.serialize()
      const restored = MinHash.deserialize(serialized)

      const testSet = new MinHash(128)
      for (let i = 0; i < 50; i++) {
        testSet.update(Buffer.from(`item-${i}`))
      }

      const simOriginal = mh.jaccardSimilarity(testSet)
      const simRestored = restored.jaccardSimilarity(testSet)

      expect(simRestored).toBeCloseTo(simOriginal, 1)

    })

    it('should throw on deserialize invalid data', () => {
      expect(() =>
        MinHash.deserialize(Buffer.from([1, 2, 3, 4, 5])),
      ).toThrow()
    })

    it('should throw on deserialize null', () => {
      expect(() => MinHash.deserialize(null as any)).toThrow()
    })
  })

  describe('large dataset', () => {
    it('should handle large sets', () => {
      const mh1 = new MinHash(128)
      const mh2 = new MinHash(128)

      for (let i = 0; i < 10000; i++) {
        mh1.update(Buffer.from(`item-${i}`))
      }

      for (let i = 0; i < 5000; i++) {
        mh2.update(Buffer.from(`item-${i}`))
      }
      for (let i = 10000; i < 15000; i++) {
        mh2.update(Buffer.from(`item-${i}`))
      }

      const similarity = mh1.jaccardSimilarity(mh2)
      // Expected: 5000 / (10000 + 10000 - 5000) = 5000/15000 = 0.333
      expect(similarity).toBeGreaterThan(0.25)
      expect(similarity).toBeLessThan(0.45)

    })

    it('should handle very large sets', () => {
      const mh1 = new MinHash(256)
      const mh2 = new MinHash(256)

      for (let i = 0; i < 100000; i++) {
        mh1.update(Buffer.from(`item-${i}`))
      }

      for (let i = 50000; i < 150000; i++) {
        mh2.update(Buffer.from(`item-${i}`))
      }

      const similarity = mh1.jaccardSimilarity(mh2)
      // Expected: 50000 / 150000 = 0.333
      expect(similarity).toBeGreaterThan(0.25)
      expect(similarity).toBeLessThan(0.45)

    })
  })

  describe('edge cases', () => {
    let mh: MinHash

    beforeEach(() => {
      mh = new MinHash(128)
    })

    afterEach(() => {
    })

    it('should handle empty MinHash similarity', () => {
      const mh2 = new MinHash(128)
      const sim = mh.jaccardSimilarity(mh2)
      expect(typeof sim).toBe('number')
    })

    it('should handle single element sets', () => {
      const mh2 = new MinHash(128)
      mh.update(Buffer.from('item'))
      mh2.update(Buffer.from('item'))

      const similarity = mh.jaccardSimilarity(mh2)
      expect(similarity).toBeCloseTo(1.0, 1)

    })

    it('should distinguish different elements', () => {
      const mh2 = new MinHash(128)
      mh.update(Buffer.from('test'))
      mh.update(Buffer.from('testing'))
      mh.update(Buffer.from('tests'))

      mh2.update(Buffer.from('tester'))
      mh2.update(Buffer.from('tested'))
      mh2.update(Buffer.from('testing'))

      const similarity = mh.jaccardSimilarity(mh2)
      // Only 'testing' is common: 1/5 = 0.2
      expect(similarity).toBeGreaterThan(0.1)
      expect(similarity).toBeLessThan(0.4)

    })

    it('should work with different numPerm values', () => {
      for (const numPerm of [8, 16, 32, 64, 128, 256]) {
        const mha = new MinHash(numPerm)
        const mhb = new MinHash(numPerm)

        mha.update(Buffer.from('item1'))
        mhb.update(Buffer.from('item2'))

        const sim = mha.jaccardSimilarity(mhb)
        expect(sim).toBeDefined()

      }
    })

    it('should handle binary data similarity', () => {
      const mh2 = new MinHash(128)
      const binary1 = Buffer.from([0, 1, 2, 3, 127])
      const binary2 = Buffer.from([0, 1, 2, 3, 127])

      mh.update(binary1)
      mh2.update(binary2)

      const similarity = mh.jaccardSimilarity(mh2)
      expect(similarity).toBeCloseTo(1.0, 1)

    })

    it('should handle unicode similarity', () => {
      const mh2 = new MinHash(128)
      const items = ['你好', 'مرحبا', 'Привет', 'こんにちは']

      for (const item of items) {
        mh.update(Buffer.from(item))
        mh2.update(Buffer.from(item))
      }

      const similarity = mh.jaccardSimilarity(mh2)
      expect(similarity).toBeCloseTo(1.0, 1)

    })
  })

})
