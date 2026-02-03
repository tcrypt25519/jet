# Jet – Copilot Onboarding Instructions

## What this repo does
- Rust workspace that JIT-compiles Ethereum Virtual Machine (EVM) bytecode to LLVM IR and then native code via LLVM ORC JIT (`inkwell` bindings).
- Core crate `crates/jet`: parses EVM opcodes, builds LLVM IR, and drives the JIT (`builder`, `engine`, `instructions`).
- Runtime crate `crates/jet_runtime`: execution context and builtins invoked from generated IR.
- `runtime-ir/jet.ll`: shared LLVM IR declarations for runtime symbols and a few IR-implemented helpers.

## Repository/layout quick map
- Root: `Cargo.toml` (workspace), `Makefile`, `.cargo/config.toml` (Android/Termux linker flags), `README.md`, `runtime-ir/jet.ll`, `scripts/` (LLVM detection/install).
- Workspace members: `crates/jet`, `crates/jet_runtime`.
- Key source:
  - `crates/jet/src/lib.rs` (exports), `builder/{contract,env,manager,ops}.rs`, `engine/mod.rs`, `instructions.rs`, `bin/jetdbg.rs`, `tests/test_roms.rs` (+ `roms/` fixtures).
  - `crates/jet_runtime/src/{lib,exec,builtins,symbols}.rs`, `binding/mod.rs`.
- Docs: `docs/architecture*.md`, `docs/jet-description.md`, `docs/TREE.md`, `docs/manifesto.md`, `docs/plans/2026-01-27-restore-build-system.md`, `docs/build-restoration/*`.
- CI: GitHub Actions workflows present for Copilot code review and Copilot coding agent (`dynamic/copilot-*/copilot`). No other workflows found.

## Toolchain & dependencies
- Rust: no pinned `rust-toolchain`; workspace uses edition 2024 (`crates/jet`) and 2021 (`crates/jet_runtime`). `cargo` locked crates assuming Rust ≥1.93 (seen during `cargo check`).
- LLVM: target version 21. `inkwell` built from Git with `llvm21-1` feature; `llvm-sys` expects `llvm-config-21` or `LLVM_SYS_211_PREFIX` pointing to LLVM 21 prefix.
- Environment: `Makefile` exports `LLVM_SYS_211_PREFIX` via `scripts/detect-llvm.sh` and `RUST_BACKTRACE=1`. Termux linker flags in `.cargo/config.toml`.

## Build, test, lint, run (validated sequences)
- **Bootstrap LLVM (required before build/test):**
  - `make install-llvm`
    - Uses `scripts/install-llvm.sh`; on Debian installs via apt (may require sudo). Sets `LLVM_PREFIX` discovered by `scripts/detect-llvm.sh`.
    - Needed to satisfy `llvm-sys` / `inkwell`. Skipping this caused failure: `No suitable version of LLVM was found system-wide or pointed to by LLVM_SYS_211_PREFIX` during `cargo check`.
- **Build:** `make build` (runs `cargo build`). Requires LLVM 21 present. Fails if `llvm-config-21` missing or `LLVM_SYS_211_PREFIX` unset.
- **Check:** `make check` (`cargo check`).
- **Tests:** `make test` (`cargo test`). Pure Rust integration tests under `crates/jet/tests/test_roms.rs`; no external EVM node required, but still needs LLVM toolchain for compilation.
- **Lint:** `make clippy` (runs `cargo clippy --all-targets --all-features -- -D warnings`).
- **Fmt:** `cargo fmt` (validated OK).
- **Run jetdbg:** `cargo run --bin jetdbg` (after LLVM present).
- **Pre-push recommendation:** `make commit-check` (aliases check, build, test, clippy).
- If builds still fail, ensure `llvm-config-21` is on PATH or set `export LLVM_SYS_211_PREFIX=$(bash scripts/detect-llvm.sh)` before cargo commands.

### Observed command results (2026-02-03)
- `cargo check` (without LLVM installed) → failed with `llvm-sys` error about missing LLVM 21.
- `cargo fmt` → succeeded.

## Additional notes & tips
- `Makefile` sets `LLVM_VERSION := 21`; all scripts assume that.
- `scripts/detect-platform.sh` branches for `darwin`, `termux`, `debian`. Unsupported platforms will error early.
- No rust-toolchain file: respect the repo’s default stable toolchain; avoid pinning unless required.
- Tests and binaries rely on `runtime-ir/jet.ll` being present; do not remove or rename.
- If targeting Android/Termux, `.cargo/config.toml` already provides linker flags; avoid overwriting.
- Trust these instructions first; only search if something appears incorrect or missing.
