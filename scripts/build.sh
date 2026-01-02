#!/bin/bash

# Build script for Bitcoin Scratch-Off Miner

set -e

echo "Building Bitcoin Scratch-Off Miner..."

# Check for wasm-pack
if ! command -v wasm-pack &> /dev/null; then
    echo "wasm-pack not found. Installing..."
    cargo install wasm-pack
fi

# Navigate to project root
cd "$(dirname "$0")/.."

# Build the WASM package
echo "Building WASM package..."
wasm-pack build crates/miner-wasm --target web --out-dir ../../web/pkg

# Clean up unnecessary files
rm -f web/pkg/.gitignore
rm -f web/pkg/package.json
rm -f web/pkg/README.md

echo "Build complete!"
echo ""
echo "To run locally, start a web server in the web directory:"
echo "  cd web && python3 -m http.server 8080"
echo ""
echo "Then open http://localhost:8080 in your browser."
