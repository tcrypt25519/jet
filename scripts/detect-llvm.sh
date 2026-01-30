#!/bin/bash
set -e

LLVM_VERSION=21
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM=$(bash "$SCRIPT_DIR/detect-platform.sh")

case "$PLATFORM" in
    darwin)
        LLVM_PREFIX=$(brew --prefix llvm 2>/dev/null || echo "/usr/local/opt/llvm")
        ;;
    termux)
        LLVM_PREFIX="/data/data/com.termux/files/usr"
        ;;
    debian)
        LLVM_PREFIX=$(llvm-config-${LLVM_VERSION} --prefix 2>/dev/null || echo "/usr/lib/llvm-${LLVM_VERSION}")
        ;;
    *)
        echo "ERROR: Unsupported platform: $PLATFORM" >&2
        exit 1
        ;;
esac

echo "$LLVM_PREFIX"
