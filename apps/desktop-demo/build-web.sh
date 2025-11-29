#!/bin/bash
set -e

echo "Building RS-Compose Demo for Web..."

# Check if wasm-pack is installed (check common locations)
WASM_PACK=""
if command -v wasm-pack &> /dev/null; then
    WASM_PACK="wasm-pack"
elif [ -f "$HOME/.cargo/bin/wasm-pack" ]; then
    WASM_PACK="$HOME/.cargo/bin/wasm-pack"
elif [ -f "~/.cargo/bin/wasm-pack" ]; then
    WASM_PACK="~/.cargo/bin/wasm-pack"
else
    echo "Error: wasm-pack is not installed or not in PATH"
    echo "Install it with: cargo install wasm-pack"
    echo "Or add ~/.cargo/bin to your PATH"
    exit 1
fi

echo "Using wasm-pack at: $WASM_PACK"

# Build the WASM module with web feature
echo "Building WASM module..."
"$WASM_PACK" build --target web --out-dir pkg --features web,renderer-wgpu --no-default-features

echo ""
echo "Build complete! 🎉"
echo ""
echo "To run the demo:"
echo "1. Start a local web server in this directory:"
echo "   python3 -m http.server 8080"
echo "   or"
echo "   npx serve ."
echo ""
echo "2. Open http://localhost:8080 in your browser"
echo ""
echo "Note: WebGPU support is required. Use Chrome 113+, Edge 113+, or Safari 18+"
