#!/bin/bash
set -e

LLVM_VERSION=21
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM=$(bash "$SCRIPT_DIR/detect-platform.sh")

case "$PLATFORM" in
  darwin)
    command -v brew &> /dev/null || {
      echo "ERROR: Homebrew required"
      exit 1
    }
    brew install llvm
    LLVM_PREFIX=$(brew --prefix llvm)
    ;;

  termux)
    pkg install -y llvm gcc-default ndk-multilib-native-static
    LLVM_PREFIX="/data/data/com.termux/files/usr"
    ;;

  debian)
    # 1. Ensure the repo is added if missing
    if ! apt-cache policy | grep -q "apt.llvm.org"; then
      sudo bash "$SCRIPT_DIR/llvm.sh" ${LLVM_VERSION}
    fi

    # 2. Update and install
    sudo apt-get update
    sudo apt-get install -y \
      llvm-${LLVM_VERSION} \
      llvm-${LLVM_VERSION}-dev \
      libclang-common-${LLVM_VERSION}-dev \
      libpolly-${LLVM_VERSION}-dev 
    LLVM_CONFIG="llvm-config-${LLVM_VERSION}"
    if command -v $LLVM_CONFIG > /dev/null; then
      LLVM_PREFIX=$($LLVM_CONFIG --prefix)
      LLVM_INCLUDE=$($LLVM_CONFIG --includedir)

      # Validation for the Ubuntu/Debian split
      if [ ! -d "$LLVM_INCLUDE/llvm" ] && [ -d "/usr/include/llvm-${LLVM_VERSION}" ]; then
        echo "Detected Debian-style header split. Redirecting include path..."
        LLVM_INCLUDE="/usr/include/llvm-${LLVM_VERSION}"
      fi
    fi
    ;;
  *)
    echo "ERROR: Unsupported platform: $PLATFORM"
    exit 1
    ;;
esac

echo "LLVM installed at: $LLVM_PREFIX"
