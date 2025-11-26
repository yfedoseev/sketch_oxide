# Version Management

This project uses a centralized version management system to keep version numbers synchronized across all language bindings.

## Single Source of Truth

The version is defined in a single file:

```
VERSION
```

This file contains only the version number (e.g., `0.1.3`).

## Files Managed by the System

The `sync-version.sh` script keeps these files in sync with the VERSION file:

- **Cargo.toml** - Rust workspace
- **nodejs/package.json** - Node.js binding
- **python/pyproject.toml** - Python binding

> Note: Java (pom.xml) and C# (SketchOxide.csproj) are not synced as they are currently disabled in CI/CD.

## How to Release a New Version

1. Update the VERSION file:
   ```bash
   echo "0.1.4" > VERSION
   ```

2. Sync all package files:
   ```bash
   ./scripts/sync-version.sh
   ```

3. Review the changes:
   ```bash
   git diff
   ```

4. Commit the changes:
   ```bash
   git add VERSION nodejs/package.json python/pyproject.toml Cargo.toml
   git commit -m "chore: Bump version to 0.1.4"
   ```

5. Tag the release:
   ```bash
   git tag -a v0.1.4 -m "Release v0.1.4"
   ```

6. Push to GitHub:
   ```bash
   git push origin main
   git push origin v0.1.4
   ```

7. Create a GitHub release from the tag (or it will trigger automatically on release)

## CI/CD Integration

The publish workflow automatically syncs versions from the VERSION file before building and publishing to:
- crates.io (Rust)
- PyPI (Python)
- npm (Node.js)

This ensures consistency across platforms and prevents version mismatch errors.

## Benefits

- ✅ Single location to update for releases
- ✅ Prevents version mismatches between platforms
- ✅ Automated syncing in CI/CD pipeline
- ✅ Clear release workflow

## Troubleshooting

If you get version mismatches during publishing:

1. Check the VERSION file:
   ```bash
   cat VERSION
   ```

2. Re-sync manually:
   ```bash
   ./scripts/sync-version.sh
   ```

3. Verify all files match:
   ```bash
   grep -h "version" VERSION Cargo.toml nodejs/package.json python/pyproject.toml | sort | uniq
   ```
