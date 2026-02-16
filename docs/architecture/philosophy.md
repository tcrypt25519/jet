# Jet: Engineering Philosophy & Design Patterns

## 1. Context & Objective

Jet is a specialized compiler system designed to translate Ethereum Virtual Machine (EVM) bytecode into LLVM intermediate representation (LLVM IR), paired with a Just-In-Time (JIT) engine that lowers this IR to native machine code.

**The Mission:** Enable semantically identical Ethereum smart contracts to execute at native machine code speeds, primarily targeting high-throughput use cases like MEV simulation and large-scale state analysis.

---

## 2. Core Architectural Philosophies

These are the foundational beliefs that govern the Jet system. They represent inviolable principles that should guide all development decisions.

### 2.1 Semantic Preservation Above All

**The Principle:** Every transformation from EVM bytecode to native machine code must preserve the exact semantics of the original contract. A compiled contract must produce identical results to an interpreted execution for all possible inputs.

**Evidence in Codebase:**
- The `ReturnCode` enum carefully distinguishes between EVM-level successes, EVM-level failures, and Jet-level failures
- Error types explicitly differentiate between "unimplemented" (not yet built but planned) and "unexpected" (should never occur)
- Stack operations faithfully maintain EVM's 256-bit word semantics using `i256` types throughout

**Practical Implications:**
- Never optimize in ways that could change observable behavior
- When in doubt, preserve the slower but semantically correct implementation
- All edge cases in EVM specification must be handled identically
- While speed is the goal, correctness is the constraint. Match the yellow paper, then optimize

### 2.2 "EVM In a JIT" (Native Execution)

We do not interpret; we compile. Every EVM opcode is translated into a sequence of LLVM IR instructions or a call to a highly optimized runtime primitive. The goal is to strip away the interpretation overhead entirely.

### 2.3 Phase Isolation with Clean Boundaries

**The Principle:** The compilation pipeline is organized into distinct, isolated phases with well-defined interfaces between them.

**The Jet Pipeline:**

```
EVM Bytecode → Code Block Discovery → IR Generation → LLVM Module → JIT Compilation → Native Execution
```

**The system is strictly stratified into three distinct domains:**

1. **Builder (Compiler):** Pure transformation of EVM bytecode to LLVM IR. It knows nothing about the execution environment's specific memory address, only its structure.
   - `instructions.rs` - Pure bytecode parsing, no compilation logic
   - `builder/contract.rs` - IR generation phase, separate from execution
   - `builder/ops.rs` - Individual opcode implementations, isolated from control flow

2. **Engine (JIT):** Orchestrates the LLVM modules, optimization passes, and linking. It bridges the gap between the abstract IR and the physical CPU.
   - `engine/mod.rs` - JIT management, separate from IR construction

3. **Runtime (Execution):** A minimal, high-performance library (`jet_runtime`) providing the "syscalls" that the compiled code invokes (e.g., stack manipulation, memory access, keccak256).
   - `jet_runtime/` - Completely separate crate for runtime functions

**Practical Implications:**
- New opcodes should only touch `instructions.rs` (definition) and `builder/ops.rs` (implementation)
- Runtime modifications should not require builder changes
- Engine changes should not require understanding IR generation details

### 2.4 Explicit Control Flow Materialization

**The Principle:** EVM control flow must be materialized explicitly before code generation. Basic blocks are discovered before any IR is emitted.

**Evidence in Codebase:**
- `find_code_blocks()` performs a complete pass to identify all block boundaries
- `CodeBlocks` structure maintains the full control flow graph
- Jump tables are built as a dedicated final step
- `JUMPDEST` locations determine block boundaries definitively

**Practical Implications:**
- Never emit IR speculatively before understanding the full control flow
- Basic block discovery is a prerequisite, not an optimization
- Dynamic jump targets require explicit jump table construction

### 2.5 The Shared Library Pattern (Dual-World Architecture)

**The Principle:** Jet operates across two worlds—compile-time (Rust/LLVM) and runtime (JIT-executed code)—with explicit interfaces between them. Contracts are not isolated scripts; they are functions in a shared symbol table.

**Evidence in Codebase:**
- `jet_runtime` is compiled as both `dylib` and `lib` (dual compilation modes)
- Runtime functions declared in LLVM IR, implemented in Rust, linked at JIT time
- `symbols.rs` defines the exact contract between compile-time and runtime
- Global `JIT_ENGINE` pointer bridges the two worlds

**Symbol Naming:**
- Contracts: `jet.contracts.<HASH>`
- Runtime functions: `jet.<domain>.<action>[.<type>]`

**Inter-Contract Calls:** Direct function calls via the JIT engine, bypassing the overhead of a full message passing loop where possible.

