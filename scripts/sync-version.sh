#!/bin/bash
# Sync version from VERSION file to all package files
# Usage: ./scripts/sync-version.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VERSION_FILE="$PROJECT_ROOT/VERSION"

# Read version from VERSION file
if [ ! -f "$VERSION_FILE" ]; then
    echo "Error: VERSION file not found at $VERSION_FILE"
    exit 1
fi

VERSION=$(cat "$VERSION_FILE" | tr -d '[:space:]')

if [ -z "$VERSION" ]; then
    echo "Error: VERSION file is empty"
    exit 1
fi

echo "Syncing version to $VERSION..."

# Update Cargo.toml (root workspace)
echo "  ✓ Cargo.toml"
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/g" "$PROJECT_ROOT/Cargo.toml"
rm -f "$PROJECT_ROOT/Cargo.toml.bak"

# Update Node.js package.json
echo "  ✓ nodejs/package.json"
sed -i.bak "s/\"version\": \".*\"/\"version\": \"$VERSION\"/g" "$PROJECT_ROOT/nodejs/package.json"
rm -f "$PROJECT_ROOT/nodejs/package.json.bak"

# Update Python pyproject.toml
echo "  ✓ python/pyproject.toml"
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/g" "$PROJECT_ROOT/python/pyproject.toml"
rm -f "$PROJECT_ROOT/python/pyproject.toml.bak"

echo "✅ Version synced to $VERSION"
