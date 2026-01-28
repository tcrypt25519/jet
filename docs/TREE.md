```
.
  .gitignore
  Cargo.toml
  Makefile
  README.md
- crates/
  - jet/
      Cargo.toml
    - runtime-ir/
        jet.ll
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
        test_roms.rs
      - roms/
          mod.rs
  - jet_runtime/
      Cargo.toml
    - src/
        builtins.rs
        exec.rs
        lib.rs
        symbols.rs
      - binding/
          mod.rs
- runtime-ir/
    jet.ll
- scratch/
    new_runtime_opt.ll
    todo.md
  - crates-bk/
    - runtime/
        cargo.toml
      - src/
          exec.rs
          lib.rs
  - examples/
      master.ll
      master_opt.ll
      new.ll
      new_opt.ll
      temp.ll
```