**Practical Implications:**
- Runtime functions must have stable ABI (`extern "C"`)
- Changes to `symbols.rs` require coordinated updates to both `jet.ll` and Rust implementations
- Memory layout in `exec::Context` must match the LLVM IR struct exactly

### 2.6 Explicit & Linear Memory Model

The execution context is a single, linear block of memory containing:
- **Registers/Metadata:** Stack pointers, return data offsets
- **EVM Stack:** Fixed-size array (1024 words)
- **EVM Memory:** Linear byte array
- **Sub-call Pointers:** For recursion

This structure allows LLVM to perform aggressive alias analysis and optimization, as the layout is predictable and contiguous.

### 2.7 Fail-Fast Compilation

**The Principle:** Invalid or unsupported bytecode should be rejected early with clear diagnostics, not discovered at runtime.

**Evidence in Codebase:**
- `Error::UnimplementedInstruction` - explicit about what's missing
- `Error::UnexpectedInstruction` - distinct from unimplemented
- `Error::InvariantViolation` - for internal consistency failures
- Verification step with `module.verify()` after construction

**Practical Implications:**
- Return errors, don't panic (except for true invariant violations)
- Error messages should identify the specific opcode or construct
- Compile-time failures are preferable to runtime failures

---

## 5. Domain Lexicon

A guide to the naming conventions and domain language used throughout the codebase. To maintain cohesion, use these terms consistently.

### EVM Domain Terms

| Term | Meaning in Jet | Codebase Usage |
|------|---------------|----------------|
| **ROM / Bytecode** | Read-only memory containing bytecode | `rom: &[u8]` parameters |
| **Word** | 256-bit (32-byte) value | `type Word = [u8; 32]` |
| **PC** | Program counter / bytecode offset | `pc: usize` |
| **Opcode** | Single EVM instruction byte | `Instruction` enum |
| **Stack** | EVM execution stack (max 1024 words) | `stack: [Word; 1024]` |
| **Memory** | Byte-addressable contract memory | `memory: [u8; ...]` |
| **Storage** | Persistent key-value store | Not yet implemented |
| **JUMPDEST** | Valid jump target marker | Identifies block boundaries |
| **Return Data** | Data returned from sub-calls | `return_off`, `return_len` |

### Compiler Domain Terms

| Term | Meaning in Jet | Codebase Usage |
|------|---------------|----------------|
| **Builder** | IR construction subsystem | `builder/` module |
| **Env** | Compilation environment | `Env<'ctx>` struct |
| **BuildCtx** | Per-function build context | `BuildCtx<'ctx, 'b>` |
| **CodeBlock** | Basic block during discovery | `CodeBlock<'ctx, 'b>` |
| **Manager** | Contract compilation coordinator | `Manager<'ctx>` |
| **Types** | LLVM type definitions | `Types<'ctx>` |
| **Symbols** | Runtime function references | `Symbols<'ctx>` |

### Execution Domain Terms

| Term | Meaning in Jet | Codebase Usage |
|------|---------------|----------------|
| **Engine** | JIT execution environment | `Engine<'ctx>` |
| **Context (`exec_ctx`)** | Runtime execution state | `exec::Context` |
| **ContractRun** | Result of contract execution | `ContractRun` struct |
| **ReturnCode** | Execution result status | `ReturnCode` enum |
| **BlockInfo** | EVM block environment data | `BlockInfo` struct |
| **Builtins** | Runtime helper functions | `builtins.rs` |

### Naming Conventions

#### Functions

| Pattern | Usage | Example |
|---------|-------|---------|
| `build_*` | IR construction functions | `build_contract_body` |
| `find_*` | Discovery/analysis functions | `find_code_blocks` |
| `load_*` | LLVM load operations | `load_i256` |
| `__*` | Internal helpers (not public) | `__stack_pop_2` |
| `jet_*` | External API (FFI) | `jet_contract_call` |

#### Types you

| Pattern | Usage | Example |
|---------|-------|---------|
| `*Ctx` | Context/state structures | `BuildCtx`, `Context` |
| `*Info` | Read-only data containers | `BlockInfo` |
| `*Run` | Execution results | `ContractRun` |
| `*Value` | LLVM value wrappers | `IntValue`, `PointerValue` |

#### LLVM IR Names

| Pattern | Usage | Example |
|---------|-------|---------|
| `*_result` | Operation outputs | `"add_result"` |
| `*_ptr` | Pointer values | `"word_ptr"` |
| `*_value` | Loaded values | `"jump_value"` |
| `block` | Basic blocks | `"jump_block"` |

### Symbol Naming in IR

Runtime functions follow the pattern: `jet.{domain}.{action}[.{type}]`

