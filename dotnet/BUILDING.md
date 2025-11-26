# Building SketchOxide .NET Bindings

This guide explains how to build, test, and publish the SketchOxide .NET bindings.

## Prerequisites

- .NET 6.0 SDK or later
- Rust 1.70+ (for building native libraries)
- Git

## Building from Source

### 1. Clone the Repository

```bash
git clone https://github.com/yfedoseev/sketch_oxide.git
cd sketch_oxide/dotnet
```

### 2. Build the Project

```bash
dotnet build
```

For Release build:

```bash
dotnet build -c Release
```

### 3. Run Tests

```bash
dotnet test
```

Run with verbose output:

```bash
dotnet test -v detailed
```

Run specific test class:

```bash
dotnet test --filter "ClassName=HyperLogLogTests"
```

## Building Native Libraries

The native libraries (Rust bindings) need to be built separately. See the Rust library documentation for building for different platforms.

### Platform-Specific Builds

#### Windows (x86_64)

```bash
cargo build --release --target x86_64-pc-windows-msvc
# Output: target/x86_64-pc-windows-msvc/release/sketch_oxide_dotnet.dll
# Place in: dotnet/SketchOxide/runtimes/win-x64/native/
```

#### Linux (x86_64)

**glibc:**
```bash
cargo build --release --target x86_64-unknown-linux-gnu
# Output: target/x86_64-unknown-linux-gnu/release/libsketch_oxide_dotnet.so
# Place in: dotnet/SketchOxide/runtimes/linux-x64/native/
```

**musl:**
```bash
cargo build --release --target x86_64-unknown-linux-musl
# Output: target/x86_64-unknown-linux-musl/release/libsketch_oxide_dotnet.so
# Place in: dotnet/SketchOxide/runtimes/linux-musl-x64/native/
```

#### macOS

**x86_64:**
```bash
cargo build --release --target x86_64-apple-darwin
# Output: target/x86_64-apple-darwin/release/libsketch_oxide_dotnet.dylib
# Place in: dotnet/SketchOxide/runtimes/osx-x64/native/
```

**arm64 (Apple Silicon):**
```bash
cargo build --release --target aarch64-apple-darwin
# Output: target/aarch64-apple-darwin/release/libsketch_oxide_dotnet.dylib
# Place in: dotnet/SketchOxide/runtimes/osx-arm64/native/
```

## Creating NuGet Package

### 1. Build Release Configuration

```bash
dotnet build -c Release
```

### 2. Create Package

```bash
dotnet pack -c Release -o ./nuget
```

This creates `SketchOxide.0.1.0.nupkg` in the `nuget` directory.

### 3. Test Package Locally

```bash
# Add local nuget source
dotnet nuget add source ./nuget -n SketchOxideLocal

# Create test project
mkdir test-package
cd test-package
dotnet new console
dotnet add package SketchOxide

# Test usage
dotnet run
```

## Publishing to NuGet.org

### 1. Get API Key

Go to https://www.nuget.org/account/apikeys and create a new API key.

### 2. Configure Local Credentials

```bash
dotnet nuget update source nuget.org -u [username] -p [api-key]
```

Or create `~/.nuget/NuGet/NuGet.Config`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<configuration>
  <packageSources>
    <add key="nuget.org" value="https://api.nuget.org/v3/index.json" protocolVersion="3" />
  </packageSources>
  <packageSourceCredentials>
    <nuget.org>
      <add key="Username" value="your-username" />
      <add key="ClearTextPassword" value="your-api-key" />
    </nuget.org>
  </packageSourceCredentials>
</configuration>
```

### 3. Push Package

```bash
dotnet nuget push nuget/SketchOxide.0.1.0.nupkg -s nuget.org
```

Or with explicit API key:

```bash
dotnet nuget push nuget/SketchOxide.0.1.0.nupkg \
  -s https://api.nuget.org/v3/index.json \
  -k [api-key]
```

### 4. Verify Package

Package appears at: https://www.nuget.org/packages/SketchOxide/

## Development Workflow

### Adding a New Test

1. Create test file in `SketchOxide.Tests/`
2. Follow the xUnit pattern from existing tests
3. Run tests:

```bash
dotnet test
```

### Updating Documentation

1. Update XML documentation comments in source files
2. Build generates XML docs:

```bash
dotnet build -c Release
```

3. XML file: `bin/Release/net8.0/SketchOxide.xml`

### Code Quality

The project uses nullable reference types and strict compiler settings:

```bash
# Enable strict analysis
dotnet build -c Release
```

Address all compiler warnings and errors.

## Troubleshooting

### Native Library Not Found

**Error:** `DllNotFoundException: Unable to load DLL 'sketch_oxide_dotnet.dll'`

**Solution:** Ensure native libraries are in the correct `runtimes/` directory structure.

```
SketchOxide/runtimes/
├── win-x64/native/sketch_oxide_dotnet.dll
├── linux-x64/native/libsketch_oxide_dotnet.so
├── linux-musl-x64/native/libsketch_oxide_dotnet.so
├── osx-x64/native/libsketch_oxide_dotnet.dylib
└── osx-arm64/native/libsketch_oxide_dotnet.dylib
```

### Test Failures

Run with detailed logging:

```bash
dotnet test -v detailed --logger "console;verbosity=detailed"
```

### NuGet Push Fails

Verify credentials:

```bash
dotnet nuget update source nuget.org --username [username] --password [api-key]
```

List configured sources:

```bash
dotnet nuget list source
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build and Test

on: [push, pull_request]

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        dotnet: [6.0, 7.0, 8.0]

    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-dotnet@v3
        with:
          dotnet-version: ${{ matrix.dotnet }}

      - name: Build
        run: dotnet build -c Release

      - name: Test
        run: dotnet test -c Release --no-build

      - name: Pack
        run: dotnet pack -c Release -o ./nuget
```

## Performance Benchmarks

Run benchmarks using BenchmarkDotNet:

```bash
dotnet run -c Release -p SketchOxide.Tests/SketchOxide.Tests.csproj --filter "*Benchmark*"
```

## Release Checklist

- [ ] Update version in `SketchOxide.csproj`
- [ ] Update `README.md` with new features
- [ ] Run full test suite: `dotnet test`
- [ ] Verify native libraries for all platforms
- [ ] Build Release: `dotnet build -c Release`
- [ ] Create package: `dotnet pack -c Release`
- [ ] Test package locally
- [ ] Push to NuGet: `dotnet nuget push`
- [ ] Tag release in git
- [ ] Update GitHub releases page
- [ ] Announce on discussions/issues

## Support

For issues or questions:

- GitHub Issues: https://github.com/yfedoseev/sketch_oxide/issues
- Discussions: https://github.com/yfedoseev/sketch_oxide/discussions
- Email: yfedoseev@gmail.com
