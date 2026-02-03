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
        if ! apt-cache show "llvm-${LLVM_VERSION}" &> /dev/null; then
            echo "LLVM ${LLVM_VERSION} packages not found for this distro; falling back to official LLVM binaries."
            if ls /etc/apt/sources.list.d/*.list >/dev/null 2>&1; then
                sudo grep -l "llvm-toolchain-jammy-${LLVM_VERSION}" /etc/apt/sources.list.d/*.list 2>/dev/null | xargs -r sudo rm -f
            fi

            LLVM_RELEASE="${LLVM_VERSION}.1.0"
            ARCH=$(uname -m)
            case "$ARCH" in
                x86_64)
                    LLVM_TARBALL="LLVM-${LLVM_RELEASE}-Linux-X64.tar.xz"
                    ;;
                aarch64|arm64)
                    LLVM_TARBALL="LLVM-${LLVM_RELEASE}-Linux-ARM64.tar.xz"
                    ;;
                *)
                    echo "ERROR: Unsupported architecture for LLVM binaries: ${ARCH}"
                    exit 1
                    ;;
            esac

            LLVM_URL="https://github.com/llvm/llvm-project/releases/download/llvmorg-${LLVM_RELEASE}/${LLVM_TARBALL}"
            sudo mkdir -p "/opt/llvm-${LLVM_VERSION}"
            sudo rm -rf "/opt/llvm-${LLVM_VERSION:?}/"*
            curl -L "${LLVM_URL}" | sudo tar -xJ --strip-components=1 -C "/opt/llvm-${LLVM_VERSION}"
            sudo ln -sf "/opt/llvm-${LLVM_VERSION}/bin/llvm-config" "/usr/local/bin/llvm-config-${LLVM_VERSION}"
            LLVM_PREFIX="/opt/llvm-${LLVM_VERSION}"
        else
            sudo apt-get install -y llvm-${LLVM_VERSION} llvm-${LLVM_VERSION}-dev clang-${LLVM_VERSION} libpolly-${LLVM_VERSION}-dev
            LLVM_PREFIX="/usr/lib/llvm-${LLVM_VERSION}"
        fi
        ;;

    *)
        echo "ERROR: Unsupported platform: $PLATFORM"
        exit 1
        ;;
esac

echo "LLVM installed at: $LLVM_PREFIX"
