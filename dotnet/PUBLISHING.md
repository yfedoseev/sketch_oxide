# Publishing SketchOxide to NuGet

Complete guide for publishing the SketchOxide .NET bindings to NuGet.org.

## Pre-Publication Checklist

Before publishing, ensure:

- [ ] All tests pass: `dotnet test`
- [ ] No compiler warnings
- [ ] Documentation is complete and accurate
- [ ] Version number is updated in `.csproj`
- [ ] Release notes are prepared
- [ ] All native libraries are built for supported platforms
- [ ] Code has been reviewed
- [ ] CHANGELOG.md is updated

## Version Management

Semantic versioning is used: `MAJOR.MINOR.PATCH`

Update version in `/dotnet/SketchOxide/SketchOxide.csproj`:

```xml
<PropertyGroup>
  <Version>0.2.0</Version>  <!-- Update this -->
</PropertyGroup>
```

## Building for Publication

### Step 1: Prepare Native Libraries

Ensure all native libraries are built and placed in the correct locations:

```
SketchOxide/runtimes/
├── win-x64/native/sketch_oxide_dotnet.dll
├── linux-x64/native/libsketch_oxide_dotnet.so
├── linux-musl-x64/native/libsketch_oxide_dotnet.so
├── osx-x64/native/libsketch_oxide_dotnet.dylib
└── osx-arm64/native/libsketch_oxide_dotnet.dylib
```

### Step 2: Clean and Build

```bash
cd dotnet
dotnet clean
dotnet build -c Release
```

### Step 3: Run Full Test Suite

```bash
dotnet test -c Release
```

All tests must pass.

### Step 4: Verify Documentation

```bash
dotnet build -c Release
# Check generated XML file
cat "bin/Release/net8.0/SketchOxide.xml"
```

## Creating the Package

### Generate NuGet Package

```bash
# Create output directory
mkdir -p ./nuget

# Pack the project
dotnet pack -c Release -o ./nuget

# Verify package contents
unzip -l ./nuget/SketchOxide.*.nupkg | head -50
```

### Package Contents Verification

The package should contain:

- `lib/` - Compiled assemblies for each target framework
- `runtimes/` - Native libraries for each platform
- `README.md` - Package documentation
- `.nuspec` - Package metadata

Example validation:

```bash
# List package contents
unzip -l ./nuget/SketchOxide.0.1.0.nupkg
```

Expected output includes:

```
  .../README.md
  .../SketchOxide.xml
  .../lib/net6.0/SketchOxide.dll
  .../lib/net7.0/SketchOxide.dll
  .../lib/net8.0/SketchOxide.dll
  .../lib/netstandard2.1/SketchOxide.dll
  .../runtimes/win-x64/native/sketch_oxide_dotnet.dll
  .../runtimes/linux-x64/native/libsketch_oxide_dotnet.so
  .../runtimes/osx-x64/native/libsketch_oxide_dotnet.dylib
  .../runtimes/osx-arm64/native/libsketch_oxide_dotnet.dylib
```

## Setting Up NuGet Credentials

### Option 1: Using Command Line

```bash
dotnet nuget update source nuget.org \
  -u [your-username] \
  -p [your-api-key] \
  --store-password-in-clear-text
```

**Note:** Use a dedicated "publish" API key with limited scope for security.

### Option 2: Using NuGet.Config

Create or edit `~/.nuget/NuGet/NuGet.Config`:

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

### Option 3: Using Environment Variables

```bash
export NUGET_API_KEY="your-api-key"

dotnet nuget push ./nuget/SketchOxide.*.nupkg \
  -s https://api.nuget.org/v3/index.json \
  -k $NUGET_API_KEY
```

## Publishing to NuGet.org

### Push the Package

```bash
dotnet nuget push ./nuget/SketchOxide.0.1.0.nupkg \
  -s https://api.nuget.org/v3/index.json \
  -k [your-api-key]
```

### Expected Output

```
Pushing SketchOxide.0.1.0.nupkg to 'https://api.nuget.org/v3/index.json'...
  PUT https://www.nuget.org/api/v2/package/
  Accepted 202   https://www.nuget.org/api/v2/package/ 1234ms

Your package was pushed.
```

### Verify Publication

Wait 5-10 minutes, then verify:

```bash
# Check if package exists
dotnet package search SketchOxide

# Or visit: https://www.nuget.org/packages/SketchOxide/0.1.0
```

## Testing the Published Package

