# Jet: LLVM JIT Compiler for EVM

[![Rust](https://img.shields.io/badge/Rust-stable-CE422B?logo=rust)](https://www.rust-lang.org/)
[![LLVM](https://img.shields.io/badge/LLVM-21-262D3A?logo=llvm)](https://llvm.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![CI](https://github.com/tcrypt25519/jet/actions/workflows/ci.yml/badge.svg)](https://github.com/tcrypt25519/jet/actions/workflows/ci.yml)

Jet is an experimental LLVM-based JIT compiler for the Ethereum Virtual Machine. It translates EVM bytecode into LLVM IR and executes it as native machine code via LLVM's ORC JIT engine — targeting high-performance scenarios like MEV simulations where the same contracts execute thousands of times with warm data in memory.

## Architecture

Jet is organized as a Rust workspace with four crates:

| Crate | Type | Purpose |
|---|---|---|
| `jet` | lib + bin | Core EVM→LLVM compiler and `jetdbg` CLI |
| `jet_runtime` | dylib + lib | Execution context, builtins, and runtime IR |
| `jet_ir` | lib | Shared LLVM types and EVM constants |
| `jet_push_macros` | proc-macro | Generates `PUSH0`–`PUSH32` bytecode macros |

**Compilation pipeline:**

```
EVM Bytecode
  → Bytecode parsing (big-endian → little-endian words)
  → Basic block discovery (JUMPDEST, terminators)
  → LLVM IR generation (per opcode)
  → Jump table construction (switch dispatch)
  → ORC JIT compilation → native machine code
```

For a detailed walkthrough see [`docs/architecture/architecture.md`](docs/architecture/architecture.md).

## Getting Started

### Prerequisites

- Rust (stable) — managed via `rust-toolchain.toml`
- LLVM 21
- `cargo-nextest` (optional, used by `make test`)

### Install LLVM and build

```shell
# Install LLVM 21 (handles Linux, macOS, and Termux)
make install-llvm

# Build all crates
make build
```

### Run the debug CLI

`jetdbg` compiles and executes an EVM contract, printing the generated LLVM IR with syntax highlighting:

```shell
cargo run --bin jetdbg
```

### Run the test suite

```shell
make test        # cargo nextest (recommended)
make test-all    # nextest + doctests
make ci          # fmt-check, check, clippy, test-all
```

### Platform notes

**Termux:** Requires `gcc-default` and `ndk-multilib-native-static`. Both are installed by `make install-llvm`. Build artifacts are redirected to `~/.cargo/jet-target` to avoid the no-exec FUSE mount.

## Repository Structure

```
jet/
├── crates/
│   ├── jet/              # Compiler: builder/, engine/, bin/jetdbg.rs
│   ├── jet_runtime/      # Runtime: exec.rs, builtins.rs, runtime_builder.rs
│   ├── jet_ir/           # Shared types: types.rs, constants.rs
│   └── jet_push_macros/  # Proc macro crate
├── docs/
│   ├── architecture/     # architecture.md, philosophy.md, bytecode-to-llvm-blocks.md
│   ├── adrs/             # Architecture Decision Records (001–004)
│   └── process/          # Guides for adding opcodes and runtime functions
├── scripts/              # install-llvm.sh and LLVM detection helpers
├── .github/workflows/    # CI (ubuntu-latest, LLVM binary cache)
├── Makefile
└── DEVELOPMENT.md        # Full development guide
```

## Development

See [`DEVELOPMENT.md`](DEVELOPMENT.md) for the complete guide, including:

- Adding a new EVM opcode (`docs/process/new-opcode.md`)
- Adding a runtime function (`docs/process/new-runtime-function.md`)
- Architecture decisions (`docs/adrs/`)

## License

MIT — see [LICENSE](LICENSE) for details.
