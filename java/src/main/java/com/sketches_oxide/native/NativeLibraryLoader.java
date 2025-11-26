package com.sketches_oxide.native;

import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardCopyOption;

/**
 * Loads the native sketch_oxide library for the current platform.
 *
 * Supports:
 * - Windows (x86_64)
 * - Linux (x86_64, musl)
 * - macOS (x86_64, aarch64)
 */
public final class NativeLibraryLoader {
    private static volatile boolean loaded = false;

    private NativeLibraryLoader() {
        // Utility class, no instantiation
    }

    /**
     * Load the native library for the current platform.
     *
     * @throws UnsatisfiedLinkError if the native library cannot be loaded
     */
    public static synchronized void load() {
        if (loaded) {
            return;
        }

        try {
            String platform = detectPlatform();
            String arch = detectArchitecture();
            String libName = String.format("sketch_oxide_java-%s-%s", platform, arch);

            // Try to load from system library path first
            try {
                System.loadLibrary(libName);
                loaded = true;
                return;
            } catch (UnsatisfiedLinkError e) {
                // Fall through to extract from JAR
            }

            // Extract from JAR and load
            String resourceName = String.format("/native/%s/%s.%s",
                    platform,
                    libName,
                    getFileExtension(platform));

            loadFromJar(resourceName, libName, platform);
            loaded = true;
        } catch (Exception e) {
            throw new UnsatisfiedLinkError(
                    "Failed to load sketch_oxide native library: " + e.getMessage());
        }
    }

    /**
     * Detect the current operating system platform.
     *
     * @return "windows", "linux", or "macos"
     */
    private static String detectPlatform() {
        String osName = System.getProperty("os.name").toLowerCase();

        if (osName.contains("win")) {
            return "windows";
        } else if (osName.contains("linux")) {
            return "linux";
        } else if (osName.contains("mac") || osName.contains("darwin")) {
            return "macos";
        } else {
            throw new UnsatisfiedLinkError("Unsupported OS: " + osName);
        }
    }

    /**
     * Detect the current processor architecture.
     *
     * @return "x86_64" or "aarch64"
     */
    private static String detectArchitecture() {
        String arch = System.getProperty("os.arch").toLowerCase();
        String osName = System.getProperty("os.name").toLowerCase();

        // Handle x86_64 variants
        if (arch.contains("amd64") || arch.contains("x86_64") || arch.contains("x64")) {
            return "x86_64";
        }

        // Handle ARM variants
        if (arch.contains("aarch64") || arch.contains("arm64")) {
            return "aarch64";
        }

        // Fallback for unknown ARM
        if (arch.contains("arm")) {
            throw new UnsatisfiedLinkError("32-bit ARM is not supported. Please use 64-bit.");
        }

        throw new UnsatisfiedLinkError("Unsupported architecture: " + arch);
    }

    /**
     * Get the file extension for the native library based on platform.
     *
     * @param platform "windows", "linux", or "macos"
     * @return "dll", "so", or "dylib"
     */
    private static String getFileExtension(String platform) {
        switch (platform) {
            case "windows":
                return "dll";
            case "linux":
                return "so";
            case "macos":
                return "dylib";
            default:
                throw new IllegalArgumentException("Unknown platform: " + platform);
        }
    }

    /**
     * Load native library from JAR resources.
     */
    private static void loadFromJar(String resourceName, String libName, String platform)
            throws IOException {
        // Try to load from resources embedded in JAR
        try (InputStream input = NativeLibraryLoader.class.getResourceAsStream(resourceName)) {
            if (input == null) {
                throw new IOException("Native library not found in JAR: " + resourceName);
            }

            // Create temporary file
            Path tempDir = Files.createTempDirectory("sketch-oxide-");
            tempDir.toFile().deleteOnExit();

            String extension = getFileExtension(platform);
            Path tempLib = tempDir.resolve(libName + "." + extension);
            tempLib.toFile().deleteOnExit();

            // Copy to temp location
            Files.copy(input, tempLib, StandardCopyOption.REPLACE_EXISTING);

            // Load from temp location
            System.load(tempLib.toAbsolutePath().toString());
        }
    }

    /**
     * Check if the native library is already loaded.
     *
     * @return true if loaded, false otherwise
     */
    public static boolean isLoaded() {
        return loaded;
    }
}
