#!/bin/bash
set -e

echo "Building Holochain DNA for Holoclaw ACP..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: Rust is not installed. Please install Rust from https://rustup.rs/"
    exit 1
fi

# Check if WASM target is installed
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Build the zomes
echo "Building integrity zome..."
cargo build --release --target wasm32-unknown-unknown --package acp_integrity

echo "Building coordinator zome..."
cargo build --release --target wasm32-unknown-unknown --package acp_coordinator

# Check if wasm files exist
if [ -f "target/wasm32-unknown-unknown/release/acp_integrity.wasm" ] && \
   [ -f "target/wasm32-unknown-unknown/release/acp_coordinator.wasm" ]; then
    echo "✓ WASM files built successfully:"
    ls -lh target/wasm32-unknown-unknown/release/*.wasm
else
    echo "Error: WASM files not found"
    exit 1
fi

# Check if hc is installed for packaging
if command -v hc &> /dev/null; then
    echo "Packaging DNA..."
    hc dna pack dnas/holoclaw_acp/
    echo "✓ DNA packaged successfully"
else
    echo "Note: 'hc' command not found. Skipping DNA packaging."
    echo "Install Holochain CLI with: cargo install holochain_cli --version 0.6.1-rc.0"
fi

echo ""
echo "Build complete! Next steps:"
echo "1. Run sandbox: hc sandbox run -p 8888 workdir"
echo "2. Test zomes: npx tsx test/test_zome_calls.ts"
