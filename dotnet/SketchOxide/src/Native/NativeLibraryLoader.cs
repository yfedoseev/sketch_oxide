using System;
using System.IO;
using System.Reflection;
using System.Runtime.InteropServices;

namespace SketchOxide.Native;

/// <summary>
/// Handles loading of platform-specific native libraries for SketchOxide.
/// Automatically detects the current platform and loads the appropriate native library.
/// </summary>
internal static class NativeLibraryLoader
{
    private static bool s_initialized;

    /// <summary>
    /// Initializes and loads the native library.
    /// Called once during static initialization.
    /// </summary>
    internal static void Initialize()
    {
        if (s_initialized)
            return;

        s_initialized = true;

        string libName = GetLibraryName();
        string libPath = GetLibraryPath(libName);

        try
        {
            NativeLibrary.Load(libPath);
        }
        catch (DllNotFoundException ex)
        {
            throw new InvalidOperationException(
                $"Failed to load native SketchOxide library. " +
                $"Platform: {RuntimeInformation.OSDescription}, " +
                $"Architecture: {RuntimeInformation.ProcessArchitecture}, " +
                $"Expected library: {libName}",
                ex);
        }
    }

    /// <summary>
    /// Gets the native library name for the current platform.
    /// </summary>
    private static string GetLibraryName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return "sketch_oxide_dotnet.dll";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            return "libsketch_oxide_dotnet.so";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return "libsketch_oxide_dotnet.dylib";

        throw new PlatformNotSupportedException(
            $"Unsupported platform: {RuntimeInformation.OSDescription}");
    }

    /// <summary>
    /// Gets the full path to the native library.
    /// Tries system library paths first, then falls back to extracting from assembly resources.
    /// </summary>
    private static string GetLibraryPath(string libName)
    {
        // Try to load from system library paths
        if (TryLoadFromSystemPaths(libName, out var systemPath))
            return systemPath;

        // Fall back to extracting from assembly resources
        return ExtractFromResources(libName);
    }

    /// <summary>
    /// Attempts to load the native library from system library paths.
    /// </summary>
    private static bool TryLoadFromSystemPaths(string libName, out string? foundPath)
    {
        foundPath = null;

        try
        {
            if (NativeLibrary.TryLoad(libName, out var handle))
            {
                NativeLibrary.Free(handle);
                foundPath = libName;
                return true;
            }
        }
        catch
        {
            // Ignore exceptions, will try other methods
        }

        return false;
    }

    /// <summary>
    /// Extracts the native library from assembly resources and saves it to a temporary directory.
    /// </summary>
    private static string ExtractFromResources(string libName)
    {
        var assembly = Assembly.GetExecutingAssembly();
        string? assemblyPath = Path.GetDirectoryName(assembly.Location);

        if (string.IsNullOrEmpty(assemblyPath))
            throw new InvalidOperationException("Cannot determine assembly location");

        // Get runtime identifier (runtimes/win-x64/native/, runtimes/linux-x64/native/, etc.)
        string runtimeId = GetRuntimeIdentifier();
        string resourcePath = $"runtimes/{runtimeId}/native/{libName}";

        // Try to find native library in runtimes directory relative to assembly
        string runtimesPath = Path.Combine(assemblyPath, "runtimes", runtimeId, "native", libName);
        if (File.Exists(runtimesPath))
            return runtimesPath;

        // Try to extract from embedded resources
        var resourceStream = assembly.GetManifestResourceStream(resourcePath);
        if (resourceStream != null)
        {
            string tempDir = Path.Combine(Path.GetTempPath(), "sketch_oxide_dotnet");
            Directory.CreateDirectory(tempDir);

            string tempPath = Path.Combine(tempDir, libName);
            using (var file = File.Create(tempPath))
            {
                resourceStream.CopyTo(file);
            }

            return tempPath;
        }

        throw new InvalidOperationException(
            $"Native library not found: {libName}. " +
            $"Expected in runtimes/{runtimeId}/native/ or as embedded resource.");
    }

    /// <summary>
    /// Gets the runtime identifier (e.g., "win-x64", "linux-x64", "osx-x64", "osx-arm64").
    /// </summary>
    private static string GetRuntimeIdentifier()
    {
        string os = GetOSIdentifier();
        string arch = GetArchitectureIdentifier();
        return $"{os}-{arch}";
    }

    /// <summary>
    /// Gets the OS identifier.
    /// </summary>
    private static string GetOSIdentifier()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return "win";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            return "linux";
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return "osx";

        throw new PlatformNotSupportedException(
            $"Unsupported platform: {RuntimeInformation.OSDescription}");
    }

    /// <summary>
    /// Gets the architecture identifier.
    /// </summary>
    private static string GetArchitectureIdentifier()
    {
        return RuntimeInformation.ProcessArchitecture switch
        {
            Architecture.X64 => "x64",
            Architecture.Arm64 => "arm64",
            _ => throw new PlatformNotSupportedException(
                $"Unsupported architecture: {RuntimeInformation.ProcessArchitecture}")
        };
    }
}