```
jet.stack.push.i256     - Push 256-bit integer to stack
jet.stack.push.ptr      - Push pointer to stack
jet.stack.pop           - Pop from stack
jet.mem.store.word      - Store 32-byte word to memory
jet.contract.call       - Call another contract
jet.ops.keccak256       - Compute keccak256 hash
```

---

## Appendix A: Architecture Diagrams

### Compilation Pipeline

```
┌─────────────────────────────────────────────────────────────────────┐
│                          JET COMPILER                               │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────────────┐│
│  │  EVM         │     │   Code       │     │      LLVM IR         ││
│  │  Bytecode    │────▶│   Block      │────▶│      Module          ││
│  │  (rom: &[u8])│     │   Discovery  │     │                      ││
│  └──────────────┘     └──────────────┘     └──────────────────────┘│
│                                                     │               │
│         instructions.rs    contract.rs          ops.rs              │
│                                                     │               │
│                                                     ▼               │
│                                            ┌──────────────────────┐│
│                                            │    JIT Engine        ││
│                                            │    (LLVM ORC)        ││
│                                            └──────────────────────┘│
│                                                     │               │
│                                                     ▼               │
│                                            ┌──────────────────────┐│
│                                            │  Native Machine      ││
│                                            │  Code                ││
│                                            └──────────────────────┘│
└─────────────────────────────────────────────────────────────────────┘
```

### Runtime Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        JIT EXECUTION                                │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────────┐        ┌────────────────────────────────┐│
│  │   Compiled Contract  │        │        Runtime Context         ││
│  │   (Native Code)      │◀──────▶│        (exec::Context)         ││
│  └──────────────────────┘        └────────────────────────────────┘│
│            │                                    │                   │
│            │    ┌───────────────────┐           │                   │
│            └───▶│  Runtime Builtins │◀──────────┘                   │
│                 │  (Rust FFI)       │                               │
│                 └───────────────────┘                               │
│                          │                                          │
│                          ▼                                          │
│                 ┌───────────────────┐                               │
│                 │  Symbol Table     │                               │
│                 │  (jet.* symbols)  │                               │
│                 └───────────────────┘                               │
└─────────────────────────────────────────────────────────────────────┘
```

### Module Dependencies

```
                    ┌───────────────┐
                    │   jet (lib)   │
                    └───────────────┘
                           │
          ┌────────────────┼────────────────┐
          ▼                ▼                ▼
   ┌────────────┐   ┌────────────┐   ┌────────────┐
   │  builder   │   │   engine   │   │instructions│
   └────────────┘   └────────────┘   └────────────┘
          │                │
          │                ▼
          │        ┌─────────────────┐
          └───────▶│  jet_runtime    │
                   └─────────────────┘
                          │
            ┌─────────────┼─────────────┐
            ▼             ▼             ▼
      ┌─────────┐   ┌─────────┐   ┌─────────┐
      │builtins │   │  exec   │   │ symbols │
      └─────────┘   └─────────┘   └─────────┘
```

---

## Appendix B: Quick Reference

### File Locations

| Purpose | Location |
|---------|----------|
| EVM opcodes | `crates/jet/src/instructions.rs` |
| IR generation | `crates/jet/src/builder/` |
| Opcode implementations | `crates/jet/src/builder/ops.rs` |
| JIT engine | `crates/jet/src/engine/mod.rs` |
| Runtime context | `crates/jet_runtime/src/exec.rs` |
| FFI builtins | `crates/jet_runtime/src/builtins.rs` |
| Symbol names | `crates/jet_runtime/src/symbols.rs` |
| LLVM IR runtime | `runtime-ir/jet.ll` |
| Tests | `crates/jet/tests/` |

### Common Operations

| Task | Location | Function |
|------|----------|----------|
| Add opcode | `instructions.rs` | `instructions!` macro |
| Implement opcode | `ops.rs` | `pub(crate) fn name()` |
| Pop from stack | `ops.rs` | `__stack_pop_N()` |
| Push to stack | `ops.rs` | `__stack_push_int/ptr()` |
| Load value | `ops.rs` | `load_iN()` |
| Build IR | `contract.rs` | `BuildCtx.builder.*` |
| Add runtime fn | `builtins.rs` | `extern "C" fn` |

### Return Codes

| Code | Meaning |
|------|---------|
| -1 | `InvalidJumpBlock` - Jet error |
| 0 | `ImplicitReturn` - Normal completion |
| 1 | `ExplicitReturn` - RETURN opcode |
| 2 | `Stop` - STOP opcode |
| 64 | `Revert` - REVERT opcode |
| 65 | `Invalid` - INVALID opcode |
| 66 | `JumpFailure` - Invalid jump target |

---

*This manifesto is a living document. Update it as the codebase evolves.*
