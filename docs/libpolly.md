# Why we don't install libpolly-dev

Polly is LLVM's loop-nest optimizer. It performs high-level transformations like
loop vectorization, tiling, and interchange — work that matters when compiling
compute-intensive code such as dense matrix kernels or DSP pipelines.

It is normally installed alongside LLVM when a project needs those advanced
loop passes at compile time, either via the LLVM C++ API (`#include <polly/...>`)
or via the `-polly` pass flag in a custom pipeline.

jet does not use any Polly passes. The JIT pipeline operates on EVM bytecode,
which is stack-based and has no loop structure that Polly could analyze. Our
LLVM usage is limited to code generation (`inkwell` / `llvm-sys`), and neither
of those crates requires the Polly library or its headers.

Additionally, `libpolly-21-dev` is absent from both Termux and the Ubuntu 24.04
repos (including the apt.llvm.org channel), so requiring it causes `apt-get
install` to fail entirely — taking `llvm-21-dev` (which *is* needed) down with
it under `set -e`.
