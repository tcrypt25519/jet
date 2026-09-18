# Jet: LLVM JIT Compiler for EVM

[![Rust](https://img.shields.io/badge/Rust-stable-CE422B?logo=rust)](https://www.rust-lang.org/)
[![LLVM](https://img.shields.io/badge/LLVM-22-262D3A?logo=llvm)](https://llvm.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![CI](https://github.com/tcrypt25519/jet/actions/workflows/ci.yml/badge.svg)](https://github.com/tcrypt25519/jet/actions/workflows/ci.yml)

Jet is an experimental LLVM-based JIT compiler for the Ethereum Virtual Machine. It translates EVM bytecode into LLVM IR and executes it as native machine code through LLVM's ORC JIT engine, targeting workloads such as MEV simulation where the same contracts run thousands of times with warm data in memory.

## Architecture

Jet is a Rust workspace with five crates:

| Crate | Type | Purpose |
|---|---|---|
| `jet` | lib | EVM to LLVM compiler: bytecode decoding, control-flow planning, opcode emitters, gas accounting, and the JIT `Engine` |
| `jet_runtime` | dylib + lib | Execution `Context`, `CallInfo`, `BlockInfo`, runtime builtins, and the generated runtime IR |
| `jet_ir` | lib | LLVM type registry and EVM constants shared by the compiler and the runtime |
| `jet_push_macros` | proc-macro | Generates the `PUSH0` to `PUSH32` bytecode macros used by tests |
| `jetdbg` | bin | Debug CLI that compiles two sample contracts, runs a CALL between them, and prints the IR |

**Compilation pipeline:**

```
EVM bytecode
  → Bytecode decoding (big-endian immediates to little-endian words)
  → Basic block discovery (JUMPDEST, terminators, dead code)
  → Stack lowering (runtime stack, or symbolic SSA stack with a planned CFG)
  → LLVM IR generation per opcode, with gas charged per basic block
  → Jump dispatch (switch over JUMPDEST blocks)
  → ORC JIT compilation to native code
```

The stack backend is chosen per compilation with `Options::with_stack_mode`. The runtime backend keeps the EVM stack in the execution context and is the default. The symbolic backend keeps stack slots as LLVM values and falls back to the runtime backend when planning exceeds its limits.

For a detailed walkthrough see [`docs/architecture/architecture.md`](docs/architecture/architecture.md) and [`docs/architecture/symbolic-stack.md`](docs/architecture/symbolic-stack.md).

## Status

Jet implements 122 of the 148 EVM opcodes it recognises: arithmetic, comparison and bitwise operations, KECCAK256, stack and memory operations, control flow, call context and block information reads, RETURN, REVERT, INVALID, and a JIT-to-JIT CALL. Gas is charged per basic block, with dynamic costs for memory expansion, KECCAK256, EXP, RETURNDATACOPY and CALL memory.

Not implemented yet: account state (BALANCE, SELFBALANCE, EXTCODESIZE, EXTCODECOPY, EXTCODEHASH), storage (SLOAD, SSTORE, TLOAD, TSTORE), code access (CODESIZE, CODECOPY), CALLDATACOPY, MCOPY, GASPRICE, BLOBHASH, logs, contract creation, CALLCODE, DELEGATECALL, STATICCALL and SELFDESTRUCT. CALL does not yet forward gas or calldata to the callee.

See [`OPERATION_STATUS.md`](OPERATION_STATUS.md) for the full list and [`docs/test_coverage.md`](docs/test_coverage.md) for per-opcode test coverage.

## Getting Started

### Prerequisites

- Rust stable, selected by `rust-toolchain.toml`
- A nightly toolchain with `rustfmt`, used only by `make fmt` and `make fmt-check`
- LLVM 22
- `cargo-nextest`, used by `make test` (install with `make install-tools`)

### Install LLVM and build

```shell
# Install LLVM 22 (handles Linux, macOS, and Termux)
make install-llvm

# Build all crates
make build
```

### Run the debug CLI

`jetdbg` compiles two sample contracts, executes a CALL from one to the other, and prints the generated LLVM IR with syntax highlighting:

```shell
cargo run -p jetdbg
```

### Run the test suite

```shell
make test        # cargo nextest (recommended)
make test-all    # nextest + doctests
make ci          # fmt-check, check, clippy, test-all
```

Every rom test runs under both stack backends, so the runtime backend acts as a differential oracle for the symbolic one.

### Platform notes

**Termux:** Requires `gcc-default` and `ndk-multilib-native-static`. Both are installed by `make install-llvm`. Build artifacts are redirected to `~/.cargo/jet-target` to avoid the no-exec FUSE mount.

## Repository Structure

```
jet/
├── crates/
│   ├── jet/              # Compiler: builder/{contract,ops,stack,symbolic,gas,env,manager}.rs, engine/
│   ├── jet_runtime/      # Runtime: exec.rs, call_info.rs, builtins.rs, runtime_builder.rs
│   ├── jet_ir/           # Shared types: types.rs, constants.rs
│   ├── jet_push_macros/  # Proc macro crate
│   └── jetdbg/           # Debug CLI
├── docs/
│   ├── architecture/     # architecture.md, symbolic-stack.md, bytecode-to-llvm-blocks.md, philosophy.md
│   ├── adrs/             # Architecture Decision Records (001 to 007)
│   ├── process/          # Guides for adding opcodes and runtime functions, segfault triage
│   └── test_coverage.md  # Per-opcode implementation and test coverage
├── .agents/skills/       # EVM opcode reference used by coding agents
├── scripts/              # install-llvm.sh and LLVM detection helpers
├── .github/workflows/    # CI (ubuntu-latest, cached LLVM 22 binaries)
├── Makefile
├── OPERATION_STATUS.md   # Implementation status and known gaps
└── DEVELOPMENT.md        # Full development guide
```

## Development

See [`DEVELOPMENT.md`](DEVELOPMENT.md) for the complete guide, including:

- Adding a new EVM opcode (`docs/process/new-opcode.md`)
- Adding a runtime function (`docs/process/new-runtime-function.md`)
- Architecture decisions (`docs/adrs/`)

## License

MIT. See [LICENSE](LICENSE) for details.
