using System;

namespace SketchOxide.Native;

/// <summary>
/// Base class for all sketch data structures backed by native Rust implementations.
/// Manages lifecycle of native pointers and provides safe resource cleanup via IDisposable.
/// </summary>
public abstract class NativeSketch : IDisposable
{
    /// <summary>
    /// Opaque pointer to the native Rust instance.
    /// </summary>
    protected nuint NativePtr { get; set; }

    /// <summary>
    /// Indicates whether this sketch has been disposed.
    /// </summary>
    protected bool IsDisposed { get; private set; }

    /// <summary>
    /// Finalizer ensures cleanup even if Dispose is not called.
    /// </summary>
    ~NativeSketch()
    {
        Dispose(disposing: false);
    }

    /// <summary>
    /// Checks if the sketch has been disposed and throws if it has.
    /// </summary>
    /// <exception cref="ObjectDisposedException">Thrown if the sketch has been disposed.</exception>
    protected void CheckAlive()
    {
        if (IsDisposed)
        {
            throw new ObjectDisposedException(GetType().Name);
        }
    }

    /// <summary>
    /// Frees the native Rust instance. Implemented by subclasses.
    /// </summary>
    protected abstract void FreeNative();

    /// <summary>
    /// Releases unmanaged resources used by this sketch.
    /// </summary>
    public void Dispose()
    {
        Dispose(disposing: true);
        GC.SuppressFinalize(this);
    }

    /// <summary>
    /// Releases resources used by this sketch.
    /// </summary>
    protected virtual void Dispose(bool disposing)
    {
        if (!IsDisposed)
        {
            if (disposing)
            {
                // Managed resources cleanup if any
            }

            FreeNative();
            IsDisposed = true;
        }
    }
}
