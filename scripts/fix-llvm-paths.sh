#!/bin/bash
# Fix Debian/Ubuntu LLVM header path mismatch
# Debian/Ubuntu packages place headers in /usr/include/llvm-21
# but llvm-config reports /usr/lib/llvm-21/include
# This script creates symlinks to match llvm-config expectations

set -e

LLVM_VERSION=21

if [ ! -d "/usr/lib/llvm-${LLVM_VERSION}/include" ] && [ -d "/usr/include/llvm-${LLVM_VERSION}" ]; then
  echo "Creating symlinks to fix Debian/Ubuntu LLVM header path..."
  sudo mkdir -p /usr/lib/llvm-${LLVM_VERSION}/include
  sudo ln -sf /usr/include/llvm-${LLVM_VERSION}/llvm /usr/lib/llvm-${LLVM_VERSION}/include/llvm
  sudo ln -sf /usr/include/llvm-${LLVM_VERSION}/llvm-c /usr/lib/llvm-${LLVM_VERSION}/include/llvm-c
  echo "Symlinks created successfully"
else
  echo "LLVM include symlinks already exist or not needed"
fi
