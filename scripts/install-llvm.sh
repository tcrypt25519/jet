#!/bin/bash
set -e

LLVM_VERSION=21
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM=$(bash "$SCRIPT_DIR/detect-platform.sh")

case "$PLATFORM" in
    darwin)
        command -v brew &> /dev/null || { echo "ERROR: Homebrew required"; exit 1; }
        brew install llvm
        LLVM_PREFIX=$(brew --prefix llvm)
        ;;

    termux)
        pkg install -y llvm gcc-default ndk-multilib-native-static
        LLVM_PREFIX="/data/data/com.termux/files/usr"
        ;;

    debian)
        if ! apt-cache policy | grep -q "apt.llvm.org"; then
            sudo bash "$SCRIPT_DIR/llvm.sh" ${LLVM_VERSION}
        fi
        sudo apt-get update
        sudo apt-get install -y \
            llvm-${LLVM_VERSION} \
            llvm-${LLVM_VERSION}-dev \
            clang-${LLVM_VERSION} \
            libpolly-${LLVM_VERSION}-dev
        LLVM_PREFIX="/usr/lib/llvm-${LLVM_VERSION}"
        ;;

    *)
        echo "ERROR: Unsupported platform: $PLATFORM"
        exit 1
        ;;
esac

echo "LLVM installed at: $LLVM_PREFIX"
