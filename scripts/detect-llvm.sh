#!/bin/bash
set -e

LLVM_VERSION=22
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLATFORM=$(bash "$SCRIPT_DIR/detect-platform.sh")

case "$PLATFORM" in
  darwin)
    LLVM_PREFIX=$(brew --prefix llvm 2> /dev/null || echo "/usr/local/opt/llvm")
    ;;
  termux)
    LLVM_PREFIX="/data/data/com.termux/files/usr"
    ;;
  debian)
    LLVM_PREFIX=$(
      llvm-config-${LLVM_VERSION} --prefix 2> /dev/null || {
        for d in /usr/lib/llvm-${LLVM_VERSION} /usr/include/llvm-${LLVM_VERSION}; do
          [ -d "$d" ] && {
            echo "$d"
            exit 0
          }
        done
        exit 1
      }
    ) || {
      echo "Could not find LLVM ${LLVM_VERSION}" >&2
      exit 1
    }
    ;;
  *)
    echo "ERROR: Unsupported platform: $PLATFORM" >&2
    exit 1
    ;;
esac

echo "$LLVM_PREFIX"