### Create Test Project

```bash
mkdir test-published
cd test-published
dotnet new console
dotnet add package SketchOxide
```

### Test Code

Create `Program.cs`:

```csharp
using SketchOxide.Cardinality;

using var hll = new HyperLogLog(14);
for (int i = 0; i < 1000; i++)
{
    hll.Update($"item-{i}");
}

Console.WriteLine($"Estimate: {hll.Estimate():F0}");
```

### Run Test

```bash
dotnet run
```

Expected output:

```
Estimate: 1000
```

## Post-Publication Tasks

### 1. Create GitHub Release

```bash
git tag v0.1.0
git push origin v0.1.0

# Or use GitHub CLI
gh release create v0.1.0 \
  --title "SketchOxide 0.1.0" \
  --body "Release notes here" \
  ./nuget/SketchOxide.0.1.0.nupkg
```

### 2. Update Documentation

- Update version in main README
- Add release notes to CHANGELOG.md
- Update installation instructions

### 3. Announce Release

- Post on discussions
- Update project website
- Notify users/stakeholders

## Troubleshooting

### Package Push Fails with 401

**Error:** `Response status code does not indicate success: 401 (Unauthorized).`

**Solution:** Verify API key:

```bash
# Test credentials
dotnet nuget update source nuget.org \
  -u [username] \
  -p [api-key]

# Check if key has push permission
# Visit: https://www.nuget.org/account/apikeys
```

### Package Already Exists

**Error:** `Response status code does not indicate success: 409 (Conflict).`

**Solution:** Version already published. Use a new version number.

```bash
# Update SketchOxide.csproj
# Increment version: 0.1.0 → 0.1.1
```

### Native Libraries Not Included

**Error:** Runtime not found after installation.

**Solution:** Verify package structure:

```bash
unzip -l ./nuget/SketchOxide.*.nupkg | grep runtimes
```

Should show:

```
  .../runtimes/win-x64/native/sketch_oxide_dotnet.dll
  .../runtimes/linux-x64/native/libsketch_oxide_dotnet.so
  etc.
```

If missing, ensure `.csproj` has:

```xml
<ItemGroup>
  <Content Include="runtimes/win-x64/native/sketch_oxide_dotnet.dll"
           Pack="true"
           PackagePath="runtimes/win-x64/native/" />
  <!-- etc. -->
</ItemGroup>
```

### Package Dependencies Issue

**Error:** Package dependencies are missing or incorrect.

**Solution:** Verify no unintended dependencies:

```bash
cat ./nuget/SketchOxide.*.nuspec | grep -A 10 "<dependencies"
```

Should only have development-time dependencies (test frameworks marked as PrivateAssets).

## Yanking Packages (Removal)

If a published package has critical issues:

```bash
# Via NuGet.org web UI
# https://www.nuget.org/packages/SketchOxide/

# Or via CLI (requires Nuget CLI)
nuget delete SketchOxide 0.1.0 -Source https://www.nuget.org/api/v2/package
```

Note: Packages are typically unlisted rather than deleted to preserve version integrity.

## Continuous Integration

### GitHub Actions Workflow

Create `.github/workflows/publish.yml`:

```yaml
name: Publish to NuGet

on:
  release:
    types: [published]

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup .NET
        uses: actions/setup-dotnet@v3
        with:
          dotnet-version: 8.0.x

      - name: Build
        run: |
          cd dotnet
          dotnet build -c Release

      - name: Test
        run: |
          cd dotnet
          dotnet test -c Release

      - name: Pack
        run: |
          cd dotnet
          dotnet pack -c Release

      - name: Push
        run: |
          cd dotnet
          dotnet nuget push bin/Release/*.nupkg \
            -s https://api.nuget.org/v3/index.json \
            -k ${{ secrets.NUGET_API_KEY }}
```

### Secrets Configuration

Add to GitHub repo settings:

- `NUGET_API_KEY` - API key from https://www.nuget.org/account/apikeys

## Security Best Practices

- Use a **scoped API key** (Push/Publish only, no delete)
- **Never commit** API keys to git
- Use **GitHub Secrets** for CI/CD
- **Rotate API keys** periodically
- **Sign packages** (optional, advanced)

## Support

For issues during publication:

- NuGet Support: https://www.nuget.org/policies/Contact
- GitHub Issues: https://github.com/yfedoseev/sketch_oxide/issues
- Email: yfedoseev@gmail.com
