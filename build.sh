#!/usr/bin/env bash
# ==============================================================================
# WADM Multi-Architecture Build Script
#
# Builds the embedded React frontend and compiles release binaries for:
# - x86_64-unknown-linux-gnu
# - aarch64-unknown-linux-gnu
#
# Usage:
#   ./build.sh [x86_64 | aarch64 | all]
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

export PATH="$HOME/.cargo/bin:$PATH"

TARGET="${1:-all}"
OUTPUT_DIR="dist"

log_info()  { echo -e "\033[1;34m[INFO]\033[0m  $1"; }
log_ok()    { echo -e "\033[1;32m[OK]\033[0m    $1"; }
log_error() { echo -e "\033[1;31m[ERROR]\033[0m $1" >&2; }
die()       { log_error "$1"; exit 1; }

# Step 1: Compile Frontend Assets
build_frontend() {
    log_info "Building embedded frontend SPA..."
    if [[ ! -d "web" ]]; then
        die "Frontend directory 'web' not found."
    fi

    (
        cd web
        if [[ ! -d "node_modules" ]]; then
            log_info "Installing frontend dependencies..."
            npm ci --prefer-offline
        fi
        npm run build
    )
    log_ok "Frontend built successfully (web/dist ready)."
}

# Step 2: Compile Target Binary
build_target() {
    local TRIPLE="$1"
    local OUT_NAME="$2"

    log_info "Compiling release binary for ${TRIPLE}..."

    mkdir -p "${OUTPUT_DIR}"

    # Use native cargo if target matches host architecture, otherwise fallback to cross
    local HOST_TRIPLE
    HOST_TRIPLE="$(rustc -vV 2>/dev/null | grep '^host:' | cut -d' ' -f2 || echo "")"

    if [[ "$TRIPLE" == "$HOST_TRIPLE" ]]; then
        cargo build --release --target "$TRIPLE"
    elif command -v cross >/dev/null 2>&1; then
        cross build --release --target "$TRIPLE"
    else
        log_info "Cross-compiler 'cross' not found in PATH; attempting cargo build..."
        cargo build --release --target "$TRIPLE"
    fi

    local BIN_SRC="target/${TRIPLE}/release/wadm"
    if [[ ! -f "$BIN_SRC" ]]; then
        die "Expected binary not found at ${BIN_SRC}"
    fi

    cp "$BIN_SRC" "${OUTPUT_DIR}/${OUT_NAME}"
    chmod +x "${OUTPUT_DIR}/${OUT_NAME}"

    log_ok "Built ${OUTPUT_DIR}/${OUT_NAME} ($(du -h "${OUTPUT_DIR}/${OUT_NAME}" | cut -f1))"
}

# Main Execution Flow
build_frontend

case "$TARGET" in
    x86_64|amd64)
        build_target "x86_64-unknown-linux-gnu" "wadm-linux-x86_64"
        ;;
    aarch64|arm64)
        build_target "aarch64-unknown-linux-gnu" "wadm-linux-aarch64"
        ;;
    all)
        build_target "x86_64-unknown-linux-gnu" "wadm-linux-x86_64"
        if command -v cross >/dev/null 2>&1 || rustup target list 2>/dev/null | grep -q "aarch64-unknown-linux-gnu (installed)"; then
            build_target "aarch64-unknown-linux-gnu" "wadm-linux-aarch64"
        else
            log_info "Skipping aarch64 build (neither 'cross' nor aarch64 toolchain available)."
        fi
        ;;
    *)
        die "Unknown target: $TARGET. Supported targets: x86_64, aarch64, all"
        ;;
esac

log_ok "Build complete. Artifacts available in '${OUTPUT_DIR}/'."
