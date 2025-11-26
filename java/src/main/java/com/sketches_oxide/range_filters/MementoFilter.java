package com.sketches_oxide.range_filters;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

public final class MementoFilter extends NativeSketch {

    public MementoFilter(int expectedElements, double fpr) {
        if (expectedElements <= 0) {
            throw new IllegalArgumentException("expectedElements must be > 0");
        }
        if (fpr <= 0.0 || fpr >= 1.0) {
            throw new IllegalArgumentException("fpr must be in (0, 1)");
        }

        this.nativePtr = SketchOxideNative.mementoFilter_new(expectedElements, fpr);
        if (this.nativePtr == 0) {
            throw new OutOfMemoryError("Failed to allocate MementoFilter");
        }
    }

    public void insert(long key, byte[] value) {
        checkAlive();
        if (value == null) {
            throw new NullPointerException("value cannot be null");
        }

        if (!SketchOxideNative.mementoFilter_insert(nativePtr, key, value)) {
            throw new RuntimeException("Insert failed - capacity may be exceeded");
        }
    }

    public boolean mayContainRange(long low, long high) {
        checkAlive();
        return SketchOxideNative.mementoFilter_mayContainRange(nativePtr, low, high);
    }

    public int len() {
        checkAlive();
        return SketchOxideNative.mementoFilter_len(nativePtr);
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
            SketchOxideNative.mementoFilter_free(nativePtr);
        }
    }
}
