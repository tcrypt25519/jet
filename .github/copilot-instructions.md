# Jet – Copilot Onboarding Instructions

## What this repo does
- **Jet** is an LLVM-based JIT compiler for the Ethereum Virtual Machine (EVM)
- Rust workspace that JIT-compiles EVM bytecode to LLVM IR, then to native code via LLVM ORC JIT (`inkwell` bindings)
- Ideal for MEV use cases: same contract executed thousands of times with warm data
- Core value: native code execution vs. interpretation, with LLVM optimization passes

## Repository structure

### Workspace layout
- **Root files:**
  - `Cargo.toml` (workspace manifest with 5 members)
  - `Makefile` (build automation, exports LLVM env vars)
  - `.cargo/config.toml` (Android/Termux linker flags)
  - `rust-toolchain.toml` (pins stable Rust channel)
  - `README.md`, `DEVELOPMENT.md` (getting started, dev workflow)
  - `.nextest.toml` (cargo-nextest test runner config)

- **Workspace members (5 crates):**
  1. `crates/jet` - Main compiler crate
     - Edition 2024
     - Parses EVM opcodes, plans control flow, builds LLVM IR, charges gas, drives JIT
     - Key modules: `builder/{contract,env,manager,ops,stack,symbolic,gas}.rs`, `engine/mod.rs`, `instructions.rs`
     - Tests: `tests/test_roms.rs` (EVM opcode integration tests, run under both stack backends), `tests/roms/mod.rs` (harness)

  2. `crates/jet_runtime` - Runtime execution context
     - Edition 2024
     - Crate type: `["dylib", "lib"]`
     - Builtins invoked from generated LLVM IR
     - Key modules: `{lib,exec,call_info,builtins,symbols,runtime_builder,address,layout_tests}.rs`, `binding/mod.rs`

  3. `crates/jet_ir` - Shared LLVM IR types
     - Edition 2024
     - LLVM type registry and constants shared by compiler and runtime

  4. `crates/jet_push_macros` - Procedural macros
     - Edition 2024
     - Proc-macro crate generating the `PUSH0`..`PUSH32` test macros

  5. `crates/jetdbg` - Debug CLI
     - Edition 2024
     - Binary that compiles two sample contracts, runs a CALL between them, and prints the IR

