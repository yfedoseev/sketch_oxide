package com.sketches_oxide.range_filters;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;

public final class Grafite extends NativeSketch {

    public static Grafite build(long[] keys, int bitsPerKey) {
        if (keys == null || keys.length == 0) {
            throw new IllegalArgumentException("keys cannot be null or empty");
        }
        if (bitsPerKey < 2 || bitsPerKey > 16) {
            throw new IllegalArgumentException("bitsPerKey must be between 2 and 16");
        }

        ByteBuffer buffer = ByteBuffer.allocate(4 + keys.length * 8).order(ByteOrder.LITTLE_ENDIAN);
        buffer.putInt(keys.length);
        for (long key : keys) {
            buffer.putLong(key);
        }

        Grafite filter = new Grafite();
        filter.nativePtr = SketchOxideNative.grafite_build(buffer.array(), bitsPerKey);
        if (filter.nativePtr == 0) {
            throw new OutOfMemoryError("Failed to build Grafite");
        }
        return filter;
    }

    private Grafite() {}

    public boolean mayContainRange(long low, long high) {
        checkAlive();
        return SketchOxideNative.grafite_mayContainRange(nativePtr, low, high);
    }

    public boolean mayContain(long key) {
        checkAlive();
        return SketchOxideNative.grafite_mayContain(nativePtr, key);
    }

    public double expectedFpr(long rangeWidth) {
        checkAlive();
        return SketchOxideNative.grafite_expectedFpr(nativePtr, rangeWidth);
    }

    public int keyCount() {
        checkAlive();
        return SketchOxideNative.grafite_keyCount(nativePtr);
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
            SketchOxideNative.grafite_free(nativePtr);
        }
    }
}
