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
repos (including the apt.llvm.org channel). The `llvm-sys` build script discovers
Polly via `llvm-config --libs` (because the official apt.llvm.org packages ship
LLVM compiled with Polly enabled) and emits `-l static=Polly -l static=PollyISL`. Without the static
archives those link lines fail.

The fix is to use the `llvm21-1-prefer-dynamic` inkwell feature so that `llvm-sys`
links against the monolithic `libLLVM-21.so` instead of individual static
archives. Polly is already compiled into that shared library, so no separate
package is needed.
