package com.sketches_oxide.streaming;

import com.sketches_oxide.NativeSketch;
import com.sketches_oxide.native.SketchOxideNative;

public final class SlidingHyperLogLog extends NativeSketch {

    public SlidingHyperLogLog(int precision, long maxWindowSeconds) {
        if (precision < 4 || precision > 16) {
            throw new IllegalArgumentException("precision must be between 4 and 16");
        }

        this.nativePtr = SketchOxideNative.slidingHyperLogLog_new(precision, maxWindowSeconds);
        if (this.nativePtr == 0) {
            throw new OutOfMemoryError("Failed to allocate SlidingHyperLogLog");
        }
    }

    public void update(byte[] item, long timestamp) {
        checkAlive();
        if (item == null) {
            throw new NullPointerException("item cannot be null");
        }

        if (!SketchOxideNative.slidingHyperLogLog_update(nativePtr, item, timestamp)) {
            throw new RuntimeException("Update failed");
        }
    }

    public double estimateWindow(long currentTime, long windowSeconds) {
        checkAlive();
        return SketchOxideNative.slidingHyperLogLog_estimateWindow(nativePtr, currentTime, windowSeconds);
    }

    public double estimateTotal() {
        checkAlive();
        return SketchOxideNative.slidingHyperLogLog_estimateTotal(nativePtr);
    }

    public void decay(long currentTime, long windowSeconds) {
        checkAlive();
        if (!SketchOxideNative.slidingHyperLogLog_decay(nativePtr, currentTime, windowSeconds)) {
            throw new RuntimeException("Decay failed");
        }
    }

    public void merge(SlidingHyperLogLog other) {
        checkAlive();
        if (other == null) {
            throw new NullPointerException("other cannot be null");
        }
        other.checkAlive();

        if (!SketchOxideNative.slidingHyperLogLog_merge(this.nativePtr, other.nativePtr)) {
            throw new IllegalArgumentException("Cannot merge SlidingHyperLogLogs with different parameters");
        }
    }

    public int precision() {
        checkAlive();
        return SketchOxideNative.slidingHyperLogLog_precision(nativePtr);
    }

    public byte[] serialize() {
        checkAlive();
        return SketchOxideNative.slidingHyperLogLog_serialize(nativePtr);
    }

    public static SlidingHyperLogLog deserialize(byte[] data) {
        if (data == null) {
            throw new NullPointerException("data cannot be null");
        }

        long ptr = SketchOxideNative.slidingHyperLogLog_deserialize(data);
        if (ptr == 0) {
            throw new IllegalArgumentException("Invalid serialized data");
        }

        SlidingHyperLogLog hll = new SlidingHyperLogLog(12, 3600); // Dummy
        hll.nativePtr = ptr;
        return hll;
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
            SketchOxideNative.slidingHyperLogLog_free(nativePtr);
        }
    }
}
