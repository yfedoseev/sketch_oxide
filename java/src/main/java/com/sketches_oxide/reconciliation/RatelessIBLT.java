package com.sketches_oxide.reconciliation;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.util.ArrayList;
import java.util.List;

/**
 * Rateless IBLT: Efficient Set Reconciliation
 *
 * <p>Rateless IBLT is a probabilistic data structure for efficiently computing the
 * symmetric difference between two sets in distributed systems without knowing the
 * difference size a priori.
 *
 * <p><strong>Use Cases:</strong>
 * <ul>
 *   <li>Blockchain synchronization (5.6x faster than naive approaches)</li>
 *   <li>P2P network synchronization</li>
 *   <li>Distributed cache invalidation</li>
 *   <li>Database replication</li>
 * </ul>
 *
 * <p><strong>Example:</strong>
 * <pre>
 * try (RatelessIBLT alice = new RatelessIBLT(100, 32);
 *      RatelessIBLT bob = new RatelessIBLT(100, 32)) {
 *     alice.insert("key1".getBytes(), "value1".getBytes());
 *     bob.insert("key2".getBytes(), "value2".getBytes());
 *
 *     RatelessIBLT diff = alice.clone();
 *     diff.subtract(bob);
 *     SetDifference result = diff.decode();
 * }
 * </pre>
 */
public final class RatelessIBLT extends NativeSketch {

    public RatelessIBLT(int expectedDiff, int cellSize) {
        if (expectedDiff <= 0) {
            throw new IllegalArgumentException("expectedDiff must be > 0");
        }
        if (cellSize < 8) {
            throw new IllegalArgumentException("cellSize must be >= 8");
        }

        this.nativePtr = SketchOxideNative.ratelessIBLT_new(expectedDiff, cellSize);
        if (this.nativePtr == 0) {
            throw new OutOfMemoryError("Failed to allocate RatelessIBLT");
        }
    }

    public void insert(byte[] key, byte[] value) {
        checkAlive();
        if (key == null || value == null) {
            throw new NullPointerException("key and value cannot be null");
        }

        if (!SketchOxideNative.ratelessIBLT_insert(nativePtr, key, value)) {
            throw new RuntimeException("Insert failed");
        }
    }

    public void delete(byte[] key, byte[] value) {
        checkAlive();
        if (key == null || value == null) {
            throw new NullPointerException("key and value cannot be null");
        }

        if (!SketchOxideNative.ratelessIBLT_delete(nativePtr, key, value)) {
            throw new RuntimeException("Delete failed");
        }
    }

    public void subtract(RatelessIBLT other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (!SketchOxideNative.ratelessIBLT_subtract(this.nativePtr, other.nativePtr)) {
            throw new IllegalArgumentException("Cannot subtract IBLTs with different parameters");
        }
    }

    public SetDifference decode() {
        checkAlive();
        byte[] data = SketchOxideNative.ratelessIBLT_decode(nativePtr);

        if (data == null) {
            throw new RuntimeException("Decode failed");
        }

        return SetDifference.deserialize(data);
    }

    @Override
    public void close() {
        if (nativePtr != 0) {
            freeNative();
            nativePtr = 0;
        }
    }

    @Override
    protected void freeNative() {
        if (nativePtr != 0) {
            SketchOxideNative.ratelessIBLT_free(nativePtr);
        }
    }

    public static final class SetDifference {
        public final List<KeyValue> toInsert;
        public final List<KeyValue> toRemove;

        SetDifference(List<KeyValue> toInsert, List<KeyValue> toRemove) {
            this.toInsert = toInsert;
            this.toRemove = toRemove;
        }

        static SetDifference deserialize(byte[] data) {
            ByteBuffer buffer = ByteBuffer.wrap(data).order(ByteOrder.LITTLE_ENDIAN);

            int insertCount = buffer.getInt();
            List<KeyValue> toInsert = new ArrayList<>(insertCount);
            for (int i = 0; i < insertCount; i++) {
                int keyLen = buffer.getInt();
                byte[] key = new byte[keyLen];
                buffer.get(key);

                int valueLen = buffer.getInt();
                byte[] value = new byte[valueLen];
                buffer.get(value);

                toInsert.add(new KeyValue(key, value));
            }

            int removeCount = buffer.getInt();
            List<KeyValue> toRemove = new ArrayList<>(removeCount);
            for (int i = 0; i < removeCount; i++) {
                int keyLen = buffer.getInt();
                byte[] key = new byte[keyLen];
                buffer.get(key);

                int valueLen = buffer.getInt();
                byte[] value = new byte[valueLen];
                buffer.get(value);

                toRemove.add(new KeyValue(key, value));
            }

            return new SetDifference(toInsert, toRemove);
        }
    }

    public static final class KeyValue {
        public final byte[] key;
        public final byte[] value;

        KeyValue(byte[] key, byte[] value) {
            this.key = key;
            this.value = value;
        }
    }
}
