```
.
  .cargo/config.toml
  .gitignore
  .nextest.toml
  Cargo.lock
  Cargo.toml
  DEVELOPMENT.md
  LICENSE
  Makefile
  README.md
  rust-toolchain.toml
- crates/
  - jet/
      Cargo.toml
    - src/
        instructions.rs
        lib.rs
      - bin/
          jetdbg.rs
      - builder/
          contract.rs
          env.rs
          manager.rs
          mod.rs
          ops.rs
      - engine/
          mod.rs
    - tests/
        invalid_opcode.rs
        test_roms.rs
      - roms/
          mod.rs
  - jet_ir/
      Cargo.toml
    - src/
        constants.rs
        lib.rs
        types.rs
  - jet_push_macros/
      Cargo.toml
    - src/
        lib.rs
    - tests/
        macro_tests.rs
      - compile_fail/
          invalid_range.rs
          not_integer.rs
          not_range.rs
          open_range.rs
  - jet_runtime/
      Cargo.toml
    - src/
        builtins.rs
        error.rs
        exec.rs
        layout_tests.rs
        lib.rs
        runtime_builder.rs
        symbols.rs
      - binding/
          mod.rs
- contrib/
  - LLVM.tmBundle/        # TextMate syntax bundle for LLVM IR
- docs/
    architecture.md
    architecture-notes.md
    architecture-refactoring-summary.md
    bytecode-to-llvm-blocks.md
    jet-description.md
    libpolly.md
    manifesto.md
    test_coverage.md
    TREE.md
  - adrs/
      adr-001.md
      adr-002.md
  - ext/
    - evm/                # External EVM reference docs (.mdx)
    - rust/
        type-system-patterns.md
  - process/
      new-opcode.md
  - tasks/
      memory-length-tracking-mstore.md
- examples/               # LLVM IR snapshots for before/after comparisons
    master.ll
    master_opt.ll
    new.ll
    new_opt.ll
    new_runtime_opt.ll
- scripts/
    detect-llvm.sh
    detect-platform.sh
    install-llvm.sh
    llvm.sh
    opcode_doc
```
