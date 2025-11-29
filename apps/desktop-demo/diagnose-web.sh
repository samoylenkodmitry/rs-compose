#!/bin/bash

echo "=== Diagnostic Script for Web Build ==="
echo ""

# 1. Check if pkg directory exists
if [ -d "pkg" ]; then
    echo "✓ pkg directory exists"
    echo "Contents:"
    ls -lh pkg/
    echo ""
else
    echo "✗ pkg directory does not exist - build hasn't run successfully"
    exit 1
fi

# 2. Check if the JS file exists and contains run_app
if [ -f "pkg/desktop_app.js" ]; then
    echo "✓ desktop_app.js exists"
    if grep -q "run_app" pkg/desktop_app.js; then
        echo "✓ run_app found in desktop_app.js"
        echo "Exports in desktop_app.js:"
        grep "export" pkg/desktop_app.js | head -20
    else
        echo "✗ run_app NOT found in desktop_app.js"
        echo "This means the build didn't include the web feature properly"
    fi
    echo ""
else
    echo "✗ desktop_app.js not found"
fi

# 3. Check WASM file
if [ -f "pkg/desktop_app_bg.wasm" ]; then
    echo "✓ WASM file exists"
    SIZE=$(wc -c < pkg/desktop_app_bg.wasm)
    echo "  Size: $SIZE bytes"
else
    echo "✗ WASM file not found"
fi

echo ""
echo "=== Rebuild Command ==="
echo "Try rebuilding with verbose output:"
echo ""
echo "  ~/.cargo/bin/wasm-pack build --target web --out-dir pkg --features web,renderer-wgpu --no-default-features -v"
echo ""
