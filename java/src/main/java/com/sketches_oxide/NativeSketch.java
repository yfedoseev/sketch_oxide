package com.sketches_oxide;

/**
 * Base class for all native sketch implementations.
 *
 * Provides common functionality for managing native memory and ensuring
 * safe access to the underlying Rust implementations.
 */
public abstract class NativeSketch implements AutoCloseable {
    protected long nativePtr;

    /**
     * Check if the sketch is still alive (not closed).
     *
     * @throws IllegalStateException if the sketch has been closed
     */
    protected final void checkAlive() {
        if (nativePtr == 0) {
            throw new IllegalStateException(
                    getClass().getSimpleName() + " has been closed");
        }
    }

    /**
     * Get the native pointer for debugging purposes.
     *
     * @return the native pointer
     */
    protected final long getNativePtr() {
        return nativePtr;
    }

    /**
     * Close the sketch and free native memory.
     *
     * Safe to call multiple times (subsequent calls do nothing).
     */
    @Override
    public abstract void close();

    /**
     * Free native memory if not already freed.
     *
     * Must be implemented by subclasses to call appropriate JNI free function.
     */
    protected abstract void freeNative();

    /**
     * Called when the object is garbage collected.
     *
     * Ensures that native memory is freed even if close() was not called.
     */
    @Override
    protected void finalize() throws Throwable {
        try {
            if (nativePtr != 0) {
                freeNative();
                nativePtr = 0;
            }
        } finally {
            super.finalize();
        }
    }

    @Override
    public String toString() {
        return getClass().getSimpleName() + "(ptr=" + nativePtr + ")";
    }
}