- **Scripts:** `scripts/{detect-llvm,detect-platform,install-llvm,llvm}.sh`
- **Documentation:** `docs/` (see [Key Documentation](#key-documentation) section)
- **CI:** `.github/workflows/ci.yml` (GitHub Actions, see [CI/CD](#cicd) section)

### Key source files
- `crates/jet/src/`:
  - `lib.rs` - Public API exports
  - `instructions.rs` - EVM instruction enum and opcode mappings
  - `builder/contract.rs` - Bytecode chunking, CFG construction, symbolic stack planning, opcode dispatch, jump dispatch
  - `builder/ops.rs` - LLVM IR generation for each EVM opcode, memory expansion, gas charges
  - `builder/stack.rs` - `StackBackend` trait with the runtime and symbolic implementations
  - `builder/symbolic.rs` - `SymbolicStack` of LLVM values with `known_u64` metadata
  - `builder/gas.rs` - Static gas costs and dynamic gas kinds per opcode
  - `builder/env.rs` - Compilation environment (`Options`, `Mode`, `StackMode`, `Env`, `Symbols`)
  - `builder/manager.rs` - Build orchestration
  - `engine/mod.rs` - LLVM ORC JIT engine, symbol resolution

- `crates/jetdbg/src/main.rs` - CLI debugger for EVM contracts

- `crates/jet_runtime/src/`:
  - `builtins.rs` - Complex operations implemented in Rust (EXP, KECCAK256, etc.)
  - `exec.rs` - Execution context (stack, memory, return data, gas), `BlockInfo`, `ReturnCode`
  - `call_info.rs` - Per-frame `CallInfo` (calldata, address, origin, caller, value, gas limit)
  - `symbols.rs` - Runtime symbol name constants
  - `runtime_builder.rs` - Runtime LLVM module: IR-defined stack and memory helpers, builtin declarations
  - `layout_tests.rs` - Rust/LLVM struct layout checks for `Context`, `CallInfo`, `BlockInfo`

## Toolchain & dependencies

### Rust
- **Version:** Stable (pinned in `rust-toolchain.toml`)
- **Editions:** 2024 (all crates)
- **Components:** rustfmt, clippy (for CI)
- **Nightly:** only for formatting. `make fmt` and `make fmt-check` run `cargo +nightly fmt`; building, testing and clippy use stable

### LLVM
- **Version:** 22 (REQUIRED)
- **Detection:** `scripts/detect-llvm.sh` finds `/usr/lib/llvm-22`
- **Environment:** Makefile exports `LLVM_SYS_221_PREFIX` automatically
- **Bindings:**
  - `inkwell` - crates.io release with the `llvm22-1-prefer-dynamic` feature
  - `llvm-sys` - Expects `llvm-config-22` or `LLVM_SYS_221_PREFIX`

### Dependencies
- **LLVM tooling:** `inkwell` (crates.io, LLVM 22 feature)
- **Crypto:** `sha3`, `bnum` (256-bit arithmetic), `hex`
- **CLI:** `clap` (derive), `colored`, `syntect` (syntax highlighting)
- **Serialization:** `serde` (derive only, used for `Options`)
- **Logging:** `log`, `simple_logger`
- **Error handling:** `thiserror`

### Platform support
- **Supported:** Linux (Debian/Ubuntu), macOS (darwin), Android (Termux)
- **Detection:** `scripts/detect-platform.sh` branches by platform
- **Termux-specific:** 
  - `.cargo/config.toml` contains linker flags
  - `CARGO_TARGET_DIR` relocated to `/data/data/com.termux/files/home/.cargo/jet-target`

## Build, test, lint workflow

### CRITICAL: Bootstrap LLVM first
**ALWAYS run this before any cargo command:**
```bash
make install-llvm
```

This:
- Detects platform via `scripts/detect-platform.sh`
- Installs LLVM 22 via apt (Debian/Ubuntu), Homebrew (macOS) or pkg (Termux)
- May require `sudo` for system package installation
- Takes 1-3 minutes depending on network/disk speed

**Failure symptoms if LLVM missing:**
- `cargo check` fails with: `No suitable version of LLVM was found system-wide or pointed to by LLVM_SYS_221_PREFIX`
- `llvm-sys` build script cannot find `llvm-config-22`

**Verification:**
```bash
bash scripts/detect-llvm.sh  # Should output: /usr/lib/llvm-22
which llvm-config-22         # Should find executable
```

### Build commands
```bash
# Standard workflow
make check      # cargo check --all-targets --all-features (fastest validation)
make build      # cargo build
make fmt        # cargo +nightly fmt --all (auto-format)
make clippy     # cargo clippy --all-targets --all-features -- -D warnings (strict)

# Combined pre-push check
make commit-check  # Runs: fmt-check, check, clippy, test-all
make ci            # Alias for commit-check (mirrors CI pipeline)
```

**Note:** `make commit-check` is the **recommended pre-push command** - it runs all CI checks locally.

### Testing

**Test infrastructure:**
- Test runner: `cargo-nextest` (faster, better output than cargo test)
- Install: `make install-tools` or `cargo install cargo-nextest --locked`
- Config: `.nextest.toml`

**Test commands:**
```bash
make test         # cargo nextest run --all-features (fastest)
make test-cargo   # cargo test --all-features (fallback if nextest unavailable)
make doctest      # cargo test --doc --all-features (nextest doesn't support doctests)
make test-all     # Runs both: nextest + doctests
```

**Test structure:**
- `crates/jet/tests/test_roms.rs` - Main EVM opcode integration tests
  - Uses `rom_tests!` macro (defined in `tests/roms/mod.rs`) to define test cases
  - Each case expands to two tests, one per `StackMode`, so both stack backends must agree
  - Each test: bytecode ROM → expected stack/memory/return/gas state
  - Plain `#[test]` functions cover call context, calldata, nested CALL and gas via `run_both_modes`
  - No external EVM node required - pure LLVM JIT execution
- `crates/jet/tests/invalid_opcode.rs` - Error handling tests
- `crates/jet/src/builder/contract.rs` - Unit tests for symbolic plan limits and static gas regions
- `crates/jet_runtime/src/{builtins,layout_tests,address,runtime_builder}.rs` - Runtime unit tests
- `docs/test_coverage.md` - Per-opcode implementation and test coverage table

### Linting & formatting
```bash
# Check formatting (CI enforced)
make fmt-check    # cargo +nightly fmt --all -- --check

# Auto-format (CI will auto-commit this)
make fmt          # cargo +nightly fmt --all

# Lint (strict mode)
make clippy       # -D warnings (all warnings are errors in CI)

# Auto-fix clippy issues
make clippy-fix   # cargo clippy --fix
```

**Clippy notes:**
- CI enforces `-D warnings` (zero-warning policy)
- Common issues: unused variables, manual_div_ceil, needless borrows
- **CRITICAL:** Agents must NOT add `#[allow(...)]` pragmas without explicit permission. Fix the underlying issue or report why it should be allowed.

## CI/CD

### GitHub Actions workflow
- **Location:** `.github/workflows/ci.yml`
- **Triggers:** push to `master`/`main`, PRs to `master`/`main`/`ts/*`, manual dispatch
- **Runner:** `ubuntu-latest` (single sequential job)
- **Permissions:** `contents: write` (for auto-commit formatting fixes)

### CI pipeline steps (17 total)
1. Checkout code
2. **Cache LLVM 22** (keyed on Cargo.lock + install scripts)
3. **Restore LLVM from cache** (conditional: cache hit)
4. **Install LLVM 22** (conditional: cache miss, ~5 minutes)
5. **Log LLVM shared libraries** (verification step)
6. **Set LLVM env vars** (`LLVM_SYS_221_PREFIX`, include paths)
7. **Cache Rust artifacts** (Swatinem/rust-cache)
8. **Install Rust nightly rustfmt** (rustfmt component only)
9. **Install Rust stable** (with rustfmt, clippy)
10. **Apply formatting fixes** (`cargo +nightly fmt --all`)
11. **Commit formatting fixes** (conditional: push or same-repo PR)
12. **Run clippy** (with `-D warnings`)
13. **Install cargo-nextest**
14. **Check all targets** (`cargo check`)
15. **Build** (`cargo build --verbose --all-features`)
16. **Run tests with nextest** (`--no-fail-fast`)
17. **Run doctests** (`cargo test --doc`)

### CI features
- **Smart caching:** LLVM binary (cached), Rust artifacts (swatinem)
- **Auto-formatting:** CI auto-commits `cargo +nightly fmt` changes and pushes back
- **Strict mode:** `RUSTFLAGS="-D warnings"` environment variable
- **Fast feedback:** Clippy before expensive build/test
- **No-fail-fast:** Tests continue after first failure (better error visibility)

### Environment variables
```bash
CARGO_TERM_COLOR=always    # Colored output
RUST_BACKTRACE=1           # Full backtraces on panic
CARGO_INCREMENTAL=0        # Disable incremental (CI optimization)
RUSTFLAGS="-D warnings"    # Treat warnings as errors
```

## Key documentation

**Architecture & design:**
- `docs/architecture/architecture.md` - Complete system architecture (compilation pipeline, memory model, CFG)
- `docs/architecture/symbolic-stack.md` - Symbolic stack lowering, planning, fault exits, runtime fallback
- `docs/architecture/bytecode-to-llvm-blocks.md` - Bytecode chunking, jump tables, CodeBlock lifecycle
- `docs/architecture/philosophy.md` - Design philosophy and naming conventions
- `OPERATION_STATUS.md` - Which opcodes are implemented, partial, or missing
- `docs/test_coverage.md` - Per-opcode test coverage

**Development guides:**
- `DEVELOPMENT.md` - Toolchain setup, workflow, CI details
- `docs/process/new-opcode.md` - **CRITICAL: Step-by-step opcode implementation guide**
  - Includes common pitfalls (endianness, stack order, zero division)
  - Required reading before implementing EVM opcodes
- `docs/process/segfault-troubleshooting.md` - Debugging and reporting segfaults (P0 priority)

**ADRs (Architecture Decision Records):**
- `docs/adrs/adr-001.md` through `adr-007.md` - Word representation, pointer-based memory, libpolly, u32 memory offsets, runtime memory expansion, `MemoryRegion` enforcement, and destackifying the EVM stack

**EVM spec references:**
- `.agents/skills/evm-opcodes/references/docs/<HEX>.md` - Per-opcode reference (stack inputs, outputs, gas, edge cases)
- Use the `evm-opcodes` skill if available; otherwise read the file directly

## Development patterns & best practices

### Stack representation
- **Internal format:** Little-endian (32-byte words)
- **PUSH immediates:** Byte-reversed on load (EVM is big-endian)
- **Builtins:** Receive little-endian data; use the `read_u256` / `write_u256` helpers in `builtins.rs`

### Stack operations
```rust
// Always go through the StackBackend (never touch Context.stack directly)
let (a, b) = bctx.stack.pop_2(bctx)?;   // Returns (top, second) as i256 IntValues
let x = bctx.stack.pop_word(bctx)?;     // Single word
bctx.stack.push_word(bctx, result)?;    // Push an IntValue (widened to i256 if narrower)
```

Emitters are generic over the backend: `fn op<'ctx, S: StackBackend<'ctx>>(bctx: &BuildCtx<'ctx, '_, S>)`.
The same code runs for the runtime stack and the symbolic stack.

**CRITICAL:** `pop_2()` returns `(top, second)` where `top` is most recently pushed.
For SUB: `PUSH 3; PUSH 10; SUB` → pops `(10, 3)` → computes `10 - 3 = 7` (NOT `3 - 10`).

Every opcode also needs a stack-effect entry in `apply_abstract_instruction` in `contract.rs` so the symbolic planner knows how many words it pops and pushes.

### Division by zero (EVM semantics)
**EVM requirement:** Division/modulo by zero MUST return 0 (not undefined behavior).

```rust
// WRONG - LLVM poison value on zero divisor
let result = bctx.builder.build_int_unsigned_div(a, b, "div")?;

// ALSO WRONG - select still evaluates both arms, causing poison
let zero = bctx.env.types().i256.const_zero();
let b_is_zero = bctx.builder.build_int_compare(IntPredicate::EQ, b, zero, "b_is_zero")?;
let div_result = bctx.builder.build_int_unsigned_div(a, b, "div")?;
let result = bctx.builder.build_select(b_is_zero, zero, div_result, "final")?;
```

**CORRECT approach:** Use `build_zero_guarded_value`, which branches around the operation and merges with a phi.

```rust
let (a, b) = bctx.stack.pop_2(bctx)?;
let result = build_zero_guarded_value(bctx, b, "div", |bctx| {
    bctx.builder.build_int_unsigned_div(a, b, "div_result")
})?;
bctx.stack.push_word(bctx, result)
```

Applies to: DIV, MOD, SDIV, SMOD, and any division-like operations.

### LLVM poison avoidance
- **Branching pattern:** Use explicit `conditional_branch` for zero checks (not `select`)
- **Reason:** LLVM eagerly evaluates both arms of `select`, causing poison on div-by-zero
- **Helper function:** `build_zero_guarded_value()` in `builder/ops.rs` implements the branch and phi; DIV/SDIV/MOD/SMOD use it

### Memory operations
**Always expand memory before access.** Memory helpers take a `MemoryRegion`, which only `expand_memory_region` can construct (ADR 006):
```rust
let (loc, val) = bctx.stack.pop_2(bctx)?;
let loc_i32 = truncate_to_i32(bctx, loc, "mstore_loc")?;
let size = bctx.env.types().i32.const_int(32, false);
let region = expand_memory_region(bctx, loc_i32, size, "mstore")?;
build_mem_store_value(bctx, &region, val)
```

- `truncate_to_i32` maps values above `u32::MAX` to a sentinel that expansion rejects (ADR 004)
- Expansion rounds to 32-byte boundaries, updates `memory_len` monotonically, and reallocates if needed
- Memory expansion gas is charged before the expansion runs, so an unaffordable expansion leaves memory unchanged

### Builtin function pattern
**Rust side** (`crates/jet_runtime/src/builtins.rs`):
```rust
pub extern "C" fn jet_ops_exp(base_ptr: &mut [u8; 32], exp_ptr: &[u8; 32]) -> i8 {
    // Little-endian representation
    let base = read_u256(base_ptr);
    let exp = read_u256(exp_ptr);
    let result = /* square-and-multiply */;
    write_u256(base_ptr, result);
    0  // Success return code
}
```

**LLVM IR declaration** (`crates/jet_runtime/src/runtime_builder.rs`):
```rust
self.module.add_function(
    "jet.ops.exp",
    self.types.i8.fn_type(&[self.types.ptr.into(), self.types.ptr.into()], false),
    None,
);
```

**Symbol registration** (`crates/jet/src/builder/env.rs`):
```rust
pub(crate) struct Symbols<'ctx> {
    exp: FunctionValue<'ctx>,
}
```

**JIT linking** (`crates/jet/src/engine/mod.rs`):
```rust
map_fn(sym.exp(), builtins::jet_ops_exp as *const () as usize);
```

### Opcode implementation checklist

For complete implementation guidance, see `docs/process/new-opcode.md`. Key steps:

1. **Read EVM spec** - `.agents/skills/evm-opcodes/references/docs/<HEX>.md`; understand operand order and edge cases
2. **Define instruction** - Add to `Instruction` enum in `instructions.rs` (most opcodes already exist)
3. **Implement handler** - Add function to `builder/ops.rs`
4. **Route opcode** - In `builder/contract.rs`: replace the `UnimplementedInstruction` arm in `build_non_jump_instruction`, and give the opcode its stack effect in `apply_abstract_instruction`
5. **Gas** - Add the static cost in `builder/gas.rs`; add a `DynamicGas` kind if the cost depends on operands
6. **Add builtin** (if complex) - Follow builtin function pattern above
7. **Write tests** - Add the opcode to `define_ops!` in `test_roms.rs`; cover basic case, edge cases, endianness, errors
8. **Test locally** - Run `make test` or `make test-cargo`
9. **Update status** - `OPERATION_STATUS.md` and `docs/test_coverage.md`
10. **Pre-push check** - Run `make commit-check` (fmt, clippy, tests)

## Troubleshooting

### LLVM not found
```bash
# Symptom
cargo check  # Error: No suitable version of LLVM was found

# Solution 1: Install LLVM
make install-llvm

# Solution 2: Verify detection
bash scripts/detect-llvm.sh  # Should output: /usr/lib/llvm-22

# Solution 3: Manual env var
export LLVM_SYS_221_PREFIX=/usr/lib/llvm-22
cargo check
```

### cargo-nextest not found
```bash
# Symptom
make test  # Error: no such command: `nextest`

# Solution: Install nextest
make install-tools
# Or: cargo install cargo-nextest --locked

# Workaround: Use cargo test instead
make test-cargo
```

### Tests segfault (SIGSEGV)

**CRITICAL:** Test segfaults are P0 priority issues that must be addressed immediately.

For complete troubleshooting guidance, see `docs/process/segfault-troubleshooting.md`.

**Quick reference:**
```bash
# Run with debug output
RUST_LOG=debug cargo test

# Run single test to isolate
cargo test --test test_roms -- test_name --exact

# Use jetdbg for interactive debugging
cargo run -p jetdbg
```

**Required action:**
- **You MUST either:** Submit a PR that fixes the segfault, OR create a new issue with detailed reproduction steps and analysis
- Do not leave segfaults unaddressed or undocumented

### Incremental compilation cache issues
```bash
# Symptom: Tests fail after fixing implementation

# Solution: Clean rebuild
rm -rf target  # Or on Termux: rm -rf /data/data/com.termux/files/home/.cargo/jet-target
cargo test
```

### Clippy warnings in CI
```bash
# Symptom: CI fails on clippy step with warnings

# Solution: Fix locally
make clippy         # See warnings
make clippy-fix     # Auto-fix some issues

# CRITICAL: Do NOT add #[allow(...)] pragmas without explicit permission
# You must fix the underlying issue or report why it should be allowed
```

### Auto-formatted code not matching expectations
```bash
# The CI auto-commits formatting changes
# If you see unexpected formatting:

# 1. Pull latest from remote (CI may have pushed formatting fixes)
git pull

# 2. Run fmt locally before committing
make fmt

# 3. Check fmt before pushing
make fmt-check
```

### Platform-specific issues
**Termux (Android):**
- Requires `gcc-default` and `ndk-multilib-native-static` packages
- Install via: `make install-llvm` (handles platform detection)
- Target directory: `/data/data/com.termux/files/home/.cargo/jet-target` (not `./target`)

**macOS:**
- LLVM installed via Homebrew or official binaries
- Detection via `scripts/detect-platform.sh` → `darwin` branch

**Unsupported platforms:**
- `scripts/detect-platform.sh` will error with clear message

## Known issues & limitations

### Current known bugs (as of 2026-09-18)
All previously known bugs have been resolved. Any new segfaults or bugs discovered must be reported immediately as P0 issues.

### EVM opcode implementation status
122 of 148 opcodes are implemented. See `OPERATION_STATUS.md` for the authoritative list.
- **Implemented:** Arithmetic, comparison, bitwise, KECCAK256, PUSH*/DUP*/SWAP*/POP, MLOAD/MSTORE/MSTORE8/MSIZE, JUMP/JUMPI/JUMPDEST/PC, GAS, call context (ADDRESS, ORIGIN, CALLER, CALLVALUE, CALLDATALOAD, CALLDATASIZE), block info (BLOCKHASH, COINBASE, TIMESTAMP, NUMBER, DIFFICULTY, GASLIMIT, CHAINID, BASEFEE, BLOBBASEFEE), RETURNDATASIZE/RETURNDATACOPY, CALL, RETURN, REVERT, INVALID
- **Not implemented:** Account state and code access (BALANCE, SELFBALANCE, EXTCODE*, CODESIZE, CODECOPY), storage (SLOAD, SSTORE, TLOAD, TSTORE), CALLDATACOPY, MCOPY, GASPRICE, BLOBHASH, LOG0-LOG4, CREATE/CREATE2, CALLCODE/DELEGATECALL/STATICCALL, SELFDESTRUCT

### Architecture limitations
- Gas: static costs are charged once per basic block and dynamic costs are computed in IR, but CALL does not forward gas (callee runs unbounded), and there is no 63/64 rule or refund handling
- CALL: JIT-to-JIT only; discards its gas and input operands, no value transfer or account state
- No storage, logs, or contract creation
- Two stack backends: the runtime stack in `Context` (default) and the symbolic SSA stack (`StackMode::SymbolicPreferred`), which falls back to the runtime backend when planning exceeds 64 entry states per block or 4096 total
- Dynamic jumps dispatch through an LLVM `switch` over all JUMPDEST blocks (shared jump block in runtime mode, site-local switches in symbolic mode)

## Additional resources

### External documentation
- [LLVM Documentation](https://llvm.org/docs/)
- [Inkwell (LLVM Rust bindings)](https://github.com/TheDan64/inkwell)
- [Ethereum Yellow Paper](https://ethereum.github.io/yellowpaper/paper.pdf)
- [EVM Opcodes](https://www.evm.codes/)
- [cargo-nextest](https://nexte.st/)

### Commit conventions
Follow conventional commits:
```
feat: implement OPCODE (0xHH)
fix: correct stack order in OPCODE
docs: update architecture documentation
test: add edge case for OPCODE
refactor: simplify LLVM IR generation
chore: update dependencies
```

### Getting help
- Review `docs/process/new-opcode.md` for detailed implementation guidance
- Check existing implementations in `builder/ops.rs` for patterns
- Reference builtin implementations in `crates/jet_runtime/src/builtins.rs`
- Use `evm-spec-lookup` skill if available for EVM opcode specifications

## Quick reference

### Essential commands
```bash
# One-time setup
make install-llvm          # Bootstrap LLVM 22 (required first)
make install-tools         # Install cargo-nextest (optional but recommended)

# Daily workflow
make fmt                   # Format code
make check                 # Fast validation
make clippy                # Lint
make test                  # Run tests (requires nextest)
make test-cargo            # Run tests (fallback, no nextest needed)
make commit-check          # Full pre-push validation

# Debugging
cargo run -p jetdbg        # Run debugger/executor
RUST_LOG=debug cargo test  # Verbose test output
```

### File locations
```
crates/jet/src/builder/ops.rs          → Opcode implementations
crates/jet/src/builder/contract.rs     → Opcode dispatch, symbolic planner, CFG
crates/jet/src/builder/stack.rs        → StackBackend trait and backends
crates/jet/src/builder/gas.rs          → Gas cost tables
crates/jet/src/instructions.rs         → Instruction enum
crates/jet_runtime/src/builtins.rs     → Complex operations (Rust)
crates/jet/tests/test_roms.rs          → Integration tests
OPERATION_STATUS.md                    → Implementation status
docs/test_coverage.md                  → Test coverage table
docs/process/new-opcode.md             → Implementation guide
docs/process/segfault-troubleshooting.md → Segfault debugging (P0)
.github/workflows/ci.yml               → CI pipeline
```

### Environment variables
```bash
export LLVM_SYS_221_PREFIX=/usr/lib/llvm-22  # Manual LLVM path
export RUST_BACKTRACE=1                       # Full backtraces (default in Makefile)
export RUST_LOG=debug                         # Verbose logging
export CARGO_TARGET_DIR=/custom/path          # Override target directory
```

---

**Last updated:** 2026-09-18  
**LLVM version:** 22  
**Rust edition:** 2024 (all crates)  
**Test runner:** cargo-nextest (recommended) or cargo test (fallback)
