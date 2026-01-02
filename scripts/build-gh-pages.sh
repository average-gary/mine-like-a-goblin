#!/bin/bash
# Build script for GitHub Pages deployment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

echo "Building Goblin Mining Co. for GitHub Pages..."

# Build the WASM module
echo "Building WASM module..."
wasm-pack build --target web crates/miner-wasm

# Copy WASM output to web/pkg
echo "Copying WASM to web/pkg..."
rm -rf web/pkg
mkdir -p web/pkg
cp crates/miner-wasm/pkg/miner_wasm.js web/pkg/
cp crates/miner-wasm/pkg/miner_wasm_bg.wasm web/pkg/
cp crates/miner-wasm/pkg/miner_wasm.d.ts web/pkg/ 2>/dev/null || true

echo ""
echo "Build complete! The 'web' folder is ready for GitHub Pages deployment."
echo ""
echo "To deploy:"
echo "  1. Push to GitHub"
echo "  2. Go to Settings > Pages"
echo "  3. Set source to 'GitHub Actions' (auto deploys on push)"
