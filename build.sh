#!/bin/bash

echo "Building Frontend..."
cd web
npm install
npm run build
cd ..

export PATH="$HOME/.cargo/bin:$PATH"

mkdir -p build/linux-x64 build/linux-arm64 build/linux-riscv64 build/windows-x64

echo "Building Backend for x86_64-unknown-linux-gnu..."
cross build --release --target x86_64-unknown-linux-gnu && \
cp target/x86_64-unknown-linux-gnu/release/wadm build/linux-x64/ || echo "Failed to build Linux x64"

echo "Building Backend for aarch64-unknown-linux-gnu (ARM64)..."
cross build --release --target aarch64-unknown-linux-gnu && \
cp target/aarch64-unknown-linux-gnu/release/wadm build/linux-arm64/ || echo "Failed to build Linux ARM64"

echo "Building Backend for riscv64gc-unknown-linux-gnu (RISC-V)..."
cross build --release --target riscv64gc-unknown-linux-gnu && \
cp target/riscv64gc-unknown-linux-gnu/release/wadm build/linux-riscv64/ || echo "Failed to build Linux RISC-V"

echo "Building Backend for x86_64-pc-windows-gnu (Windows .exe)..."
cross build --release --target x86_64-pc-windows-gnu && \
cp target/x86_64-pc-windows-gnu/release/wadm.exe build/windows-x64/ || echo "Failed to build Windows x64. WADM might rely on Unix-specific features (nix, bollard Unix sockets, std::os::unix, etc.)."

echo "Build process completed! Check the 'build' directory."
